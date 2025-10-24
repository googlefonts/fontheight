use std::{
    collections::{HashMap, HashSet},
    fmt,
};

use indexmap::IndexMap;
use ordered_float::NotNan;
use skrifa::MetadataProvider;

use crate::errors::{InvalidTagError, MismatchedAxesError};

// TODO: document NaN panics

/// A mapping of axis tags to values.
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
#[derive(Clone, Default)]
pub struct Location(IndexMap<skrifa::Tag, NotNan<f32>>);

impl Location {
    /// Create a new location.
    #[must_use]
    pub fn new() -> Self {
        // IndexMap::new isn't const so even if we desugared this we couldn't
        // make Location::new const
        Default::default()
    }

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

    /// Creates a [`harfrust::Variation`] from `&self`.
    #[must_use]
    pub fn to_harfrust(&self) -> Vec<harfrust::Variation> {
        self.0
            .iter()
            .map(|(&tag, value)| harfrust::Variation {
                tag,
                value: value.into_inner(),
            })
            .collect()
    }

    /// Checks that `&self` doesn't specify any axes that aren't present in
    /// `font`.
    ///
    /// Omitting axes is allowed as most libraries will just use the default
    /// value if one isn't provided for an axis.
    ///
    /// Note: if you're just using Font Height, it will perform this validation
    /// for you as necessary.
    pub fn validate_for(
        &self,
        font: &skrifa::FontRef,
    ) -> Result<(), MismatchedAxesError> {
        let mut provided = self.0.keys().copied().collect::<HashSet<_>>();
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
            .entries(self.0.iter().map(|(tag, &val)| (tag.to_string(), val)))
            .finish()
    }
}

impl TryFrom<HashMap<String, f32>> for Location {
    type Error = InvalidTagError;

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
