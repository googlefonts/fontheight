use std::{
    fs, io,
    ops::{Deref, Index},
    path::{Path, PathBuf},
    slice,
};

use thiserror::Error;

use crate::metadata::WordListMetadata;

impl WordListMetadata {
    pub(crate) fn load(
        metadata_path: impl Into<std::path::PathBuf>,
    ) -> Result<Self, WordListError> {
        let path = metadata_path.into();
        let metadata_content = fs::read_to_string(&path).map_err(|io_err| {
            WordListError::FailedToRead(path.to_owned(), io_err)
        })?;
        let metadata: WordListMetadata =
            serde_json::from_str(&metadata_content).map_err(|json_err| {
                WordListError::MetadataError(path.to_owned(), json_err)
            })?;
        Ok(metadata)
    }

    pub(crate) fn new_from_name(name: impl Into<String>) -> Self {
        WordListMetadata {
            name: name.into(),
            script: None,
            language: None,
        }
    }
}

#[derive(Debug)]
pub struct WordList {
    words: Vec<String>,
    metadata: WordListMetadata,
}

impl WordList {
    /// Load a word list from a file.
    ///
    /// The file is expected to contain one word per line.
    /// The word list may also be accompanied by a metadata file in JSON format.
    /// See [`WordListMetadata`] for the expected format.
    pub fn load(
        path: impl AsRef<Path>,
        metadata_path: Option<impl AsRef<Path>>,
    ) -> Result<Self, WordListError> {
        let path = path.as_ref();
        let file_content = fs::read_to_string(path).map_err(|io_err| {
            WordListError::FailedToRead(path.to_owned(), io_err)
        })?;
        let metadata = if let Some(metadata_path) = metadata_path {
            let metadata_path = metadata_path.as_ref();
            WordListMetadata::load(metadata_path)?
        } else {
            // Fake metadata as much as we can
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
                .to_string()
                .replace("/", "_");
            WordListMetadata::new_from_name(name)
        };
        Ok(WordList {
            metadata,
            words: file_content
                .split_whitespace()
                .filter(|word| !word.is_empty())
                .map(String::from)
                .collect(),
        })
    }

    pub fn define(
        name: impl Into<String>,
        words: impl IntoIterator<Item = impl Into<String>>,
    ) -> Self {
        WordList {
            metadata: WordListMetadata::new_from_name(name.into()),
            words: words.into_iter().map(Into::into).collect(),
        }
    }

    // Private API used by static-lang-word-lists
    #[doc(hidden)]
    pub fn new(metadata: WordListMetadata, words: Vec<String>) -> Self {
        WordList { metadata, words }
    }

    #[inline]
    pub fn name(&self) -> &str {
        &self.metadata.name
    }

    #[inline]
    pub fn script(&self) -> Option<&str> {
        self.metadata.script.as_deref()
    }

    #[inline]
    pub fn language(&self) -> Option<&str> {
        self.metadata.language.as_deref()
    }

    pub fn iter(&self) -> WordListIter {
        WordListIter(self.words.iter())
    }

    #[inline]
    pub fn len(&self) -> usize {
        self.words.len()
    }

    #[inline]
    pub fn is_empty(&self) -> bool {
        self.words.is_empty()
    }

    #[inline]
    pub fn get(&self, index: usize) -> Option<&str> {
        self.words.get(index).map(|word| word.as_str())
    }
}

impl Index<usize> for WordList {
    type Output = str;

    fn index(&self, index: usize) -> &Self::Output {
        self.words.index(index).deref()
    }
}

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

#[derive(Debug, Error)]
pub enum WordListError {
    #[error("failed to read from {}: {}", .0.display(), .1)]
    FailedToRead(PathBuf, io::Error),
    #[error("failed to parse metadata from {}: {}", .0.display(), .1)]
    MetadataError(PathBuf, serde_json::Error),
}

#[cfg(feature = "rayon")]
pub(crate) mod rayon {
    use rayon::iter::{
        plumbing::{
            bridge, Consumer, Producer, ProducerCallback, UnindexedConsumer,
        },
        IndexedParallelIterator, ParallelIterator,
    };

    use super::{WordList, WordListIter};

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
        pub fn par_iter(&self) -> ParWordListIter {
            ParWordListIter(&self.words)
        }
    }
}
