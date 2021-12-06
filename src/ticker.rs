use crate::{ArcStr, Range, Sequence};
use std::fmt;

/// A position on an axis that we can mark and label.
pub struct Tick {
    /// The distance along the axis that the tick should be displayed, in `0..=axis_len`
    pub pos: f64,
    /// The label that should be displayed
    pub label: ArcStr,
}

pub trait Ticker {
    type TickIter<'a>: Iterator<Item = Tick>
    where
        Self: 'a;

    /// How many ticks are we going to draw.
    fn len(&self, axis_len: f64) -> usize;

    /// Returns the set of ticks for this scale.
    fn ticks(&self, axis_len: f64) -> Self::TickIter<'_>;
}

impl Ticker for Range {
    type TickIter<'a>
    where
        Self: 'a,
    = impl Iterator<Item = Tick>;

    fn len(&self, axis_len: f64) -> usize {
        todo!()
    }

    fn ticks(&self, axis_len: f64) -> Self::TickIter<'_> {
        std::iter::empty()
    }
}

impl<S> Ticker for S
where
    S: Sequence,
    for<'a> S::Item<'a>: fmt::Display,
{
    type TickIter<'a>
    where
        Self: 'a,
    = impl Iterator<Item = Tick>;

    fn len(&self, _axis_len: f64) -> usize {
        Sequence::len(self)
    }

    fn ticks(&self, axis_len: f64) -> Self::TickIter<'_> {
        // gap between labels. We'll get NaN when self.len() == 0, but it doesn't matter
        // because the iterator will be empty
        let gap = axis_len / (self.len() as f64 - 1.);
        self.iter().enumerate().map(move |(idx, v)| Tick {
            pos: idx as f64 * gap,
            label: v.to_string().into(),
        })
    }
}
