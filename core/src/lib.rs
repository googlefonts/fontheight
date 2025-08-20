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
    borrow::Cow,
    collections::{BTreeSet, HashMap},
    str::FromStr,
};

pub use exemplars::{CollectToExemplars, Exemplars};
use harfrust::{
    Direction, Language, Script, ShapePlan, Shaper, ShaperData, ShaperInstance,
    Tag, UnicodeBuffer, script,
};
use itertools::Itertools;
use kurbo::Shape;
pub use locations::Location;
use ordered_float::OrderedFloat;
use pens::BezierPen;
use skrifa::{
    FontRef, MetadataProvider, instance::Size, outline::DrawSettings,
};
pub use static_lang_word_lists::WordList;
use static_lang_word_lists::WordListIter;

use crate::errors::{
    FontHeightError, HarfRustUnknownLanguageError, ShapingPlanError,
    SkrifaDrawError, SkrifaReadError,
};

pub mod errors;
mod exemplars;
mod locations;
mod pens;

/// Font Height's entrypoint. Parses fonts and can check word lists at
/// specified locations.
pub struct Reporter<'a> {
    font: FontRef<'a>,
    shaper_data: ShaperData,
}

impl<'a> Reporter<'a> {
    /// Parses the byte slice as a font to create a new [`Reporter`].
    ///
    /// Fails if the bytes couldn't be parsed.
    pub fn new(font_bytes: &'a [u8]) -> Result<Self, FontHeightError> {
        let font = FontRef::new(font_bytes).map_err(SkrifaReadError::from)?;
        Ok(Reporter {
            shaper_data: ShaperData::new(&font),
            font,
        })
    }

    /// Access the `read-fonts`-parsed font.
    ///
    /// ⚠️ Warning: changes to the return type of this function (i.e. by
    /// `fontheight` changing [`skrifa`]/[`harfrust`]/`read-fonts` version)
    /// are **not** covered by this crate's efforts to follow SemVer.
    #[inline]
    #[must_use]
    pub const fn fontref(&self) -> &FontRef<'_> {
        &self.font
    }

    /// Gets all combinations of axis coordinates seen in named instances, axis
    /// extremes, and the default location.
    ///
    /// Note: the number of [`Location`]s this method returns scales
    /// exponentially with the number of axes.
    #[must_use]
    pub fn interesting_locations(&self) -> Vec<Location> {
        let mut axis_coords =
            vec![BTreeSet::<OrderedFloat<f32>>::new(); self.font.axes().len()];

        self.font
            .named_instances()
            .iter()
            .flat_map(|instance| instance.user_coords().enumerate())
            .for_each(|(axis, coord)| {
                axis_coords[axis].insert(coord.into());
            });

        self.font.axes().iter().for_each(|axis| {
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
                    .zip(self.font.axes().iter())
                    .map(|(coord, axis)| (axis.tag(), From::from(*coord)))
                    .collect();
                Location::from_skrifa(inner)
            })
            .collect()
    }

    /// Create an [`InstanceReporter`] at a given location.
    ///
    /// Fails if the [`Location`] isn't valid for the font (e.g. specifying axes
    /// that don't exist), or if an error occurs while drawing glyphs.
    ///
    /// Consider your use case:
    /// - checking the default location: use [`Reporter::default_instance`]
    /// - checking all extremes, naming instances, and the default location: use
    ///   [`Reporter::interesting_locations`] and create many
    ///   [`InstanceReporter`]s
    /// - something else specific: create a [`Location`] yourself
    pub fn instance(
        &'a self,
        location: &'a Location,
    ) -> Result<InstanceReporter<'a>, FontHeightError> {
        // Creating InstanceExtremes also validates the Location; do this first
        let instance_extremes = InstanceExtremes::new(&self.font, location)?;
        let shaper_instance =
            ShaperInstance::from_variations(&self.font, location.to_harfrust());

        Ok(InstanceReporter {
            font: &self.font,
            location: Cow::Borrowed(location),
            shaper_data: &self.shaper_data,
            shaper_instance,
            instance_extremes,
        })
    }

    /// Create an [`InstanceReporter`] at the default location.
    ///
    /// Fails if any glyphs in the font can't be drawn.
    pub fn default_instance(
        &'a self,
    ) -> Result<InstanceReporter<'a>, SkrifaDrawError> {
        let location = Cow::<Location>::default();
        let instance_extremes = InstanceExtremes::new(&self.font, &location)
            .map_err(|err| {
                let FontHeightError::Drawing(draw_err) = err else {
                    unreachable!(
                        "InstanceExtremes with a known-good location returned \
                         an error that wasn't a SkrifaDrawError"
                    );
                };
                draw_err
            })?;
        let shaper_instance =
            ShaperInstance::from_variations(&self.font, location.to_harfrust());

        Ok(InstanceReporter {
            font: &self.font,
            location,
            shaper_data: &self.shaper_data,
            shaper_instance,
            instance_extremes,
        })
    }
}

