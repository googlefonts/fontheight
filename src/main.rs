mod locations;
mod pens;
mod word_lists;

use std::{collections::HashMap, fs, path::PathBuf, process::ExitCode};

use anyhow::{anyhow, Context};
use clap::Parser;
use env_logger::Env;
use kurbo::Shape;
use locations::{interesting_locations, Location};
use log::{error, info, LevelFilter};
use rustybuzz::UnicodeBuffer;
use skrifa::{outline::DrawSettings, prelude::Size, MetadataProvider};

use crate::{pens::BezierPen, word_lists::WordList};

fn main() -> ExitCode {
    env_logger::builder()
        .filter_level(if cfg!(debug_assertions) {
            LevelFilter::Debug
        } else {
            LevelFilter::Warn
        })
        .parse_env(Env::new().filter("FONTHEIGHT_LOG"))
        .init();
    match _main() {
        Ok(()) => ExitCode::SUCCESS,
        Err(why) => {
            error!("{why}");
            ExitCode::FAILURE
        },
    }
}

#[derive(Debug, Parser)]
#[command(version, about)]
struct Args {
    /// The TTF to analyze
    font_path: PathBuf,
}

fn _main() -> anyhow::Result<()> {
    let args = Args::parse();

    let font_bytes =
        fs::read(&args.font_path).context("failed to read font file")?;

    let mut font_face = rustybuzz::Face::from_slice(&font_bytes, 0)
        .context("rustybuzz could not parse font")?;

    let skrifa_font = skrifa::FontRef::new(&font_bytes)
        .context("skrifa could not parse font")?;

    let locations = interesting_locations(&skrifa_font);

    info!("testing font at {} locations", locations.len());
    locations
        .iter()
        .try_for_each(|location| -> anyhow::Result<()> {
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
        })?;

    Ok(())
}

#[derive(Debug)]
struct InstanceExtremes(HashMap<u32, VerticalExtremes>);

impl InstanceExtremes {
    pub fn new(
        font: &skrifa::FontRef,
        location: &Location,
    ) -> anyhow::Result<Self> {
        let instance_extremes = font
            .outline_glyphs()
            .iter()
            .map(|(id, outline)| -> anyhow::Result<(u32, VerticalExtremes)> {
                let mut bez_pen = BezierPen::default();
                outline
                    .draw(
                        DrawSettings::unhinted(
                            Size::unscaled(),
                            &location.to_skrifa(font),
                        ),
                        &mut bez_pen,
                    )
                    .map_err(|err| {
                        anyhow!("could not draw glyph {id}: {err}")
                    })?;

                let kurbo::Rect { y0, y1, .. } = bez_pen.path.bounding_box();
                Ok((u32::from(id), VerticalExtremes {
                    lowest: y0,
                    highest: y1,
                }))
            })
            .collect::<anyhow::Result<HashMap<_, _>>>()?;
        Ok(InstanceExtremes(instance_extremes))
    }

    pub fn get(&self, glyph_id: u32) -> Option<VerticalExtremes> {
        self.0.get(&glyph_id).copied()
    }
}

#[derive(Debug, Clone, Copy, Default)]
struct VerticalExtremes {
    lowest: f64,
    highest: f64,
}
