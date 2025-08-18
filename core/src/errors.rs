//! The errors produced by Font Height.
//!
//! Hopefully you don't have to get too familiar with these 😃
//!
//! The top-level catch-all error is [`FontHeightError`], but individual APIs
//! may return more specific errors (all of which will up-convert to
//! [`FontHeightError`] one way or another).

use skrifa::{outline::DrawError, raw::types::InvalidTag};
use thiserror::Error;

/// Font Height hit an error, sorry!
#[non_exhaustive]
#[derive(Debug, Error)]
pub enum FontHeightError {
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
    /// Invalid metadata for a [`WordList`](crate::WordList) meant creating a
    /// shaping plan for it failed.
    #[error(transparent)]
    WordListMetadata(#[from] ShapingPlanError),
}

/// Creating the shaping plan for a [`WordList`](crate::WordList) failed.
///
/// # What is a shaping plan?
///
/// A shaping plan is a HarfBuzz/[`harfrust`] optimisation where you inform it
/// ahead-of-time about the text you're going to give it, telling it things like
/// the direction, script, and language of the text. You can read more about
/// this [here](https://harfbuzz.github.io/shaping-plans-and-caching.html).
///
/// # If it's just an optimisation technique, why is this a fatal error for Font Height?
///
/// This error will only occur if the [`WordList`](crate::WordList) has
/// metadata and it's unable to used. [`WordList`](crate::WordList)s without
/// metadata will not cause this error.
#[derive(Debug, Error)]
pub enum ShapingPlanError {
    /// The script metadata value on the [`WordList`](crate::WordList) was
    /// invalid
    #[error(
        "invalid script in word list metadata for {word_list_name}: {inner}"
    )]
    UnknownScriptTag {
        /// The name of the word list that had invalid metadata
        word_list_name: String,
        /// The underlying error
        inner: InvalidTagError,
    },
    /// The language metadata value on the [`WordList`](crate::WordList) was
    /// invalid
    #[error(
        "invalid language in word list metadata for {word_list_name}: {inner}"
    )]
    UnknownLanguage {
        /// The name of the word list that had invalid metadata
        word_list_name: String,
        /// The underlying error
        inner: HarfRustUnknownLanguageError,
    },
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
/// Skrifa could not parse the font.
#[derive(Debug, Error)]
#[error(transparent)]
pub struct SkrifaReadError(#[from] skrifa::raw::ReadError);

/// [`skrifa`] failed to draw a glyph.
#[derive(Debug, Error)]
#[error("could not draw glyph {0}: {1}")]
pub struct SkrifaDrawError(pub(crate) skrifa::GlyphId, pub(crate) DrawError);

/// Returned by [`Location::validate_for`](crate::Location::validate_for),
/// indicating axes are specified in the [`Location`](crate::Location) that
/// aren't in the font being validated against.
#[derive(Debug, Error)]
#[error("mismatched axes: present in Location but not font {extras:?}")]
pub struct MismatchedAxesError {
    pub(crate) extras: Vec<skrifa::Tag>,
}

/// The axis/script tag was invalid (it had illegal characters or was too long).
#[derive(Debug, Error)]
#[error(transparent)]
pub struct InvalidTagError(#[from] InvalidTag);
