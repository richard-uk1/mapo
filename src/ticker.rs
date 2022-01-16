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
    /// How many ticks are we going to draw.
    fn len(&self, axis_len: f64) -> usize;

    /// Get the `idx`th tick
    ///
    /// This should return `Some` if `idx < Ticker::len(self)`, `None` otherwise.
    fn get(&self, axis_len: f64, idx: usize) -> Option<Tick>;

    fn ticks(&self, axis_len: f64) -> Box<dyn Iterator<Item = Tick> + '_> {
        let range = 0..Ticker::len(self, axis_len);
        Box::new(range.map(|idx| Ticker::get(self, axis_len, idx)))
    }
}
