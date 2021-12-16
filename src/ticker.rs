use crate::ArcStr;
use std::fmt;

/// A position on an axis that we can mark and label.
#[derive(Debug)]
pub struct Tick {
    /// The distance along the axis that the tick should be displayed, in `0..=axis_len`
    pub pos: f64,
    /// The label that should be displayed
    pub label: ArcStr,
}

/// Object that implement can take an axis length (in device-independent pixels) and return a set
/// of marks (a.k.a. ticks) that mark significant points along the axis.
pub trait Ticker: fmt::Debug {
    type TickIter: Iterator<Item = Tick>;

    /// How many ticks are we going to draw.
    fn len(&self, axis_len: f64) -> usize;

    /// Returns the set of ticks for this scale.
    fn ticks(&self, axis_len: f64) -> Self::TickIter;

    /// Get the `idx`th tick
    fn get(&self, axis_len: f64, idx: usize) -> Option<Tick> {
        self.ticks(axis_len).nth(idx)
    }
}
