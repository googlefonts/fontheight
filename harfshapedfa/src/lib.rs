#![cfg_attr(docsrs, feature(doc_cfg))]
#![doc = include_str!("../README.md")]

use std::str::FromStr;

use harfrust::{
    Direction, Feature, GlyphBuffer, Language, Script, ShapePlan, Shaper, Tag,
    UnicodeBuffer,
};
pub use location::*;

use crate::{
    convert::direction_from_script,
    errors::{HarfRustUnknownLanguageError, InvalidTagError, ShapingPlanError},
};

/// Helper functions for converting between differing standards.
pub mod convert;
/// Something went wrong!
pub mod errors;
mod location;
/// Pens, used to transform or calculate information about glyph outlines.
///
/// A pen is a kind of object that standardizes the way how to "draw" outlines:
/// it is a middle man between an outline and a drawing. In other words: it is
/// an abstraction for drawing outlines, making sure that outline objects don’t
/// need to know the details about how and where they’re being drawn, and that
/// drawings don’t need to know the details of how outlines are stored.
// ^ re-used from: https://fonttools.readthedocs.io/en/latest/pens/basePen.html
#[cfg(feature = "pens")]
pub mod pens;

/// Re-exports from [`kurbo`](::kurbo)
///
/// This should cover the API surface that [`pens`] exposes.
#[cfg(feature = "pens")]
pub mod kurbo {
    pub use kurbo::{BezPath, PathEl, Point, Rect};
}

/// Metadata related to shaping.
///
/// Stores information on script, language, direction, and the resultant
/// [`harfrust::ShapePlan`] that this produces.
///
/// See [`Shaper::shape_with_meta`](HarfRustShaperExt::shape_with_meta) &
/// [`UnicodeBuffer::configure_with_meta`](HarfRustBufferExt::configure_with_meta)
/// for usage.
pub struct ShapingMeta {
    shaping_plan: ShapePlan,
    script: Script,
    direction: Direction,
    language: Option<Language>,
}

impl ShapingMeta {
    /// Create a new `ShapingMeta`.
    ///
    /// Errors if `script` or `language` are invalid/unrecognised.
    pub fn new(
        script: &str,
        language: Option<&str>,
        shaper: &Shaper,
    ) -> Result<Self, ShapingPlanError> {
        let script_tag = script.parse::<Tag>().map_err(InvalidTagError)?;
        // Unwrap is safe here as script_tag is never null as [0, 0, 0, 0] isn't
        // a valid Rust string
        let script = Script::from_iso15924_tag(script_tag).unwrap();

        let language = language
            .map(|lang| {
                // harfrust's own error here is just "invalid language"
                // (v0.3.1), so discard it for our own
                Language::from_str(lang)
                    .map_err(|_| HarfRustUnknownLanguageError::new(lang))
            })
            .transpose()?;
        let direction =
            direction_from_script(script).unwrap_or(Direction::LeftToRight);

        let shaping_plan = ShapePlan::new(
            shaper,
            direction,
            Some(script),
            language.as_ref(),
            // Default features are still included by default
            &[],
        );

        Ok(Self {
            shaping_plan,
            script,
            direction,
            language,
        })
    }

    /// Get access to the inner [`ShapePlan`].
    #[must_use]
    pub const fn shaping_plan(&self) -> &ShapePlan {
        &self.shaping_plan
    }
}

/// Extension trait for [`harfrust::UnicodeBuffer`].
pub trait HarfRustBufferExt: private::Sealed {
    /// Configures the buffer with script/language/direction information from
    /// [`ShapingMeta`].
    fn configure_with_meta(&mut self, meta: &ShapingMeta);
}

impl HarfRustBufferExt for UnicodeBuffer {
    fn configure_with_meta(&mut self, meta: &ShapingMeta) {
        self.set_script(meta.script);
        if let Some(lang) = meta.language.clone() {
            self.set_language(lang);
        }
        self.set_direction(meta.direction);
    }
}

/// Extension trait for [`harfrust::Shaper`].
pub trait HarfRustShaperExt: private::Sealed {
    /// A convenience method that configures the buffer and then shapes it.
    ///
    /// Equivalent to:
    // TODO: make this code sample compile & run
    /// ```ignore
    /// buffer.configure_with_meta(meta);
    /// shaper.shape_with_plan(meta.shaping_plan(), buffer, features)
    /// ```
    fn shape_with_meta(
        &self,
        meta: &ShapingMeta,
        buffer: UnicodeBuffer,
        features: &[Feature],
    ) -> GlyphBuffer;
}

impl HarfRustShaperExt for Shaper<'_> {
    fn shape_with_meta(
        &self,
        meta: &ShapingMeta,
        mut buffer: UnicodeBuffer,
        features: &[Feature],
    ) -> GlyphBuffer {
        buffer.configure_with_meta(meta);
        self.shape_with_plan(meta.shaping_plan(), buffer, features)
    }
}

mod private {
    use harfrust::{Shaper, UnicodeBuffer};
    pub trait Sealed {}
    impl Sealed for UnicodeBuffer {}
    impl Sealed for Shaper<'_> {}
}
