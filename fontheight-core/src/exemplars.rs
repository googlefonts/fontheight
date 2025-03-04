use std::{cmp::Ordering, collections::BinaryHeap};

use crate::WordExtremes;

/// A summary of the lowest lows and highest highs.
#[derive(Debug, Clone)]
pub struct Exemplars<'w> {
    pub lowest: Vec<WordExtremes<'w>>,
    pub highest: Vec<WordExtremes<'w>>,
}

/// A builder to construct a limited size summary from a stream of words. We do
/// this as an explicit step with a binary heap for assured runtime complexity.
#[derive(Debug, Clone)]
pub(crate) struct ExemplarCollector<'w> {
    lowest: BinaryHeap<ByLowest<'w>>,
    highest: BinaryHeap<ByHighest<'w>>,
}

/// Report, but sorted ascending by lowest for a max-heap.
#[derive(Debug, Copy, Clone, Eq, PartialEq)]
struct ByLowest<'w>(WordExtremes<'w>);

/// Report, but sorted descending by highest for a min-heap.
#[derive(Debug, Copy, Clone, Eq, PartialEq)]
struct ByHighest<'w>(WordExtremes<'w>);

impl Ord for ByLowest<'_> {
    fn cmp(&self, other: &Self) -> Ordering {
        self.0.extremes.lowest.cmp(&other.0.extremes.lowest)
    }
}

impl Ord for ByHighest<'_> {
    fn cmp(&self, other: &Self) -> Ordering {
        self.0
            .extremes
            .highest
            .cmp(&other.0.extremes.highest)
            .reverse()
    }
}

impl PartialOrd for ByLowest<'_> {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        // Defer to Ord.
        Some(self.cmp(other))
    }
}

impl PartialOrd for ByHighest<'_> {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        // Defer to Ord.
        Some(self.cmp(other))
    }
}

impl<'w> ExemplarCollector<'w> {
    /// Build a summary with a maximum number of reports.
    pub(crate) fn new(top_n: usize) -> Self {
        Self {
            lowest: BinaryHeap::with_capacity(top_n),
            highest: BinaryHeap::with_capacity(top_n),
        }
    }

    /// Consider adding a new report, if it is low and/or high enough.
    pub(crate) fn push(&mut self, elem: WordExtremes<'w>) {
        // Store if this report is stronger than the weakest report that would
        // be evicted from the heap to accommodate it.
        let by_lowest = ByLowest(elem);
        if self.lowest.len() < self.lowest.capacity() {
            self.lowest.push(by_lowest);
        } else if let Some(mut weakest_low) = self.lowest.peek_mut() {
            if by_lowest < *weakest_low {
                *weakest_low = by_lowest;
            }
        }

        // Store if this report is stronger than the weakest report that would
        // be evicted from the heap to accommodate it.
        let by_highest = ByHighest(elem);
        if self.highest.len() < self.highest.capacity() {
            self.highest.push(by_highest);
        } else if let Some(mut weakest_high) = self.highest.peek_mut() {
            if by_highest < *weakest_high {
                *weakest_high = by_highest;
            }
        }
    }

    /// Consume this builder and produce the summary.
    pub(crate) fn build(self) -> Exemplars<'w> {
        Exemplars {
            lowest: self
                .lowest
                .into_sorted_vec()
                .into_iter()
                .map(|by| by.0)
                .collect(),
            highest: self
                .highest
                .into_sorted_vec()
                .into_iter()
                .map(|by| by.0)
                .collect(),
        }
    }
}
