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
            words: newline_delimited_words(file_content),
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

fn newline_delimited_words(input: impl AsRef<str>) -> Vec<String> {
    input
        .as_ref()
        .split_whitespace()
        .filter(|word| !word.is_empty())
        .map(String::from)
        .collect()
}

#[allow(dead_code)] // FIXME: remove once we've worked out how we'll access these
pub mod builtins {
    macro_rules! builtin_wordlist {
        (name: $name:ident, path: $path:literal $(,)?) => {
            ::paste::paste! {
                pub static [<$name:snake:upper>]: ::std::sync::LazyLock<super::WordList> =
                    ::std::sync::LazyLock::new(|| {
                        ::include_flate::flate!(
                            static RAW_STR: str from $path
                        );
                        ::log::debug!("loaded {}", ::std::stringify!($name));
                        super::WordList {
                            name: ::std::string::String::from(
                                ::std::stringify!($name),
                            ),
                            words: super::newline_delimited_words(
                                ::std::ops::Deref::deref(&RAW_STR),
                            ),
                        }
                    });
            }
        };
    }

    builtin_wordlist! {
        name: diffenator2_latin,
        path: "data/diffenator_word_lists/Latin.txt",
    }
    builtin_wordlist! {
        name: diffenator2_greek,
        path: "data/diffenator_word_lists/Greek.txt",
    }
    builtin_wordlist! {
        name: diffenator2_cyrillic,
        path: "data/diffenator_word_lists/Cyrillic.txt",
    }
}
