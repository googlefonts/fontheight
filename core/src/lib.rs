#![cfg_attr(docsrs, feature(doc_auto_cfg))]
// Copied from the top of the repo README with minor edits
//! Font Height is a tool that provides recommendations on setting font
//! vertical metrics based on **shaped words**.
//!
//! ## Motivation
//!
//! Vertical metrics frequently decide clipping boundaries, but are not used
//! consistently across platforms: e.g. Windows uses OS/2 WinAscent/WinDescent,
//! whereas for system fonts [Android uses TypoAscent/TypoDescent and a combination of custom heuristics](https://simoncozens.github.io/android-clipping/).
//!
//! It is often desirable to derive metrics from shaped words as opposed to
//! individual glyphs, as words may reach greater extents:
//!
//! > Early versions of this specification suggested that the usWinAscent value
//! > be computed as the yMax for all characters in the Windows “ANSI” character
//! > set.
//! > For new fonts, the value should be determined based on the primary
//! > languages the font is designed to support, and should take into
//! > consideration additional height that could be required to accommodate tall
//! > glyphs or mark positioning.
//!
//! ⬆️ [OS/2 — OS/2 and Windows Metrics Table, OpenType Specification 1.9.1](https://learn.microsoft.com/en-us/typography/opentype/spec/os2#uswinascent)
//!
//! For this reason, vertical metrics must be chosen with a combination of
//! design (e.g. aesthetic, legibility) and engineering (e.g. clipping)
//! considerations in mind. For the latter, `fontheight` evaluates the extents
//! of a corpus of shaped text across each writing system that a font intends to
//! support.

use std::{
    collections::{BTreeSet, HashMap},
    convert::identity,
    str::FromStr,
};

use exemplars::ExemplarCollector;
pub use exemplars::{CollectToExemplars, Exemplars};
use itertools::Itertools;
use kurbo::Shape;
pub use locations::{Location, SimpleLocation};
use ordered_float::OrderedFloat;
use pens::BezierPen;
use rustybuzz::UnicodeBuffer;
use skrifa::{MetadataProvider, instance::Size, outline::DrawSettings};
pub use static_lang_word_lists::WordList;
use static_lang_word_lists::WordListIter;

use crate::errors::{
    FontHeightError, RustybuzzUnknownLanguageError, SkrifaDrawError,
    SkrifaReadError,
};

pub mod errors;
mod exemplars;
mod locations;
mod pens;

/// Font Height's entrypoint. Parses fonts and can check word lists at
/// specified locations.
pub struct Reporter<'a> {
    rusty_face: rustybuzz::Face<'a>,
    skrifa_font: skrifa::FontRef<'a>,
}

impl<'a> Reporter<'a> {
    /// Parses the byte slice as a font to create a new [`Reporter`].
    ///
    /// Fails if the bytes couldn't be parsed.
    pub fn new(font_bytes: &'a [u8]) -> Result<Self, FontHeightError> {
        let rusty_face = rustybuzz::Face::from_slice(font_bytes, 0)
            .ok_or(FontHeightError::RustybuzzParse)?;

        let skrifa_font =
            skrifa::FontRef::new(font_bytes).map_err(SkrifaReadError::from)?;

        Ok(Reporter {
            rusty_face,
            skrifa_font,
        })
    }

