use std::collections::HashMap;

use kurbo::Shape;
use log::info;
use rustybuzz::UnicodeBuffer;
use skrifa::{
    instance::Size,
    outline::{DrawError, DrawSettings},
    MetadataProvider,
};
use thiserror::Error;

use crate::{locations::interesting_locations, pens::BezierPen};

mod locations;
mod pens;
mod word_lists;

pub use locations::Location;
pub use word_lists::*;

pub fn the_owl(font_bytes: impl AsRef<[u8]>) -> Result<(), FontHeightError> {
    let font_bytes = font_bytes.as_ref();
    let mut font_face = rustybuzz::Face::from_slice(font_bytes, 0)
        .ok_or(FontHeightError::Rustybuzz)?;

    let skrifa_font = skrifa::FontRef::new(font_bytes)?;

    let locations = interesting_locations(&skrifa_font);

    info!("testing font at {} locations", locations.len());
    locations.iter().try_for_each(
        |location| -> Result<(), SkrifaDrawError> {
            font_face.set_variations(&location.to_rustybuzz());

            let instance_extremes =
                InstanceExtremes::new(&skrifa_font, location)?;

            let test_words = WordList::define("test", ["hello", "apple"]);
            test_words.iter().for_each(|word| {
                let mut buffer = UnicodeBuffer::new();
                buffer.push_str(word);
                buffer.guess_segment_properties();
                let glyph_buffer = rustybuzz::shape(&font_face, &[], buffer);
                // TODO: remove empty glyphs and/or .notdef?
                let _word_extremes = glyph_buffer
                    .glyph_infos()
                    .iter()
                    .zip(glyph_buffer.glyph_positions())
                    .map(|(info, pos)| {
                        let y_offset = pos.y_offset;
                        let heights =
                            instance_extremes.get(info.glyph_id).unwrap();

                        (
                            y_offset as f64 + heights.lowest,
                            y_offset as f64 + heights.highest,
                        )
                    })
                    .fold(
                        VerticalExtremes::default(),
                        |extremes, (low, high)| {
                            let VerticalExtremes { highest, lowest } = extremes;
                            VerticalExtremes {
                                highest: highest.max(high),
                                lowest: lowest.min(low),
                            }
                        },
                    );
            });

            Ok(())
        },
    )?;
    Ok(())
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
