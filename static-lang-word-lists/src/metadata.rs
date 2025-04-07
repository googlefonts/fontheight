use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct WordListMetadata {
    pub(crate) name: String,
    pub(crate) script: Option<String>,
    pub(crate) language: Option<String>,
}
