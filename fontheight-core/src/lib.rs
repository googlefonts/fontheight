#![cfg_attr(docsrs, feature(doc_auto_cfg))]

use std::collections::HashSet;

use diffenator3_lib::{dfont::DFont, render::renderer::Renderer};
pub use exemplars::Exemplars;
pub use locations::SimpleLocation;
use ordered_float::OrderedFloat;
use skrifa::outline::DrawError;
use thiserror::Error;
use zeno::Command;

use crate::locations::interesting_locations;

mod exemplars;
mod locations;
mod pens;
mod word_lists;

pub use exemplars::CollectToExemplars;
pub use locations::Location;
#[cfg(feature = "rayon")]
pub use word_lists::rayon::ParWordListIter;
pub use word_lists::*;

pub struct Reporter<'a> {
    dfont: DFont,
    skrifa_font: skrifa::FontRef<'a>,
}

impl<'a> Reporter<'a> {
    pub fn new(font_bytes: &'a [u8]) -> Result<Self, FontHeightError> {
        let skrifa_font = skrifa::FontRef::new(font_bytes)?;
        let dfont = DFont::new(font_bytes);

        Ok(Reporter { dfont, skrifa_font })
    }

    pub fn interesting_locations(&self) -> Vec<Location> {
        interesting_locations(&self.skrifa_font)
    }

    pub fn check_location(
        &'a mut self,
        location: &'a Location,
        word_list: &'a WordList,
    ) -> Result<WordExtremesIterator<'a>, FontHeightError> {
        self.dfont.normalized_location = location.to_skrifa(&self.skrifa_font);

        Ok(WordExtremesIterator {
            renderer: Renderer::new(
                &self.dfont,
                1024.0,
                rustybuzz::Direction::LeftToRight,
                Some(rustybuzz::script::LATIN),
            ),
            codepoints: &self.dfont.codepoints,
            word_iter: word_list.iter(),
        })
    }

    #[cfg(feature = "rayon")]
    pub fn par_check_location(
        &'a self,
        location: &'a Location,
        word_list: &'a WordList,
        k_words: usize,
        n_exemplars: usize,
    ) -> Result<Exemplars<'a>, FontHeightError> {
        use std::cell::RefCell;

        use ::rayon::iter::{IndexedParallelIterator, ParallelIterator};
        use exemplars::ExemplarCollector;
        use skrifa::raw::TableProvider;
        use thread_local::ThreadLocal;

        let mut dfont = self.dfont.clone();
        dfont.location = location.to_settings();
        dfont.normalize_location();
        let upem = dfont.fontref().head()?.units_per_em() as f32;

        let tl_a = ThreadLocal::new();

        let Some((direction, script)) = word_list.properties() else {
            return Err(FontHeightError::WordListProperties);
        };

        let collector = word_list
            .par_iter()
            .take(k_words)
            .filter(|word| {
                word.chars().all(|c| dfont.codepoints.contains(&(c as u32)))
            })
            .flat_map(|word| {
                let renderer = tl_a.get_or(|| {
                    RefCell::new(Renderer::new(
                        &dfont,
                        upem,
                        direction,
                        Some(script),
                    ))
                });
                renderer
                    .borrow_mut()
                    .string_to_positioned_glyphs(word)
                    .map(|(_buffer, commands)| (word, commands))
            })
            .map(|(word, commands)| {
                let extremes = fold_extremes(&commands);
                WordExtremes { word, extremes }
            })
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
    renderer: Renderer<'a>,
    codepoints: &'a HashSet<u32>,
    word_iter: WordListIter<'a>,
}

impl<'a> Iterator for WordExtremesIterator<'a> {
    type Item = WordExtremes<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        // Consume words until we get a shaped buffer without .notdefs
        let (word, commands) = self.word_iter.find_map(|word| {
            if word.chars().all(|c| self.codepoints.contains(&(c as u32))) {
                self.renderer
                    .string_to_positioned_glyphs(word)
                    .map(|(_buffer, commands)| (word, commands))
            } else {
                None
            }
        })?;

        let word_extremes = fold_extremes(&commands);

        Some(WordExtremes {
            word,
            extremes: word_extremes,
        })
    }
}

fn fold_extremes(commands: &[Command]) -> VerticalExtremes {
    commands
        .iter()
        .fold(VerticalExtremes::default(), |extremes, command| {
            if let Some((high, low)) = command_extremes(command) {
                let VerticalExtremes { highest, lowest } = extremes;
                VerticalExtremes {
                    highest: highest.max(OrderedFloat::from(high as f64)),
                    lowest: lowest.min(OrderedFloat::from(low as f64)),
                }
            } else {
                extremes
            }
        })
}

fn command_extremes(command: &Command) -> Option<(f32, f32)> {
    match command {
        Command::Close => None,
        Command::MoveTo(p) => Some((p.y, p.y)),
        Command::LineTo(p) => Some((p.y, p.y)),
        Command::QuadTo(p1, p2) => Some((p1.y.max(p2.y), p1.y.min(p2.y))),
        Command::CurveTo(p1, p2, p3) => {
            Some((p1.y.max(p2.y).max(p3.y), p1.y.min(p2.y).min(p3.y)))
        },
    }
}

#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
pub struct WordExtremes<'w> {
    pub word: &'w str,
    pub extremes: VerticalExtremes,
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
    #[error("could not determine the wordlist properties")]
    WordListProperties,
}
