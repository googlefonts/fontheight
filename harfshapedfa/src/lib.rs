#![allow(missing_docs)] // FIXME: remove this

use std::str::FromStr;

use harfrust::{
    Direction, Feature, GlyphBuffer, Language, Script, ShapePlan, Shaper, Tag,
    UnicodeBuffer,
};
pub use location::*;

use crate::{
    errors::{HarfRustUnknownLanguageError, InvalidTagError, ShapingPlanError},
    utils::direction_from_script,
};

pub mod errors;
mod location;
#[cfg(feature = "pens")]
pub mod pens;
pub mod utils;

#[cfg(feature = "pens")]
pub mod kurbo {
    pub use kurbo::{BezPath, PathEl, Point, Rect};
}

pub struct ShapingMeta {
    shaping_plan: ShapePlan,
    script: Script,
    direction: Direction,
    language: Option<Language>,
}

impl ShapingMeta {
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

    #[must_use]
    pub const fn shaping_plan(&self) -> &ShapePlan {
        &self.shaping_plan
    }
}

pub trait HarfRustBufferExt: private::Sealed {
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

pub trait HarfRustShaperExt: private::Sealed {
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
