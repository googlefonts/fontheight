use std::{borrow::Cow, fs, path::Path};

use serde::Deserialize;

use crate::WordListError;

#[derive(Debug, Clone, Eq, PartialEq, Hash, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WordListMetadata {
    pub(crate) name: Cow<'static, str>,
    pub(crate) script: Option<Cow<'static, str>>,
    pub(crate) language: Option<Cow<'static, str>>,
}

impl WordListMetadata {
    // Used by word_list!
    // Library users should use the Builder struct
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
    pub(crate) fn load(
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

    #[must_use]
    pub fn name(&self) -> &str {
        self.name.as_ref()
    }

    #[must_use]
    pub fn script(&self) -> Option<&str> {
        self.script.as_deref()
    }

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

#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub struct WordListMetadataBuilder(WordListMetadata);

impl WordListMetadataBuilder {
    pub fn new(word_list_name: impl Into<Cow<'static, str>>) -> Self {
        Self(WordListMetadata {
            name: word_list_name.into(),
            script: None,
            language: None,
        })
    }

    pub fn script(self, script: impl Into<Cow<'static, str>>) -> Self {
        Self(WordListMetadata {
            script: Some(script.into()),
            ..self.0
        })
    }

    pub fn language(self, language: impl Into<Cow<'static, str>>) -> Self {
        Self(WordListMetadata {
            language: Some(language.into()),
            ..self.0
        })
    }

    pub fn build(self) -> WordListMetadata {
        self.into()
    }
}

impl From<WordListMetadataBuilder> for WordListMetadata {
    fn from(builder: WordListMetadataBuilder) -> Self {
        builder.0
    }
}

impl From<WordListMetadata> for WordListMetadataBuilder {
    fn from(metadata: WordListMetadata) -> Self {
        Self(metadata)
    }
}
