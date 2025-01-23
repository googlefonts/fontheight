use std::{fs, path::Path};

use anyhow::Context;

#[derive(Debug)]
pub struct WordList {
    #[allow(dead_code)] // FIXME: remove when no longer needed
    name: String,
    words: Vec<String>,
}

impl WordList {
    #[allow(dead_code)] // FIXME: remove when no longer needed
    pub fn load(
        name: impl Into<String>,
        path: impl AsRef<Path>,
    ) -> anyhow::Result<Self> {
        let path = path.as_ref();
        let file_content = fs::read_to_string(path)
            .with_context(|| format!("unable to read {}", path.display()))?;
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

    pub fn iter(&self) -> impl Iterator<Item = &str> {
        self.words.iter().map(String::as_ref)
    }
}
