use std::{
    borrow::Cow,
    fs, io,
    ops::{Deref, Index},
    path::{Path, PathBuf},
    slice,
    sync::LazyLock,
};

use serde::Deserialize;
use thiserror::Error;

use crate::newline_delimited_words;

// TODO: this can be Box<str>
pub(crate) type Word = String;
pub(crate) type WordSource = Box<[Word]>;

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct WordListMetadata {
    name: Cow<'static, str>,
    script: Option<Cow<'static, str>>,
    language: Option<Cow<'static, str>>,
}

impl WordListMetadata {
    // Used by word_list!
    #[must_use]
    pub(crate) const fn new(
        name: &'static str,
        script: Option<&'static str>,
        language: Option<&'static str>,
    ) -> Self {
        // Can't use Option::map in const context
        let script = match script {
            Some(script) => Some(Cow::Borrowed(script)),
            None => None,
        };
        let language = match language {
            Some(language) => Some(Cow::Borrowed(language)),
            None => None,
        };
        WordListMetadata {
            name: Cow::Borrowed(name),
            script,
            language,
        }
    }

    #[allow(clippy::result_large_err)]
    fn load(metadata_path: impl AsRef<Path>) -> Result<Self, WordListError> {
        let path = metadata_path.as_ref();
        let metadata_content = fs::read_to_string(path).map_err(|io_err| {
            WordListError::FailedToRead(path.to_owned(), io_err)
        })?;
        let metadata: WordListMetadata = toml::from_str(&metadata_content)
            .map_err(|json_err| {
                WordListError::MetadataError(path.to_owned(), json_err)
            })?;
        Ok(metadata)
    }

    fn new_from_name(name: impl Into<String>) -> Self {
        WordListMetadata {
            name: Cow::Owned(name.into()),
            script: None,
            language: None,
        }
    }
}

/// A list of words, with optional additional metadata.
#[derive(Debug)]
pub struct WordList {
    words: EagerOrLazy<WordSource>,
    metadata: WordListMetadata,
}

impl WordList {
    /// Load a word list from a file.
    ///
    /// The file is expected to contain one word per line.
    /// The word list may also be accompanied by a metadata TOML file.
    // TODO: some kind of schema or similar
    /// A fully specified metadata file may look like this:
    /// ```toml
    #[doc = include_str!("../data/diffenator/Latin.toml")]
    /// ```
    #[allow(clippy::result_large_err)]
    pub fn load(
        path: impl AsRef<Path>,
        metadata_path: impl AsRef<Path>,
    ) -> Result<Self, WordListError> {
        let mut word_list = WordList::load_without_metadata(path)?;
        word_list.metadata = WordListMetadata::load(metadata_path)?;
        Ok(word_list)
    }

    /// Load a word list from a file.
    ///
    /// The file is expected to contain one word per line.
    /// Always prefer [`WordList::load`] if metadata is available.
    #[allow(clippy::result_large_err)]
    pub fn load_without_metadata(
        path: impl AsRef<Path>,
    ) -> Result<Self, WordListError> {
        let path = path.as_ref();
        let file_content = fs::read_to_string(path).map_err(|io_err| {
            WordListError::FailedToRead(path.to_owned(), io_err)
        })?;
        let name = path
            .file_stem()
            .ok_or_else(|| {
                WordListError::FailedToRead(
                    path.to_owned(),
                    io::Error::new(
                        io::ErrorKind::InvalidData,
                        "file name is empty",
                    ),
                )
            })?
            .to_string_lossy()
            .replace("/", "_");

        Ok(WordList {
            metadata: WordListMetadata::new_from_name(name),
            words: newline_delimited_words(file_content).into(),
        })
    }

    /// Create a new word list from an iterable.
    ///
    /// Metadata is unspecified.
    #[must_use]
    pub fn define(
        name: impl Into<String>,
        words: impl IntoIterator<Item = impl Into<String>>,
    ) -> Self {
        WordList {
            metadata: WordListMetadata::new_from_name(name.into()),
            words: words.into_iter().map(Into::into).collect::<Vec<_>>().into(),
        }
    }

    // Used by wordlist! {}
    #[must_use]
    pub(crate) const fn new_lazy(
        metadata: WordListMetadata,
        words: LazyLock<WordSource>,
    ) -> Self {
        WordList {
            words: EagerOrLazy::Lazy(words),
            metadata,
        }
    }

    // Used by wordlist! when building on docs.rs
    #[allow(dead_code)]
    pub(crate) const fn stub() -> Self {
        WordList {
            metadata: WordListMetadata {
                name: Cow::Borrowed("stub"),
                script: None,
                language: None,
            },
            words: EagerOrLazy::Lazy(LazyLock::new(|| unreachable!())),
        }
    }

    /// Get the name of the word list.
    #[inline]
    #[must_use]
    pub fn name(&self) -> &str {
        &self.metadata.name
    }

    /// Get the script of the word list, if known.
    ///
    /// The script is expected to be a [ISO 15924](https://en.wikipedia.org/wiki/ISO_15924)
    /// four-letter capitalised code, but this is only guaranteed for built-in
    /// word lists.
    #[inline]
    #[must_use]
    pub fn script(&self) -> Option<&str> {
        self.metadata.script.as_deref()
    }

