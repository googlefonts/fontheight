use std::{
    fs, io,
    path::{Path, PathBuf},
};

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

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn iter(&self) -> impl Iterator<Item = &str> {
        self.words.iter().map(String::as_ref)
    }
}

#[derive(Debug, Error)]
pub enum WordListError {
    #[error("failed to read from {}: {}", .0.display(), .1)]
    FailedToRead(PathBuf, io::Error),
}
