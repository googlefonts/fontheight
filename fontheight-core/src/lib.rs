#![cfg_attr(docsrs, feature(doc_auto_cfg))]

use std::{collections::HashMap, convert::identity};

use exemplars::ExemplarCollector;
pub use exemplars::Exemplars;
use kurbo::Shape;
pub use locations::SimpleLocation;
use ordered_float::OrderedFloat;
use rustybuzz::UnicodeBuffer;
use skrifa::{
    instance::Size,
    outline::{DrawError, DrawSettings},
    MetadataProvider,
};
use thiserror::Error;

use crate::{locations::interesting_locations, pens::BezierPen};

mod exemplars;
mod locations;
mod pens;

pub use exemplars::CollectToExemplars;
pub use locations::Location;
pub use static_lang_word_lists::WordList;
use static_lang_word_lists::WordListIter;

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
    ) -> Result<WordExtremesIterator<'a>, SkrifaDrawError> {
        let mut rusty_face = self.rusty_face.clone();
        rusty_face.set_variations(&location.to_rustybuzz());

        let instance_extremes =
            InstanceExtremes::new(&self.skrifa_font, location)?;

        Ok(WordExtremesIterator {
            rusty_face,
            word_iter: word_list.iter(),
            instance_extremes,
            unicode_buffer: Some(UnicodeBuffer::new()),
        })
    }

    #[cfg(feature = "rayon")]
    pub fn par_check_location(
        &'a self,
        location: &'a Location,
        word_list: &'a WordList,
        k_words: Option<usize>,
        n_exemplars: usize,
    ) -> Result<Exemplars<'a>, SkrifaDrawError> {
        use rayon::prelude::*;

        struct WorkerState {
            // UnicodeBuffer is transformed into another type during shaping,
            // and then can only be reverted once we've finished
            // analysing the shaped buffer. The Option allows us to
            // take ownership of it during each iteration for these
            // type changes to happen, while still re-using the buffer
            unicode_buffer: Option<UnicodeBuffer>,
            // shaping_plan: ShapePlan,
        }

        let mut rusty_face = self.rusty_face.clone();
        rusty_face.set_variations(&location.to_rustybuzz());

        let instance_extremes =
            InstanceExtremes::new(&self.skrifa_font, location)?;

        let collector = word_list
            .par_iter()
            .take(k_words.unwrap_or(usize::MAX))
            .map_init(
                || WorkerState {
                    unicode_buffer: Some(UnicodeBuffer::new()),
                },
                |state, word| {
                    // Take buffer; it should always be present
                    let mut buffer = state.unicode_buffer.take().unwrap();

                    buffer.push_str(word);
                    buffer.guess_segment_properties();
                    // TODO: this is where you would use the shaping plan
                    let glyph_buffer =
                        rustybuzz::shape(&rusty_face, &[], buffer);

                    let glyphs_missing = glyph_buffer
                        .glyph_infos()
                        .iter()
                        .any(|info| info.glyph_id == 0); // is .notdef
                    if glyphs_missing {
                        // Return buffer, abort mission
                        state.unicode_buffer = Some(glyph_buffer.clear());
                        return None;
                    }

                    let extremes = glyph_buffer
                        .glyph_infos()
                        .iter()
                        .zip(glyph_buffer.glyph_positions())
                        .map(|(info, pos)| {
                            // TODO: Remove empty glyphs?
                            let y_offset = pos.y_offset;
                            let heights =
                                instance_extremes.get(info.glyph_id).unwrap();

                            (
                                heights.lowest + y_offset as f64,
                                heights.highest + y_offset as f64,
                            )
                        })
                        .fold(
                            VerticalExtremes::default(),
                            |extremes, (low, high)| {
                                let VerticalExtremes { highest, lowest } =
                                    extremes;
                                VerticalExtremes {
                                    highest: highest.max(high),
                                    lowest: lowest.min(low),
                                }
                            },
                        );

                    // Return buffer
                    state.unicode_buffer = Some(glyph_buffer.clear());
                    Some(WordExtremes { word, extremes })
                },
            )
            .filter_map(identity)
            .fold(
                || ExemplarCollector::new(n_exemplars),
                |mut acc, word_extremes| {
                    acc.push(word_extremes);
                    acc
                },
            )
            .reduce(
                || ExemplarCollector::new(n_exemplars),
                |mut acc, other| {
                    acc.merge_with(other);
                    acc
                },
            );

        Ok(collector.build())
    }
}

pub struct WordExtremesIterator<'a> {
    rusty_face: rustybuzz::Face<'a>,
    instance_extremes: InstanceExtremes,
    word_iter: WordListIter<'a>,
    // UnicodeBuffer is transformed into another type during shaping, and then
    // can only be reverted once we've finished analysing the shaped buffer.
    // The Option allows us to take ownership of it during each iteration for
    // these type changes to happen, while still re-using the buffer
    unicode_buffer: Option<UnicodeBuffer>,
}

impl<'a> Iterator for WordExtremesIterator<'a> {
    type Item = WordExtremes<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        debug_assert!(
            self.unicode_buffer.is_some(),
            "WordExtremesIterator.unicode_buffer wasn't stored back in self \
             during the previous iteration"
        );

        // Consume words until we get a shaped buffer without .notdefs
        let (word, glyph_buffer) = self.word_iter.find_map(|word| {
            // Take buffer; it should always be present
            let mut buffer = self.unicode_buffer.take().unwrap();

            buffer.push_str(word);
            buffer.guess_segment_properties();
            let glyph_buffer = rustybuzz::shape(&self.rusty_face, &[], buffer);

            let glyphs_missing = glyph_buffer
                .glyph_infos()
                .iter()
                .any(|info| info.glyph_id == 0); // is .notdef

            if !glyphs_missing {
                // Buffer still held, can't be replaced until after calculating
                // VerticalExtremes
                Some((word, glyph_buffer))
            } else {
                // Return buffer
                self.unicode_buffer = Some(glyph_buffer.clear());
                None
            }
        })?;

        let word_extremes = glyph_buffer
            .glyph_infos()
            .iter()
            .zip(glyph_buffer.glyph_positions())
            .map(|(info, pos)| {
                // TODO: Remove empty glyphs?
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

        // Return buffer
        self.unicode_buffer = Some(glyph_buffer.clear());

        Some(WordExtremes {
            word,
            extremes: word_extremes,
        })
    }
}

#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
pub struct WordExtremes<'w> {
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

#[derive(Debug, Clone, Copy, Default, Eq, PartialEq, Hash)]
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

#[derive(Debug, Clone)]
pub struct Report<'a> {
    pub location: &'a Location,
    pub word_list: &'a WordList,
    pub exemplars: Exemplars<'a>,
}

impl<'a> Report<'a> {
    #[inline]
    pub const fn new(
        location: &'a Location,
        word_list: &'a WordList,
        exemplars: Exemplars<'a>,
    ) -> Self {
        Report {
            location,
            word_list,
            exemplars,
        }
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