    /// Get the language of the word list, if known.
    ///
    /// The language is expected to be a [ISO 639-1](https://en.wikipedia.org/wiki/ISO_639-1)
    /// two-letter code, but this is only guaranteed for built-in word lists.
    #[inline]
    #[must_use]
    pub fn language(&self) -> Option<&str> {
        self.metadata.language.as_deref()
    }

    /// Iterate through the word list.
    #[must_use]
    pub fn iter(&self) -> WordListIter<'_> {
        WordListIter(self.words.iter())
    }

    /// Get how many words there are in the word list.
    #[inline]
    #[must_use]
    pub fn len(&self) -> usize {
        self.words.len()
    }

    /// Returns `true` if there are no words in the word list.
    #[inline]
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.words.is_empty()
    }
}

impl Index<usize> for WordList {
    type Output = str;

    fn index(&self, index: usize) -> &Self::Output {
        self.words.index(index).deref()
    }
}

#[derive(Debug)]
enum EagerOrLazy<T> {
    Eager(T),
    Lazy(LazyLock<T>),
}

impl<T> From<T> for EagerOrLazy<T> {
    fn from(value: T) -> Self {
        EagerOrLazy::Eager(value)
    }
}

impl<T> Deref for EagerOrLazy<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        match self {
            EagerOrLazy::Eager(e) => e,
            EagerOrLazy::Lazy(l) => l,
        }
    }
}

impl From<Vec<String>> for EagerOrLazy<WordSource> {
    fn from(value: Vec<String>) -> Self {
        Self::Eager(value.into_boxed_slice())
    }
}

/// An iterator over a [`WordList`].
///
/// Returned by [`WordList::iter`].
#[derive(Debug)]
pub struct WordListIter<'a>(slice::Iter<'a, String>);

impl<'a> Iterator for WordListIter<'a> {
    type Item = &'a str;

    fn next(&mut self) -> Option<Self::Item> {
        self.0.next().map(String::as_ref)
    }
}

impl ExactSizeIterator for WordListIter<'_> {
    fn len(&self) -> usize {
        self.0.len()
    }
}

impl DoubleEndedIterator for WordListIter<'_> {
    fn next_back(&mut self) -> Option<Self::Item> {
        self.0.next_back().map(String::as_ref)
    }
}

/// An error encountered while loading a [`WordList`] and its metadata.
#[derive(Debug, Error)]
pub enum WordListError {
    /// Unable to read either the word list or the metadata file.
    #[error("failed to read from {}: {}", .0.display(), .1)]
    FailedToRead(PathBuf, io::Error),
    /// Unable to parse the metadata.
    #[error("failed to parse metadata from {}: {}", .0.display(), .1)]
    MetadataError(PathBuf, toml::de::Error),
}

#[cfg(feature = "rayon")]
pub(crate) mod rayon {
    use rayon::iter::{
        IndexedParallelIterator, ParallelIterator,
        plumbing::{
            Consumer, Producer, ProducerCallback, UnindexedConsumer, bridge,
        },
    };

    use super::{WordList, WordListIter};

    /// A [`rayon`]-powered parallel iterator over a [`WordList`].
    ///
    /// Returned by [`WordList::par_iter`].
    #[derive(Debug)]
    pub struct ParWordListIter<'a>(&'a [String]);

    impl<'a> ParallelIterator for ParWordListIter<'a> {
        type Item = &'a str;

        fn drive_unindexed<C>(self, consumer: C) -> C::Result
        where
            C: UnindexedConsumer<Self::Item>,
        {
            bridge(self, consumer)
        }

        fn opt_len(&self) -> Option<usize> {
            Some(self.0.len())
        }
    }

    impl<'a> Producer for ParWordListIter<'a> {
        type IntoIter = WordListIter<'a>;
        type Item = &'a str;

        fn into_iter(self) -> Self::IntoIter {
            WordListIter(self.0.iter())
        }

        fn split_at(self, index: usize) -> (Self, Self) {
            let (left, right) = self.0.split_at(index);
            (ParWordListIter(left), ParWordListIter(right))
        }
    }

    impl IndexedParallelIterator for ParWordListIter<'_> {
        fn len(&self) -> usize {
            self.0.len()
        }

        fn drive<C: Consumer<Self::Item>>(self, consumer: C) -> C::Result {
            bridge(self, consumer)
        }

        fn with_producer<CB>(self, callback: CB) -> CB::Output
        where
            CB: ProducerCallback<Self::Item>,
        {
            callback.callback(self)
        }
    }

    impl<'a> rayon::iter::IntoParallelIterator for &'a WordList {
        type Item = &'a str;
        type Iter = ParWordListIter<'a>;

        fn into_par_iter(self) -> Self::Iter {
            ParWordListIter(&self.words)
        }
    }

    impl WordList {
        /// Iterate through the word list in parallel with `rayon`.
        #[must_use]
        pub fn par_iter(&self) -> ParWordListIter<'_> {
            ParWordListIter(&self.words)
        }
    }
}
