use std::collections::HashMap;

use kurbo::Shape;
pub use locations::SimpleLocation;
use ordered_float::OrderedFloat;
use rustybuzz::UnicodeBuffer;
use skrifa::{
    instance::Size,
    outline::{DrawError, DrawSettings},
    MetadataProvider,
};
use summary::ReportSummarizer;
pub use summary::ReportSummary;
use thiserror::Error;

use crate::{locations::interesting_locations, pens::BezierPen};

mod locations;
mod pens;
mod summary;
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
        &'a self,
        location: &'a Location,
        word_list: &'a WordList,
    ) -> Result<ReportIterator<'a>, SkrifaDrawError> {
        let mut rusty_face = self.rusty_face.clone();
        rusty_face.set_variations(&location.to_rustybuzz());

        let instance_extremes =
            InstanceExtremes::new(&self.skrifa_font, location)?;

        Ok(ReportIterator {
            rusty_face,
            word_iter: word_list.iter(),
            instance_extremes,
        })
    }
}

pub struct ReportIterator<'a> {
    rusty_face: rustybuzz::Face<'a>,
    instance_extremes: InstanceExtremes,
    word_iter: WordListIter<'a>,
}

impl<'a> ReportIterator<'a> {
    pub fn collect_min_max_extremes(self, n: usize) -> ReportSummary<'a> {
        self.fold(ReportSummarizer::new(n), |mut acc, report| {
            acc.push(report);
            acc
        })
        .build()
    }
}

impl<'a> Iterator for ReportIterator<'a> {
    type Item = Report<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        let word = self.word_iter.next()?;
        let mut buffer = UnicodeBuffer::new();
        buffer.push_str(word);
        buffer.guess_segment_properties();
        let glyph_buffer = rustybuzz::shape(&self.rusty_face, &[], buffer);
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
                    heights.lowest + y_offset as f64,
                    heights.highest + y_offset as f64,
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

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
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
                    lowest: y0.into(),
                    highest: y1.into(),
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

#[derive(Debug, Clone, Copy, Default, Eq, PartialEq)]
pub struct VerticalExtremes {
    lowest: OrderedFloat<f64>,
    highest: OrderedFloat<f64>,
}

impl VerticalExtremes {
    #[inline]
    pub fn lowest(&self) -> f64 {
        *self.lowest
    }

    #[inline]
    pub fn highest(&self) -> f64 {
        *self.highest
    }
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
