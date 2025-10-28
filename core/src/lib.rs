#![cfg_attr(docsrs, feature(doc_cfg))]
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
    cmp,
    collections::{BTreeSet, HashMap},
};

pub use exemplars::{CollectToExemplars, Exemplars};
use harfrust::{Shaper, ShaperData, ShaperInstance, UnicodeBuffer};
pub use harfshapedfa::Location;
use harfshapedfa::{HarfRustShaperExt, ShapingMeta, pens::BoundsPen};
use itertools::Itertools;
use ordered_float::{NotNan, OrderedFloat};
use skrifa::{
    FontRef, MetadataProvider, instance::Size, outline::DrawSettings,
};
pub use static_lang_word_lists::WordList;
use static_lang_word_lists::WordListIter;

use crate::errors::{
    FontHeightError, SkrifaDrawError, SkrifaReadError, WordListShapingPlanError,
};

pub mod errors;
mod exemplars;

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
        // Note: this could probably be a NotNan<f32>, but then all the .into
        // calls become exceedingly painful
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
                let mut loc = Location::from_skrifa(inner);
                loc.sort_axes();
                loc
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
    ) -> Result<WordExtremesIterator<'_>, WordListShapingPlanError> {
        let shaper = self
            .shaper_data
            .shaper(self.font)
            .instance(Some(&self.shaper_instance))
            .build();
        let shaping_meta = word_list
            .script()
            .map(|script| {
                ShapingMeta::new(script, word_list.language(), &shaper)
            })
            .transpose()
            .map_err(|err| WordListShapingPlanError {
                word_list_name: word_list.name().to_owned(),
                inner: err,
            })?;
        Ok(WordExtremesIterator {
            shaper,
            instance_extremes: &self.instance_extremes,
            shaping_meta,
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
    ) -> Result<Report<'a>, WordListShapingPlanError> {
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
        let shaping_meta = word_list
            .script()
            .map(|script| {
                ShapingMeta::new(script, word_list.language(), &shaper)
            })
            .transpose()
            .map_err(|err| WordListShapingPlanError {
                word_list_name: word_list.name().to_owned(),
                inner: err,
            })?;

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

                    // Default features are still included by default
                    let glyph_buffer = match &shaping_meta {
                        Some(meta) => shaper.shape_with_meta(meta, buffer, &[]),
                        None => {
                            buffer.guess_segment_properties();
                            shaper.shape(buffer, &[])
                        },
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
                            let y_offset = NotNan::new(pos.y_offset as f64)
                                .expect("NaN y offset");
                            let heights = self
                                .instance_extremes
                                .get(info.glyph_id)
                                .unwrap();

                            VerticalExtremes {
                                lowest: heights.lowest + y_offset,
                                highest: heights.highest + y_offset,
                            }
                        })
                        .reduce(VerticalExtremes::merge)
                        .unwrap_or_default();

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
    shaping_meta: Option<ShapingMeta>,
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

            // Default features are still included by default
            let glyph_buffer = match &self.shaping_meta {
                Some(meta) => self.shaper.shape_with_meta(meta, buffer, &[]),
                None => {
                    buffer.guess_segment_properties();
                    self.shaper.shape(buffer, &[])
                },
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
                let y_offset =
                    NotNan::new(pos.y_offset as f64).expect("NaN y offset");
                let heights =
                    self.instance_extremes.get(info.glyph_id).unwrap();

                VerticalExtremes {
                    lowest: heights.lowest + y_offset,
                    highest: heights.highest + y_offset,
                }
            })
            .reduce(VerticalExtremes::merge)
            .unwrap_or_default();

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

impl WordExtremes<'_> {
    /// The lowest/smaller extreme, in font units.
    ///
    /// Sugar for [`VerticalExtremes::lowest`].
    #[inline]
    #[must_use]
    pub fn lowest(&self) -> f64 {
        self.extremes.lowest()
    }

    /// The highest/bigger extreme, in font units.
    ///
    /// Sugar for [`VerticalExtremes::highest`].
    #[inline]
    #[must_use]
    pub fn highest(&self) -> f64 {
        self.extremes.highest()
    }

    /// Get the `WordExtremes` that reaches the lowest.
    #[inline]
    #[must_use]
    pub fn lower(self, other: Self) -> Self {
        if self.extremes.lowest <= other.extremes.lowest {
            self
        } else {
            other
        }
    }

    /// Get the `WordExtremes` that reaches the highest.
    #[inline]
    #[must_use]
    pub fn higher(self, other: Self) -> Self {
        if self.extremes.highest >= other.extremes.highest {
            self
        } else {
            other
        }
    }
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
                let mut bounds_pen = BoundsPen::new();
                outline
                    .draw(
                        DrawSettings::unhinted(
                            Size::unscaled(),
                            &location.to_skrifa(font),
                        ),
                        &mut bounds_pen,
                    )
                    .map_err(|err| SkrifaDrawError(id, err))?;

                let harfshapedfa::pens::Rect { y0, y1, .. } = bounds_pen.bounding_box();
                Ok((u32::from(id), VerticalExtremes {
                    lowest: NotNan::new(y0).expect("bounding box with NaN y0"),
                    highest: NotNan::new(y1).expect("bounding box with NaN y1"),
                }))
            })
            .collect::<Result<HashMap<_, _>, _>>()?;
        Ok(InstanceExtremes(instance_extremes))
    }

    /// Get the [`VerticalExtremes`] for the given glyph ID.
    #[must_use]
    pub fn get(&self, glyph_id: u32) -> Option<VerticalExtremes> {
        self.0.get(&glyph_id).copied()
    }
}

/// The highest & lowest point on the vertical (y) axis.
///
/// Vertical extremes are measured in font units.
#[derive(Debug, Clone, Copy, Default, Eq, PartialEq, Hash)]
pub struct VerticalExtremes {
    lowest: NotNan<f64>,
    highest: NotNan<f64>,
}

impl VerticalExtremes {
    /// Create a new `VerticalExtremes` from the two provided values.
    ///
    /// Panics if either value is `NaN`, or if `lowest > highest`.
    #[inline]
    #[must_use]
    pub fn new(lowest: f64, highest: f64) -> Self {
        let lowest = NotNan::new(lowest).expect("lowest was NaN");
        let highest = NotNan::new(highest).expect("highest was NaN");
        assert!(
            lowest <= highest,
            "lowest value was greater than highest value"
        );
        Self { lowest, highest }
    }

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

    /// Combine two `VerticalExtremes`, taking the higher `highest` value, and
    /// lower `lowest` value.
    #[inline]
    #[must_use]
    pub fn merge(self, other: Self) -> Self {
        Self {
            lowest: cmp::min(self.lowest, other.lowest),
            highest: cmp::max(self.highest, other.highest),
        }
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
