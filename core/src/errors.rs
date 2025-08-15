//! The errors produced by Font Height.
//!
//! Hopefully you don't have to get too familiar with these 😃
use skrifa::{outline::DrawError, raw::types::InvalidTag};
use thiserror::Error;

/// Font Height hit an error, sorry!
#[non_exhaustive]
#[derive(Debug, Error)]
pub enum FontHeightError {
    /// [`harfrust`] didn't recognise the language of the word list you chose.
    #[error("rustybuzz did not recognise the language: {0}")]
    HarfRustUnknownLanguage(#[from] HarfRustUnknownLanguageError),
    /// [`skrifa`] could not parse the font.
    #[error("skrifa could not parse the font: {0}")]
    Skrifa(#[from] SkrifaReadError),
    /// An axis tag you provided was invalid.
    #[error("invalid tag: {0}")]
    InvalidTag(InvalidTagError),
    /// We couldn't shape the text.
    #[error(transparent)]
    Drawing(#[from] SkrifaDrawError),
    /// The axes your [`Location`](crate::Location) specified didn't match
    /// those in the font.
    #[error(transparent)]
    MismatchedAxes(#[from] MismatchedAxesError),
}

/// [`harfrust`] didn't recognise the language
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

// New-typed errors to not have 3rd party errors in public API
/// Skrifa could not parse the font
#[derive(Debug, Error)]
#[error(transparent)]
pub struct SkrifaReadError(#[from] skrifa::raw::ReadError);

/// [`skrifa`] failed to draw a glyph.
#[derive(Debug, Error)]
#[error("could not draw glyph {0}: {1}")]
pub struct SkrifaDrawError(pub(crate) skrifa::GlyphId, pub(crate) DrawError);

/// Returned by [`Location::validate_for`](crate::Location::validate_for),
/// indicating axes are specified in the [`Location`](crate::Location) that
/// aren't in the `font`.
#[derive(Debug, Error)]
#[error("mismatched axes: present in Location but not font {extras:?}")]
pub struct MismatchedAxesError {
    pub(crate) extras: Vec<skrifa::Tag>,
}

/// The axis tag was invalid (illegal characters, too long)
#[derive(Debug, Error)]
#[error(transparent)]
pub struct InvalidTagError(#[from] InvalidTag);
