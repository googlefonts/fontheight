use std::{
    cmp::Ordering,
    collections::{HashMap, HashSet},
    fmt,
};

use indexmap::IndexMap;
use ordered_float::NotNan;
use skrifa::MetadataProvider;

use crate::errors::{InvalidTagError, MismatchedAxesError};

/// A mapping of axis tags to values.
///
/// Retains insertion order of axes.
///
/// ```
/// # use harfshapedfa::Location;
/// # use harfshapedfa::errors::InvalidTagError;
/// # fn main() -> Result<(), InvalidTagError> {
/// let mut loc = Location::new();
/// loc.axis("wght", 400.0)?
///     .axis("ital", 1.0)?
///     .axis("wdth", 1000.0)?;
/// # Ok(())
/// # }
/// ```
#[derive(Clone, Default, Eq, PartialEq)]
pub struct Location(IndexMap<skrifa::Tag, NotNan<f32>>);

impl Location {
    /// Create a new location.
    #[must_use]
    pub fn new() -> Self {
        // IndexMap::new isn't const so even if we desugared this we couldn't
        // make Location::new const
        Default::default()
    }

    /// Convert from a [`HashMap`] using [`skrifa::Tag`]s as keys.
    ///
    /// # Panics
    ///
    /// If any axis values are `NaN`.
    #[must_use]
    pub fn from_skrifa(user_coords: HashMap<skrifa::Tag, f32>) -> Self {
        Self(
            user_coords
                .into_iter()
                .map(|(tag, value)| {
                    let value = NotNan::new(value).unwrap_or_else(|_| {
                        panic!("{tag} coordinate was NaN");
                    });
                    (tag, value)
                })
                .collect(),
        )
    }

    /// Set the value of an axis.
    ///
    /// Fails if `tag` isn't a valid axis tag.
    ///
    /// Designed to support method chaining:
    ///
    /// ```
    /// # use harfshapedfa::Location;
    /// # use harfshapedfa::errors::InvalidTagError;
    /// # fn main() -> Result<(), InvalidTagError> {
    /// let mut loc = Location::new();
    /// loc.axis("wght", 400.0)?
    ///     .axis("ital", 1.0)?
    ///     .axis("wdth", 1000.0)?;
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// # Panics
    ///
    /// If any axis value is `NaN`.
    pub fn axis(
        &mut self,
        tag: impl AsRef<[u8]>,
        value: f32,
    ) -> Result<&mut Self, InvalidTagError> {
        let tag = skrifa::Tag::new_checked(tag.as_ref())?;
        let value = NotNan::new(value).unwrap_or_else(|_| {
            panic!("{tag} coordinate was NaN");
        });
        self.0.insert(tag, value);
        Ok(self)
    }

    /// Converts a [`HashMap`] to a Font Height [`Location`].
    ///
    /// Fails if any keys aren't valid axis tags.
    ///
    /// Note: this is just an alias to the [`TryFrom`] implementation.
    ///
    /// # Panics
    ///
    /// If any axis values are `NaN`.
    // TODO: I think this one should error, not panic, on NaNs
    pub fn try_from_std(
        location: HashMap<String, f32>,
    ) -> Result<Self, InvalidTagError> {
        Self::try_from(location)
    }

    /// Creates a [`HashMap<String, f32>`](HashMap) from `&self`.
    #[must_use]
    pub fn to_std(&self) -> HashMap<String, f32> {
        self.0
            .iter()
            .map(|(tag, val)| (tag.to_string(), val.into_inner()))
            .collect()
    }

    /// Creates a [`skrifa::instance::Location`] from `&self`.
    #[must_use]
    pub fn to_skrifa(
        &self,
        font: &skrifa::FontRef,
    ) -> skrifa::instance::Location {
        font.axes().location(
            self.0.iter().map(|(tag, coord)| (*tag, coord.into_inner())),
        )
    }

    /// Creates a [`harfrust::Variation`] iterator from `&self`.
    pub fn to_harfrust(&self) -> impl Iterator<Item = harfrust::Variation> {
        self.0.iter().map(|(&tag, value)| harfrust::Variation {
            tag,
            value: value.into_inner(),
        })
    }

    /// Checks that `&self` doesn't specify any axes that aren't present in
    /// `font`.
    ///
    /// Omitting axes is allowed as most libraries will just use the default
    /// value if one isn't provided for an axis.
    ///
    /// ⚠️ Does not current check axis values are valid / in range.
    ///
    /// Note: if you're just using Font Height, it will perform this validation
    /// for you as necessary.
    pub fn validate_for(
        &self,
        font: &skrifa::FontRef,
    ) -> Result<(), MismatchedAxesError> {
        let mut provided = self.0.keys().copied().collect::<HashSet<_>>();
        // TODO: check values are legal too
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

    /// Sort axes lexicographically.
    ///
    /// Axes being ordered allows for [sorting](Location::partial_cmp).
    pub fn sort_axes(&mut self) {
        self.0.sort_keys();
    }

    // TODO
    // pub fn sort_axes_by(&mut self, func)
    // pub fn sort_axes_with_fvar(&mut self, font)
    // pub fn sort_axes_with_stat(&mut self, font)
}

impl PartialOrd for Location {
    /// Sorts two `Location`s iff they have the same axes in the same order.
    /// Will return `None` if this is not the case.
    // FIXME: will return None for some Locations that are considered equal
    //        (when axis order differs). Does this violate expected
    //        invariants of PartialOrd/Eq?
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        if self.0.len() != other.0.len() {
            // Difference in axes
            return None;
        }

        for ((left_tag, left_val), (right_tag, right_val)) in
            self.0.iter().zip(other.0.iter())
        {
            if left_tag != right_tag {
                // Difference in axes (order)
                return None;
            }
            match NotNan::cmp(left_val, right_val) {
                Ordering::Equal => { /* check next axis */ },
                not_equal => return Some(not_equal),
            }
        }
        Some(Ordering::Equal)
    }
}

impl fmt::Debug for Location {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_map()
            .entries(self.0.iter().map(|(tag, &val)| (tag.to_string(), val)))
            .finish()
    }
}

impl<T> FromIterator<(T, f32)> for Location
where
    T: AsRef<[u8]>,
{
    /// Support collecting into a `Location`.
    // TODO: code example
    ///
    /// # Panics
    ///
    /// If any axis value is `NaN`.
    fn from_iter<I: IntoIterator<Item = (T, f32)>>(iter: I) -> Self {
        iter.into_iter()
            .fold(Location::new(), |mut loc, (tag, value)| {
                loc.axis(tag, value)
                    .expect("invalid tag when building Location");
                loc
            })
    }
}

impl TryFrom<HashMap<String, f32>> for Location {
    type Error = InvalidTagError;

    /// Convert standard library types into a `Location`.
    ///
    /// # Panics
    ///
    /// If any value of `location` is `NaN`.
    // TODO: make NaNs an error
    fn try_from(location: HashMap<String, f32>) -> Result<Self, Self::Error> {
        let user_coords = location
            .into_iter()
            .map(|(tag, value)| -> Result<_, InvalidTagError> {
                let tag = skrifa::Tag::new_checked(tag.as_bytes())?;
                let value = NotNan::new(value).unwrap_or_else(|_| {
                    panic!("{tag} coordinate was NaN");
                });
                Ok((tag, value))
            })
            .collect::<Result<_, _>>()?;
        Ok(Self(user_coords))
    }
}