    /// Access the [`rustybuzz`]-parsed font.
    ///
    /// ⚠️ Warning: changes to the return type of this function (i.e. by
    /// `fontheight` changing [`rustybuzz`] version) are **not** covered by this
    /// crate's efforts to follow SemVer.
    #[inline]
    #[must_use]
    pub fn rustybuzz_face(&self) -> &rustybuzz::Face<'_> {
        &self.rusty_face
    }

    /// Access the [`skrifa`]-parsed font.
    ///
    /// ⚠️ Warning: changes to the return type of this function (i.e. by
    /// `fontheight` changing [`skrifa`] version) are **not** covered by this
    /// crate's efforts to follow SemVer.
    #[inline]
    #[must_use]
    pub fn skrifa_fontref(&self) -> &skrifa::FontRef<'_> {
        &self.skrifa_font
    }

    /// Gets all combinations of axis coordinates seen in named instances, axis
    /// extremes, and default locations.
    #[must_use]
    pub fn interesting_locations(&self) -> Vec<Location> {
        let font = &self.skrifa_font;
        let mut axis_coords =
            vec![BTreeSet::<OrderedFloat<f32>>::new(); font.axes().len()];

        font.named_instances()
            .iter()
            .flat_map(|instance| instance.user_coords().enumerate())
            .for_each(|(axis, coord)| {
                axis_coords[axis].insert(coord.into());
            });

        font.axes().iter().for_each(|axis| {
            axis_coords[axis.index()].extend(&[
                axis.default_value().into(),
                axis.min_value().into(),
                axis.max_value().into(),
            ]);
        });

        axis_coords
            .iter()
            .multi_cartesian_product()
            .map(|coords| {
                let inner = coords
                    .into_iter()
                    .zip(font.axes().iter())
                    .map(|(coord, axis)| (axis.tag(), From::from(*coord)))
                    .collect();
                Location::from_skrifa(inner)
            })
            .collect()
    }

    /// Create an iterator for [`WordExtremes`] at a given location.
    ///
    /// Fails if the [`Location`] isn't valid for the font (e.g. specifying axes
    /// that don't exist).
    ///
    /// See [`Location`] if you want to specify locations yourself, or otherwise
    /// you could consider using [`Reporter::interesting_locations`].
    pub fn check_location(
        &'a self,
        location: &'a Location,
        word_list: &'a WordList,
    ) -> Result<WordExtremesIterator<'a>, FontHeightError> {
        // Creating InstanceExtremes also validates the Location; do this first
        let instance_extremes =
            InstanceExtremes::new(&self.skrifa_font, location)?;

        let mut rusty_face = self.rusty_face.clone();
        rusty_face.set_variations(&location.to_rustybuzz());

        Ok(WordExtremesIterator {
            rusty_plan: word_list.shaping_plan(&rusty_face)?,
            rusty_face,
            word_iter: word_list.iter(),
            instance_extremes,
            unicode_buffer: Some(UnicodeBuffer::new()),
        })
    }

    /// Create a parallel iterator for [`WordExtremes`] at a given location.
    ///
    /// Fails if the [`Location`] isn't valid for the font (e.g. specifying axes
    /// that don't exist).
    ///
    /// See [`Location`] if you want to specify locations yourself, or otherwise
    /// you could consider using [`Reporter::interesting_locations`].
    #[cfg(feature = "rayon")]
    pub fn par_check_location(
        &'a self,
        location: &'a Location,
        word_list: &'a WordList,
        k_words: Option<usize>,
        n_exemplars: usize,
    ) -> Result<Exemplars<'a>, FontHeightError> {
        use rayon::prelude::*;
        use rustybuzz::ShapePlan;

        struct WorkerState<'a> {
            // UnicodeBuffer is transformed into another type during shaping,
            // and then can only be reverted once we've finished
            // analysing the shaped buffer. The Option allows us to
            // take ownership of it during each iteration for these
            // type changes to happen, while still re-using the buffer
            unicode_buffer: Option<UnicodeBuffer>,
            shaping_plan: Option<&'a ShapePlan>,
        }

        // Creating InstanceExtremes also validates the Location; do this first
        let instance_extremes =
            InstanceExtremes::new(&self.skrifa_font, location)?;

        let mut rusty_face = self.rusty_face.clone();
        rusty_face.set_variations(&location.to_rustybuzz());

        let plan = word_list.shaping_plan(&rusty_face).unwrap_or_default();

        let collector = word_list
            .par_iter()
            .take(k_words.unwrap_or(usize::MAX))
            .map_init(
                || WorkerState {
                    unicode_buffer: Some(UnicodeBuffer::new()),
                    shaping_plan: plan.as_ref(),
                },
                |state, word| {
                    // Take buffer; it should always be present
                    let mut buffer = state.unicode_buffer.take().unwrap();

                    buffer.push_str(word);
                    buffer.guess_segment_properties();
                    // TODO: this is where you would use the shaping plan
                    let glyph_buffer = if let Some(plan) = &state.shaping_plan {
                        rustybuzz::shape_with_plan(&rusty_face, plan, buffer)
                    } else {
                        rustybuzz::shape(&rusty_face, &[], buffer)
                    };

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

/// An iterator of [`WordExtremes`] for one specific font, [`WordList`], and
/// [`Location`].
///
/// Produced by a [`Reporter`].
pub struct WordExtremesIterator<'a> {
    rusty_face: rustybuzz::Face<'a>,
    instance_extremes: InstanceExtremes,
    rusty_plan: Option<rustybuzz::ShapePlan>,
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
            let glyph_buffer = if let Some(plan) = &self.rusty_plan {
                rustybuzz::shape_with_plan(&self.rusty_face, plan, buffer)
            } else {
                rustybuzz::shape(&self.rusty_face, &[], buffer)
            };

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

/// A word and the vertical extremes it reached when shaped.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
pub struct WordExtremes<'w> {
    /// The word that was shaped.
    pub word: &'w str,
    /// The high & low point reached while shaping.
    pub extremes: VerticalExtremes,
}

/// A cache of the vertical bounds for all the glyphs in a font at a certain
/// location.
#[derive(Debug)]
pub(crate) struct InstanceExtremes(HashMap<u32, VerticalExtremes>);

impl InstanceExtremes {
    /// Create the cache for the given `font` at a [`Location`].
    pub fn new(
        font: &skrifa::FontRef,
        location: &Location,
    ) -> Result<Self, FontHeightError> {
        location.validate_for(font)?;
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

    /// Get the [`VerticalExtremes`] for the given glyph ID.
    pub fn get(&self, glyph_id: u32) -> Option<VerticalExtremes> {
        self.0.get(&glyph_id).copied()
    }
}

/// The highest & lowest point on the vertical (y) axis.
#[derive(Debug, Clone, Copy, Default, Eq, PartialEq, Hash)]
pub struct VerticalExtremes {
    lowest: OrderedFloat<f64>,
    highest: OrderedFloat<f64>,
}

impl VerticalExtremes {
    /// The lowest/smaller extreme.
    #[inline]
    #[must_use]
    pub fn lowest(&self) -> f64 {
        *self.lowest
    }

    /// The highest/bigger extreme.
    #[inline]
    #[must_use]
    pub fn highest(&self) -> f64 {
        *self.highest
    }
}

/// A report documenting the furthest extents reached at a location by a word
/// list.
#[derive(Debug, Clone)]
pub struct Report<'a> {
    /// The [`Location`] the exemplars were found at.
    pub location: &'a Location,
    /// The [`WordList`] that was shaped.
    ///
    /// Will always be the full word list, even if only part of it was tested.
    pub word_list: &'a WordList,
    /// The highest & lowest-reaching words shaped.
    pub exemplars: Exemplars<'a>,
}

impl<'a> Report<'a> {
    /// Create a new report from its fields
    #[inline]
    #[must_use]
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

trait WordListExt {
    fn shaping_plan(
        &self,
        rusty_face: &rustybuzz::Face,
    ) -> Result<Option<rustybuzz::ShapePlan>, FontHeightError>;
}

impl WordListExt for WordList {
    fn shaping_plan(
        &self,
        rusty_face: &rustybuzz::Face,
    ) -> Result<Option<rustybuzz::ShapePlan>, FontHeightError> {
        let script = self
            .script()
            .map(str::as_bytes)
            .map(rustybuzz::ttf_parser::Tag::from_bytes_lossy)
            .and_then(rustybuzz::Script::from_iso15924_tag);
        let shaping_plan = match script {
            Some(script) => {
                let language = self
                    .language()
                    .map(|lang| {
                        // rustybuzz's own error here is just "invalid language"
                        // (v0.20.1), so discard it for our own
                        rustybuzz::Language::from_str(lang).map_err(|_| {
                            RustybuzzUnknownLanguageError::new(lang)
                        })
                    })
                    .transpose()?;
                Some(rustybuzz::ShapePlan::new(
                    rusty_face,
                    direction_from_script(script)
                        .unwrap_or(rustybuzz::Direction::LeftToRight),
                    Some(script),
                    language.as_ref(),
                    &[],
                ))
            },
            None => None,
        };
        Ok(shaping_plan)
    }
}

fn direction_from_script(
    script: rustybuzz::Script,
) -> Option<rustybuzz::Direction> {
    // Copied from Rustybuzz
    // https://docs.rs/rustybuzz/0.20.1/src/rustybuzz/hb/common.rs.html#72-154

    match script {
        // Unicode-1.1 additions
        rustybuzz::script::ARABIC |
        rustybuzz::script::HEBREW |

        // Unicode-3.0 additions
        rustybuzz::script::SYRIAC |
        rustybuzz::script::THAANA |

        // Unicode-4.0 additions
        rustybuzz::script::CYPRIOT |

        // Unicode-4.1 additions
        rustybuzz::script::KHAROSHTHI |

        // Unicode-5.0 additions
        rustybuzz::script::PHOENICIAN |
        rustybuzz::script::NKO |

        // Unicode-5.1 additions
        rustybuzz::script::LYDIAN |

        // Unicode-5.2 additions
        rustybuzz::script::AVESTAN |
        rustybuzz::script::IMPERIAL_ARAMAIC |
        rustybuzz::script::INSCRIPTIONAL_PAHLAVI |
        rustybuzz::script::INSCRIPTIONAL_PARTHIAN |
        rustybuzz::script::OLD_SOUTH_ARABIAN |
        rustybuzz::script::OLD_TURKIC |
        rustybuzz::script::SAMARITAN |

        // Unicode-6.0 additions
        rustybuzz::script::MANDAIC |

        // Unicode-6.1 additions
        rustybuzz::script::MEROITIC_CURSIVE |
        rustybuzz::script::MEROITIC_HIEROGLYPHS |

        // Unicode-7.0 additions
        rustybuzz::script::MANICHAEAN |
        rustybuzz::script::MENDE_KIKAKUI |
        rustybuzz::script::NABATAEAN |
        rustybuzz::script::OLD_NORTH_ARABIAN |
        rustybuzz::script::PALMYRENE |
        rustybuzz::script::PSALTER_PAHLAVI |

        // Unicode-8.0 additions
        rustybuzz::script::HATRAN |

        // Unicode-9.0 additions
        rustybuzz::script::ADLAM |

        // Unicode-11.0 additions
        rustybuzz::script::HANIFI_ROHINGYA |
        rustybuzz::script::OLD_SOGDIAN |
        rustybuzz::script::SOGDIAN |

        // Unicode-12.0 additions
        rustybuzz::script::ELYMAIC |

        // Unicode-13.0 additions
        rustybuzz::script::CHORASMIAN |
        rustybuzz::script::YEZIDI |

        // Unicode-14.0 additions
        rustybuzz::script::OLD_UYGHUR => {
            Some(rustybuzz::Direction::RightToLeft)
        }

        // https://github.com/harfbuzz/harfbuzz/issues/1000
        rustybuzz::script::OLD_HUNGARIAN |
        rustybuzz::script::OLD_ITALIC |
        rustybuzz::script::RUNIC |
        rustybuzz::script::TIFINAGH => {
            None
        }

        _ => Some(rustybuzz::Direction::LeftToRight),
    }
}
