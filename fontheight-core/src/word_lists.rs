use std::{
    fs, io,
    ops::{Deref, Index},
    path::{Path, PathBuf},
    slice,
};

use rustybuzz::{script, Direction, Script};
use thiserror::Error;

#[derive(Debug)]
pub struct WordList {
    name: String,
    words: Vec<String>,
}

impl WordList {
    pub fn load(
        name: impl Into<String>,
        path: impl AsRef<Path>,
    ) -> Result<Self, WordListError> {
        let path = path.as_ref();
        let file_content = fs::read_to_string(path).map_err(|io_err| {
            WordListError::FailedToRead(path.to_owned(), io_err)
        })?;
        Ok(WordList {
            name: name.into(),
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
            name: name.into(),
            words: words.into_iter().map(Into::into).collect(),
        }
    }

    // Private API used by static-lang-word-lists
    #[doc(hidden)]
    pub fn new(name: String, words: Vec<String>) -> Self {
        WordList { name, words }
    }

    pub fn name(&self) -> &str {
        &self.name
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

    pub fn properties(&self) -> Option<(Direction, Script)> {
        for word in self.words.iter().take(20) {
            let mut buffer = rustybuzz::UnicodeBuffer::new();
            buffer.push_str(word);
            buffer.guess_segment_properties();
            if buffer.direction() != Direction::Invalid
                && buffer.script() != script::UNKNOWN
            {
                return Some((buffer.direction(), buffer.script()));
            }
        }
        None
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
