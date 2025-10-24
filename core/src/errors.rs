//! The errors produced by Font Height.
//!
//! Hopefully you don't have to get too familiar with these 😃
//!
//! The top-level catch-all error is [`FontHeightError`], but individual APIs
//! may return more specific errors (all of which will up-convert to
//! [`FontHeightError`] one way or another).

use harfshapedfa::errors::ShapingPlanError;
pub use harfshapedfa::errors::{InvalidTagError, MismatchedAxesError};
use skrifa::outline::DrawError;
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
    /// Extracting outlines from the font failed.
    #[error(transparent)]
    Drawing(#[from] SkrifaDrawError),
    /// The axes your [`Location`](crate::Location) specified didn't match
    /// those in the font.
    #[error(transparent)]
    MismatchedAxes(#[from] MismatchedAxesError),
    /// Invalid metadata for a [`WordList`](crate::WordList) meant creating a
    /// shaping plan for it failed.
    #[error(transparent)]
    WordListMetadata(#[from] WordListShapingPlanError),
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
/// metadata and it's unable to be used. [`WordList`](crate::WordList)s without
/// metadata will not cause this error.
#[derive(Debug, Error)]
#[error("couldn't make shaping plan for {word_list_name}: {inner}")]
pub struct WordListShapingPlanError {
    pub(crate) word_list_name: String,
    pub(crate) inner: ShapingPlanError,
}

// New-typed errors to not have 3rd party errors in public API
/// Skrifa could not parse the font.
#[derive(Debug, Error)]
#[error(transparent)]
pub struct SkrifaReadError(#[from] skrifa::raw::ReadError);

/// [`skrifa`] failed to extract outlines for a glyph.
#[derive(Debug, Error)]
#[error("could not draw glyph {0}: {1}")]
pub struct SkrifaDrawError(pub(crate) skrifa::GlyphId, pub(crate) DrawError);