/// A Font Height [`Reporter`] configured to a specific font instance.
///
/// Re-use this if you want to check multiple word-lists at this location.
pub struct InstanceReporter<'a> {
    font: &'a FontRef<'a>,
    location: Cow<'a, Location>,
    shaper_data: &'a ShaperData,
    shaper_instance: ShaperInstance,
    instance_extremes: InstanceExtremes,
}

impl<'a> InstanceReporter<'a> {
    /// Get the [`Location`] that this instance reporter is checking.
    #[inline]
    #[must_use]
    pub fn location(&self) -> &Location {
        self.location.as_ref()
    }

    /// Create an iterator for [`WordExtremes`] with the given [`WordList`].
    ///
    /// Can fail if the [`WordList`]'s metadata is invalid.
    pub fn to_word_extremes_iter(
        &self,
        word_list: &'a WordList,
    ) -> Result<WordExtremesIterator<'_>, ShapingPlanError> {
        let shaper = self
            .shaper_data
            .shaper(self.font)
            .instance(Some(&self.shaper_instance))
            .build();
        let shaping_plan = word_list.shaping_plan(&shaper)?;
        Ok(WordExtremesIterator {
            shaper,
            instance_extremes: &self.instance_extremes,
            shaping_plan,
            word_iter: word_list.iter(),
            unicode_buffer: Some(UnicodeBuffer::new()),
        })
    }

    /// Create a parallel iterator for [`WordExtremes`] at a given location.
    ///
    /// Can fail if the [`WordList`]'s metadata is invalid.
    #[cfg(feature = "rayon")]
    pub fn par_check(
        &'a self,
        word_list: &'a WordList,
        k_words: Option<usize>,
        n_exemplars: usize,
    ) -> Result<Report<'a>, ShapingPlanError> {
        use std::convert::identity;

        use exemplars::ExemplarCollector;
        use rayon::prelude::*;

        struct WorkerState {
            // UnicodeBuffer is transformed into another type during shaping,
            // and then can only be reverted once we've finished
            // analysing the shaped buffer. The Option allows us to
            // take ownership of it during each iteration for these
            // type changes to happen, while still re-using the buffer
            unicode_buffer: Option<UnicodeBuffer>,
        }

        let shaper = self
            .shaper_data
            .shaper(self.font)
            .instance(Some(&self.shaper_instance))
            .build();
        let shaping_plan = word_list.shaping_plan(&shaper)?;

        let exemplars = word_list
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
                    // Default features are still included by default
                    let glyph_buffer = match &shaping_plan {
                        Some(plan) => shaper.shape_with_plan(plan, buffer, &[]),
                        None => shaper.shape(buffer, &[]),
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
                            let heights = self
                                .instance_extremes
                                .get(info.glyph_id)
                                .unwrap();

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
            )
            .build();

        Ok(Report {
            location: self.location.as_ref(),
            word_list,
            exemplars,
        })
    }
}

/// An iterator of [`WordExtremes`] for one specific font, [`WordList`], and
/// [`Location`].
///
/// Produced by a [`InstanceReporter`].
pub struct WordExtremesIterator<'a> {
    shaper: Shaper<'a>,
    instance_extremes: &'a InstanceExtremes,
    shaping_plan: Option<ShapePlan>,
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
            // Default features are still included by default
            let glyph_buffer = match &self.shaping_plan {
                Some(plan) => self.shaper.shape_with_plan(plan, buffer, &[]),
                None => self.shaper.shape(buffer, &[]),
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
        font: &FontRef,
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
///
/// Vertical extremes are measured in font units.
#[derive(Debug, Clone, Copy, Default, Eq, PartialEq, Hash)]
pub struct VerticalExtremes {
    lowest: OrderedFloat<f64>,
    highest: OrderedFloat<f64>,
}

impl VerticalExtremes {
    /// The lowest/smaller extreme, in font units.
    #[inline]
    #[must_use]
    pub fn lowest(&self) -> f64 {
        *self.lowest
    }

    /// The highest/bigger extreme, in font units.
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
    /// This will always be the full word list, even if only part of it was
    /// tested.
    pub word_list: &'a WordList,
    /// The highest & lowest-reaching words shaped.
    pub exemplars: Exemplars<'a>,
}

impl<'a> Report<'a> {
    /// Create a new report from its fields.
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
        shaper: &Shaper,
    ) -> Result<Option<ShapePlan>, ShapingPlanError>;
}

