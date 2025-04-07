use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct WordListMetadata {
    pub(crate) name: String,
    pub(crate) script: Option<String>,
    pub(crate) language: Option<String>,
}
