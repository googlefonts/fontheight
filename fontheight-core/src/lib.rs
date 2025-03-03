use std::collections::HashMap;

use kurbo::Shape;
use rustybuzz::UnicodeBuffer;
use skrifa::{
    instance::Size,
    outline::{DrawError, DrawSettings},
    MetadataProvider,
};
use thiserror::Error;
use unicode_script::UnicodeScript;
pub use unicode_script::{Script, ScriptExtension};

use crate::{locations::interesting_locations, pens::BezierPen};

mod locations;
mod pens;
mod word_lists;

pub use locations::Location;
pub use word_lists::*;

pub struct Reporter<'a> {
    rusty_face: rustybuzz::Face<'a>,
    skrifa_font: skrifa::FontRef<'a>,
}

impl<'a> Reporter<'a> {
    pub fn new(font_bytes: &'a [u8]) -> Result<Self, FontHeightError> {
        let rusty_face = rustybuzz::Face::from_slice(font_bytes, 0)
            .ok_or(FontHeightError::Rustybuzz)?;

        let skrifa_font = skrifa::FontRef::new(font_bytes)?;

        Ok(Reporter {
            rusty_face,
            skrifa_font,
        })
    }

    pub fn interesting_locations(&self) -> Vec<Location> {
        interesting_locations(&self.skrifa_font)
    }

    pub fn check_location(
        &'a mut self,
        location: &'a Location,
        word_list: &'a WordList,
    ) -> Result<ReportIterator<'a>, SkrifaDrawError> {
        self.rusty_face.set_variations(&location.to_rustybuzz());

        let instance_extremes =
            InstanceExtremes::new(&self.skrifa_font, location)?;

        Ok(ReportIterator {
            parent: self,
            word_iter: word_list.iter(),
            instance_extremes,
        })
    }
}

pub struct ReportIterator<'a> {
    parent: &'a Reporter<'a>,
    instance_extremes: InstanceExtremes,
    word_iter: WordListIter<'a>,
}

impl<'a> Iterator for ReportIterator<'a> {
    type Item = Report<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        let word = self.word_iter.next()?;
        let mut buffer = UnicodeBuffer::new();
        buffer.push_str(word);
        buffer.guess_segment_properties();
        let glyph_buffer =
            rustybuzz::shape(&self.parent.rusty_face, &[], buffer);
        // TODO: remove empty glyphs and/or .notdef?
        let word_extremes = glyph_buffer
            .glyph_infos()
            .iter()
            .zip(glyph_buffer.glyph_positions())
            .map(|(info, pos)| {
                let y_offset = pos.y_offset;
                let heights =
                    self.instance_extremes.get(info.glyph_id).unwrap();

                (
                    y_offset as f64 + heights.lowest,
                    y_offset as f64 + heights.highest,
                )
            })
            .fold(VerticalExtremes::default(), |extremes, (low, high)| {
                let VerticalExtremes { highest, lowest } = extremes;
                VerticalExtremes {
                    highest: highest.max(high),
                    lowest: lowest.min(low),
                }
            });
        Some(Report {
            word,
            extremes: word_extremes,
        })
    }
}

#[derive(Debug)]
pub struct Report<'w> {
    pub word: &'w str,
    pub extremes: VerticalExtremes,
}

#[derive(Debug)]
pub struct InstanceExtremes(HashMap<u32, VerticalExtremes>);

impl InstanceExtremes {
    pub fn new(
        font: &skrifa::FontRef,
        location: &Location,
    ) -> Result<Self, SkrifaDrawError> {
        let instance_extremes = font
            .outline_glyphs()
            .iter()
            .map(|(id, outline)| -> Result<(u32, VerticalExtremes), SkrifaDrawError> {
                let mut bez_pen = BezierPen::default();
                outline
                    .draw(
                        DrawSettings::unhinted(
                            Size::unscaled(),
                            &location.to_skrifa(font),
                        ),
                        &mut bez_pen,
                    )
                    .map_err(|err| SkrifaDrawError(id, err))?;

                let kurbo::Rect { y0, y1, .. } = bez_pen.path.bounding_box();
                Ok((u32::from(id), VerticalExtremes {
                    lowest: y0,
                    highest: y1,
                }))
            })
            .collect::<Result<HashMap<_, _>, _>>()?;
        Ok(InstanceExtremes(instance_extremes))
    }

    pub fn get(&self, glyph_id: u32) -> Option<VerticalExtremes> {
        self.0.get(&glyph_id).copied()
    }
}

#[derive(Debug, Error)]
#[error("could not draw glyph {0}: {1}")]
pub struct SkrifaDrawError(skrifa::GlyphId, DrawError);

#[derive(Debug, Clone, Copy, Default)]
pub struct VerticalExtremes {
    pub lowest: f64,
    pub highest: f64,
}

#[derive(Debug, Error)]
pub enum FontHeightError {
    #[error("rustybuzz could not parse the font")]
    Rustybuzz,
    #[error("skrifa could not parse the font: {0}")]
    Skrifa(#[from] skrifa::raw::ReadError),
    #[error(transparent)]
    Drawing(#[from] SkrifaDrawError),
}

pub fn discover_font_scripts(
    font: &skrifa::FontRef,
) -> Result<ScriptExtension, ScriptDiscoveryError> {
    let mut empty_cmap = true;
    let scripts = font.charmap().mappings().try_fold(
        ScriptExtension::default(),
        |acc, (codepoint, _)| {
            empty_cmap = false;
            let script_extension = char::from_u32(codepoint)
                .ok_or(ScriptDiscoveryError::InvalidCodepoint(codepoint))?
                .script_extension();
            Ok(acc.union(script_extension))
        },
    )?;
    if empty_cmap {
        Err(ScriptDiscoveryError::EmptyCmap)
    } else {
        Ok(scripts)
    }
}

#[derive(Debug, Error)]
pub enum ScriptDiscoveryError {
    #[error("invalid codepoint in cmap: {0:#x}")]
    InvalidCodepoint(u32),
    #[error("empty cmap")]
    EmptyCmap,
}