impl WordListExt for WordList {
    fn shaping_plan(
        &self,
        shaper: &Shaper,
    ) -> Result<Option<ShapePlan>, ShapingPlanError> {
        let script = self
            .script()
            .map(Tag::from_str)
            .transpose()
            .map_err(|inner| ShapingPlanError::UnknownScriptTag {
                word_list_name: self.name().to_owned(),
                inner: inner.into(),
            })?
            .and_then(Script::from_iso15924_tag);
        let shaping_plan = match script {
            Some(script) => {
                let language = self
                    .language()
                    .map(|lang| {
                        // harfrust's own error here is just "invalid language"
                        // (v0.1.1), so discard it for our own
                        Language::from_str(lang).map_err(|_| {
                            ShapingPlanError::UnknownLanguage {
                                word_list_name: self.name().to_owned(),
                                inner: HarfRustUnknownLanguageError::new(lang),
                            }
                        })
                    })
                    .transpose()?;
                Some(ShapePlan::new(
                    shaper,
                    direction_from_script(script)
                        .unwrap_or(Direction::LeftToRight),
                    Some(script),
                    language.as_ref(),
                    // Default features are still included by default
                    &[],
                ))
            },
            None => None,
        };
        Ok(shaping_plan)
    }
}

const fn direction_from_script(script: Script) -> Option<Direction> {
    // Copied from harfrust (internal API)
    // https://github.com/harfbuzz/harfrust/blob/bf4b7ca20cf95e7183c5f9e1c13a56e9ca6c1174/src/hb/common.rs#L75-L161

    match script {
        // Unicode-1.1 additions
        script::ARABIC |
        script::HEBREW |

        // Unicode-3.0 additions
        script::SYRIAC |
        script::THAANA |

        // Unicode-4.0 additions
        script::CYPRIOT |

        // Unicode-4.1 additions
        script::KHAROSHTHI |

        // Unicode-5.0 additions
        script::PHOENICIAN |
        script::NKO |

        // Unicode-5.1 additions
        script::LYDIAN |

        // Unicode-5.2 additions
        script::AVESTAN |
        script::IMPERIAL_ARAMAIC |
        script::INSCRIPTIONAL_PAHLAVI |
        script::INSCRIPTIONAL_PARTHIAN |
        script::OLD_SOUTH_ARABIAN |
        script::OLD_TURKIC |
        script::SAMARITAN |

        // Unicode-6.0 additions
        script::MANDAIC |

        // Unicode-6.1 additions
        script::MEROITIC_CURSIVE |
        script::MEROITIC_HIEROGLYPHS |

        // Unicode-7.0 additions
        script::MANICHAEAN |
        script::MENDE_KIKAKUI |
        script::NABATAEAN |
        script::OLD_NORTH_ARABIAN |
        script::PALMYRENE |
        script::PSALTER_PAHLAVI |

        // Unicode-8.0 additions
        script::HATRAN |

        // Unicode-9.0 additions
        script::ADLAM |

        // Unicode-11.0 additions
        script::HANIFI_ROHINGYA |
        script::OLD_SOGDIAN |
        script::SOGDIAN |

        // Unicode-12.0 additions
        script::ELYMAIC |

        // Unicode-13.0 additions
        script::CHORASMIAN |
        script::YEZIDI |

        // Unicode-14.0 additions
        script::OLD_UYGHUR => {
            Some(Direction::RightToLeft)
        }

        // https://github.com/harfbuzz/harfbuzz/issues/1000
        script::OLD_HUNGARIAN |
        script::OLD_ITALIC |
        script::RUNIC |
        script::TIFINAGH => {
            None
        }

        _ => Some(Direction::LeftToRight),
    }
}
