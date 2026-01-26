use std::{borrow::Cow, fs, path::Path};

use serde::Deserialize;

use crate::WordListError;

/// Metadata about a [`WordList`](crate::WordList).
///
/// If you don't want to mess around with the 🐄s, convenience methods are
/// provided for reading fields:
/// - [`WordListMetadata::name`]
/// - [`WordListMetadata::script`]
/// - [`WordListMetadata::language`]
#[derive(Debug, Clone, Eq, PartialEq, Hash, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WordListMetadata {
    /// The cosmetic name for the word list
    pub name: Cow<'static, str>,
    /// The script of the word list, if known.
    ///
    /// The script is expected to be an [ISO 15924](https://en.wikipedia.org/wiki/ISO_15924)
    /// four-letter capitalised code.
    pub script: Option<Cow<'static, str>>,
    /// The language of the word list, if known.
    ///
    /// The language is expected to be an [ISO 639-1](https://en.wikipedia.org/wiki/ISO_639-1)
    /// two-letter code.
    pub language: Option<Cow<'static, str>>,
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

    /// Load metadata from an on-disk TOML file
    #[allow(clippy::result_large_err)]
    pub fn load(
        metadata_path: impl AsRef<Path>,
    ) -> Result<Self, WordListError> {
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

    pub(crate) fn new_from_name(name: impl Into<String>) -> Self {
        WordListMetadata {
            name: Cow::Owned(name.into()),
            script: None,
            language: None,
        }
    }

    /// Get the name of the word list.
    #[inline]
    #[must_use]
    pub fn name(&self) -> &str {
        self.name.as_ref()
    }

    /// Get the script of the word list, if known.
    ///
    /// The script is expected to be an [ISO 15924](https://en.wikipedia.org/wiki/ISO_15924)
    /// four-letter capitalised code, but this is only guaranteed for built-in
    /// word lists.
    #[inline]
    #[must_use]
    pub fn script(&self) -> Option<&str> {
        self.script.as_deref()
    }

    /// Get the language of the word list, if known.
    ///
    /// The language is expected to be an [ISO 639-1](https://en.wikipedia.org/wiki/ISO_639-1)
    /// two-letter code, but this is only guaranteed for built-in word lists.
    #[inline]
    #[must_use]
    pub fn language(&self) -> Option<&str> {
        self.language.as_deref()
    }
}

impl<S> From<S> for WordListMetadata
where
    S: Into<Cow<'static, str>>,
{
    fn from(word_list_name: S) -> Self {
        WordListMetadata {
            name: word_list_name.into(),
            script: None,
            language: None,
        }
    }
}
