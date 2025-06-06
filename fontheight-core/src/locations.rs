use std::{
    collections::{HashMap, HashSet},
    fmt,
};

use skrifa::{raw::collections::int_set::Domain, MetadataProvider};
use thiserror::Error;

/// A mapping of axis names as [`String`]s to values
pub type SimpleLocation = HashMap<String, f32>;

/// A mapping of axis names to values
#[derive(Clone, Default)]
pub struct Location {
    user_coords: HashMap<skrifa::Tag, f32>,
}

impl Location {
    pub(crate) fn new(user_coords: HashMap<skrifa::Tag, f32>) -> Self {
        Self { user_coords }
    }

    /// Set the value of an axis.
    ///
    /// Fails if `tag` isn't a valid axis tag.
    pub fn axis(
        &mut self,
        tag: impl AsRef<[u8]>,
        value: impl Into<f32>,
    ) -> Result<(), skrifa::raw::types::InvalidTag> {
        let tag = skrifa::Tag::new_checked(tag.as_ref())?;
        self.user_coords.insert(tag, value.into());
        Ok(())
    }

    /// Converts a [`SimpleLocation`] to a Font Height `Location`
    ///
    /// Fails if any keys of the [`SimpleLocation`] aren't valid axis tags
    pub fn from_simple(
        location: SimpleLocation,
    ) -> Result<Self, skrifa::raw::types::InvalidTag> {
        Self::try_from(location)
    }

    /// Creates a [`SimpleLocation`] from `&self`.
    pub fn to_simple(&self) -> SimpleLocation {
        self.user_coords
            .iter()
            .map(|(tag, &val)| (tag.to_string(), val))
            .collect()
    }

    /// Creates a [`skrifa::instance::Location`] from `&self`.
    pub(crate) fn to_skrifa(
        &self,
        font: &skrifa::FontRef,
    ) -> skrifa::instance::Location {
        font.axes().location(
            self.user_coords.iter().map(|(tag, coord)| (*tag, *coord)),
        )
    }

    pub(crate) fn to_rustybuzz(&self) -> Vec<rustybuzz::Variation> {
        self.user_coords
            .iter()
            .map(|(tag, coord)| rustybuzz::Variation {
                tag: rustybuzz::ttf_parser::Tag(tag.to_u32()),
                value: *coord,
            })
            .collect()
    }

    /// Checks that `&self` doesn't specify any axes that aren't present in
    /// `font`.
    ///
    /// Omitting axes is allowed as most libraries will just use the default
    /// value if one isn't provided for an axis.
    pub fn validate_for(
        &self,
        font: &skrifa::FontRef,
    ) -> Result<(), MismatchedAxesError> {
        let mut provided =
            self.user_coords.keys().copied().collect::<HashSet<_>>();
        font.axes().iter().map(|axis| axis.tag()).for_each(|tag| {
            provided.remove(&tag);
        });
        let extras = provided;
        if extras.is_empty() {
            Ok(())
        } else {
            Err(MismatchedAxesError {
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

impl TryFrom<SimpleLocation> for Location {
    type Error = skrifa::raw::types::InvalidTag;

    fn try_from(location: SimpleLocation) -> Result<Self, Self::Error> {
        let user_coords = location
            .into_iter()
            .map(|(tag, val)| {
                skrifa::Tag::new_checked(tag.as_bytes()).map(|t| (t, val))
            })
            .collect::<Result<_, _>>()?;
        Ok(Self { user_coords })
    }
}

/// Returned by [`Location::validate_for`], indicating axes are specified in the
/// [`Location`] that aren't in the `font`
#[derive(Debug, Error)]
#[error("mismatched axes: present in Location but not font {extras:?}")]
pub struct MismatchedAxesError {
    extras: Vec<skrifa::Tag>,
}
