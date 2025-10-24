use skrifa::raw::types::InvalidTag;
use thiserror::Error;

/// Creating the shaping plan failed.
///
/// # What is a shaping plan?
///
/// A shaping plan is a HarfBuzz/[`harfrust`] optimisation where you inform it
/// ahead-of-time about the text you're going to give it, telling it things like
/// the direction, script, and language of the text. You can read more about
/// this [here](https://harfbuzz.github.io/shaping-plans-and-caching.html).
// TODO: re-home these docs back in Font Height somewhere
// # If it's just an optimisation technique, why is this a fatal error for Font
// Height?
//
// This error will only occur if the [`WordList`](crate::WordList) has
// metadata and it's unable to be used. [`WordList`](crate::WordList)s without
// metadata will not cause this error.
#[derive(Debug, Error)]
pub enum ShapingPlanError {
    /// The script metadata value was
    /// invalid
    #[error("invalid script: {0}")]
    UnknownScriptTag(#[from] InvalidTagError),
    /// The language metadata value was
    /// invalid
    #[error("invalid language: {0}")]
    UnknownLanguage(#[from] HarfRustUnknownLanguageError),
}

/// [`harfrust`] didn't recognise the language.
#[derive(Debug, Error)]
#[error("invalid language: \"{language}\"")]
pub struct HarfRustUnknownLanguageError {
    language: String,
}

impl HarfRustUnknownLanguageError {
    pub(crate) fn new(lang: impl Into<String>) -> Self {
        HarfRustUnknownLanguageError {
            language: lang.into(),
        }
    }
}

/// Returned by [`Location::validate_for`](crate::Location::validate_for),
/// indicating axes are specified in the [`Location`](crate::Location) that
/// aren't in the font being validated against.
#[derive(Debug, Error)]
#[error("mismatched axes: present in Location but not font {extras:?}")]
pub struct MismatchedAxesError {
    pub(crate) extras: Vec<skrifa::Tag>,
}

/// The axis/script tag was invalid (it had illegal characters or wasn't four
/// characters).
#[derive(Debug, Error)]
#[error(transparent)]
pub struct InvalidTagError(#[from] pub(crate) InvalidTag);
