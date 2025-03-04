use std::collections::{BTreeSet, HashMap};

use itertools::Itertools;
use ordered_float::OrderedFloat;
use skrifa::{raw::collections::int_set::Domain, MetadataProvider};

pub type SimpleLocation = HashMap<String, f32>;

#[derive(Debug, Clone)]
pub struct Location {
    user_coords: HashMap<skrifa::Tag, f32>,
}

impl Location {
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
