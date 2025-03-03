use std::{
    fs, io,
    ops::{Deref, Index},
    path::{Path, PathBuf},
    slice,
};

use thiserror::Error;

use crate::{Script, ScriptExtension};

#[derive(Debug)]
pub struct WordList {
    name: String,
    words: Vec<String>,
    scripts: ScriptExtension,
}

impl WordList {
    pub fn load(
        name: impl Into<String>,
        path: impl AsRef<Path>,
        scripts: ScriptExtension,
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
            scripts,
        })
    }

    pub fn define(
        name: impl Into<String>,
        words: impl IntoIterator<Item = impl Into<String>>,
        scripts: ScriptExtension,
    ) -> Self {
        WordList {
            name: name.into(),
            words: words.into_iter().map(Into::into).collect(),
            scripts,
        }
    }

    // Private API used by static-lang-word-lists
    #[doc(hidden)]
    #[inline]
    pub fn new(
        name: String,
        words: Vec<String>,
        scripts: ScriptExtension,
    ) -> Self {
        WordList {
            name,
            words,
            scripts,
        }
    }

    #[inline]
    pub fn name(&self) -> &str {
        &self.name
    }

    #[inline]
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

    #[inline]
    pub fn covers(&self, script: Script) -> bool {
        self.scripts.contains_script(script)
    }

    #[inline]
    pub fn covers_all(&self, scripts: ScriptExtension) -> bool {
        self.scripts.intersection(scripts) == self.scripts
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

#[derive(Debug, Error)]
pub enum WordListError {
    #[error("failed to read from {}: {}", .0.display(), .1)]
    FailedToRead(PathBuf, io::Error),
}
