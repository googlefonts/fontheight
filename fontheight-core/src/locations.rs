use std::{
    collections::{BTreeSet, HashMap, HashSet},
    fmt,
};

// Re-exported as it appears in public API of Location::axis
pub use font_types::InvalidTag;
use itertools::Itertools;
use ordered_float::OrderedFloat;
use skrifa::{raw::collections::int_set::Domain, MetadataProvider};
use thiserror::Error;

use crate::FontHeightError;

pub type SimpleLocation = HashMap<String, f32>;

#[derive(Clone, Default)]
pub struct Location {
    user_coords: HashMap<skrifa::Tag, f32>,
}

impl Location {
    pub fn new(user_coords: HashMap<skrifa::Tag, f32>) -> Self {
        Self { user_coords }
    }

    pub fn axis(
        &mut self,
        tag: impl AsRef<[u8]>,
        value: impl Into<f32>,
    ) -> Result<(), InvalidTag> {
        let tag = skrifa::Tag::new_checked(tag.as_ref())?;
        self.user_coords.insert(tag, value.into());
        Ok(())
    }

    pub fn from_simple(
        location: SimpleLocation,
    ) -> Result<Self, FontHeightError> {
        let user_coords = location
            .into_iter()
            .map(|(tag, val)| {
                skrifa::Tag::new_checked(tag.as_bytes()).map(|t| (t, val))
            })
            .collect::<Result<_, _>>()?;
        Ok(Self { user_coords })
    }

    pub fn to_simple(&self) -> SimpleLocation {
        self.user_coords
            .iter()
            .map(|(tag, &val)| (tag.to_string(), val))
            .collect()
    }

    pub fn to_skrifa(
        &self,
        font: &skrifa::FontRef,
    ) -> skrifa::instance::Location {
        font.axes().location(
            self.user_coords.iter().map(|(tag, coord)| (*tag, *coord)),
        )
    }

    pub fn to_rustybuzz(&self) -> Vec<rustybuzz::Variation> {
        self.user_coords
            .iter()
            .map(|(tag, coord)| rustybuzz::Variation {
                tag: rustybuzz::ttf_parser::Tag(tag.to_u32()),
                value: *coord,
            })
            .collect()
    }

    pub fn validate_for(
        &self,
        font: &skrifa::FontRef,
    ) -> Result<(), MismatchedAxesError> {
        let mut extras =
            self.user_coords.keys().copied().collect::<HashSet<_>>();
        let missing = font
            .axes()
            .iter()
            .map(|axis| axis.tag())
            .filter(|tag| !extras.remove(tag))
            .collect::<Vec<_>>();
        if extras.is_empty() && missing.is_empty() {
            Ok(())
        } else {
            Err(MismatchedAxesError {
                missing,
                extras: Vec::from_iter(extras),
            })
        }
    }
}

impl fmt::Debug for Location {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_map()
            .entries(
                self.user_coords
                    .iter()
                    .map(|(tag, &val)| (tag.to_string(), val)),
            )
            .finish()
    }
}

#[derive(Debug, Error)]
pub struct MismatchedAxesError {
    extras: Vec<skrifa::Tag>,
    missing: Vec<skrifa::Tag>,
}

impl fmt::Display for MismatchedAxesError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("mismatched axes: ")?;
        match (self.extras.is_empty(), self.missing.is_empty()) {
            (false, false) => write!(
                f,
                "in font but not Location {:?}; in Location but not font {:?}",
                self.missing, self.extras,
            ),
            (true, false) => {
                write!(f, "in font but not Location {:?}", self.missing)
            },
            (false, true) => {
                write!(f, "in Location but not font {:?}", self.extras)
            },
            (true, true) => unreachable!(
                "MismatchedAxesError constructed with two empty lists"
            ),
        }
    }
}

/// Gets the cartesian product of axis coordinates seen in named instances, axis
/// extremes, and defaults.
pub(crate) fn interesting_locations(font: &skrifa::FontRef) -> Vec<Location> {
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
        .map(|coords| Location {
            user_coords: coords
                .into_iter()
                .zip(font.axes().iter())
                .map(|(coord, axis)| (axis.tag(), From::from(*coord)))
                .collect(),
        })
        .collect()
}
