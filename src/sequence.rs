use crate::ticker::{Tick, Ticker};
use crate::Range;
use std::{borrow::Cow, fmt, mem, sync::Arc};

/// Because we layout all labels, we should have some cap for when there are so many it will affect perf.
/// The number should be high enough that you couldn't possibly want more.
const MAX_LABELS: usize = 100;

/// The discrete analogue of `Ranged`.
pub trait Sequence {
    type Item<'a>
    where
        Self: 'a;
    type Iter<'a>: Iterator<Item = Self::Item<'a>> + 'a
    where
        Self: 'a;

    /// The number of items in the sequence.
    fn len(&self) -> usize;

    /// The value of the sequence at `idx`.
    ///
    /// This function should always return `Some` if `idx` < `self.len()`
    fn get(&self, idx: usize) -> Option<Self::Item<'_>>;

    /// Returns an iterator over the values.
    fn iter(&self) -> Self::Iter<'_>;
}

/// A numeric sequence
#[derive(Copy, Clone)]
pub struct Numeric {
    range: Range,
    step: f64,
}

impl Numeric {
    /// Construct a numeric sequence.
    ///
    /// # Panics
    ///
    /// Panics if `0 < step < ∞` is not true.
    pub fn from_range_step(range: Range, step: f64) -> Self {
        assert!(step.is_finite() && step > 0., "0 < {} < ∞", step);
        Self { range, step }
    }

    /// Construct a numeric sequence.
    ///
    /// # Panics
    ///
    /// Panics if `0 < step < ∞` or `-∞ < min <= max < ∞` is not true.
    pub fn new(min: f64, max: f64, step: f64) -> Self {
        Self::from_range_step(Range::new(min, max), step)
    }

    /// Get the range for this sequence.
    pub fn range(&self) -> Range {
        self.range
    }

    /// Get the minimum value for this sequence.
    pub fn min(&self) -> f64 {
        self.range.min()
    }

    /// Get the maximum value for this sequence.
    ///
    /// Note that this might be different from `self.range().max()` if `self.range()`s length is
    /// not an exact multiple of `self.step()`.
    pub fn max(&self) -> f64 {
        let last_idx = (self.range.width() / self.step).floor();
        self.range.min() + self.step * last_idx
    }

    /// Get the step value for this sequence.
    pub fn step(&self) -> f64 {
        self.step
    }
}

impl Sequence for Numeric {
    type Item<'a> = f64;
    type Iter<'a> = impl Iterator<Item = Self::Item<'a>> + 'a;

    fn len(&self) -> usize {
        // ignoring overflow for now
        ((self.range.max() - self.range.min()) / self.step).floor() as usize
    }

    fn get(&self, idx: usize) -> Option<Self::Item<'_>> {
        let val = self.range.min() + idx as f64 * self.step;
        if val > self.range.max() {
            None
        } else {
            Some(val)
        }
    }

    fn iter(&self) -> Self::Iter<'_> {
        NumericIter::new(*self)
    }
}

struct NumericIter {
    inner: Numeric,
    next: f64,
}

impl NumericIter {
    fn new(inner: Numeric) -> Self {
        NumericIter {
            inner,
            next: inner.range.min(),
        }
    }
}

impl Iterator for NumericIter {
    type Item = f64;

    fn next(&mut self) -> Option<Self::Item> {
        if self.next > self.inner.range.max() {
            return None;
        }
        let out = self.next;
        self.next += self.inner.step;
        Some(out)
    }
}

#[derive(Debug, Clone)]
pub struct Categorical<T> {
    categories: Arc<[T]>,
}

impl<T> Categorical<T> {
    pub fn new(categories: impl Into<Arc<[T]>>) -> Self {
        Categorical {
            categories: categories.into(),
        }
    }

    /// Get the categories
    pub fn categories(&self) -> &[T] {
        &self.categories[..]
    }

    /// Sets the categories. Returns old value.
    pub fn set_categories(&mut self, categories: impl Into<Arc<[T]>>) -> &mut Self {
        self.categories = categories.into();
        self
    }
}

impl<T> Sequence for Categorical<T> {
    type Item<'a>
    where
        T: 'a,
    = &'a T;
    type Iter<'a>
    where
        T: 'a,
    = impl Iterator<Item = Self::Item<'a>> + 'a;

    fn len(&self) -> usize {
        self.categories.len()
    }

    fn get(&self, idx: usize) -> Option<Self::Item<'_>> {
        self.categories.get(idx)
    }

    fn iter(&self) -> Self::Iter<'_> {
        self.categories.iter()
    }
}

impl<T> From<Arc<[T]>> for Categorical<T> {
    fn from(f: Arc<[T]>) -> Self {
        Self::new(f)
    }
}

impl<T> From<Vec<T>> for Categorical<T> {
    fn from(f: Vec<T>) -> Self {
        Self::new(Arc::from(f))
    }
}

impl<T> From<Box<[T]>> for Categorical<T> {
    fn from(f: Box<[T]>) -> Self {
        Self::new(Arc::from(f))
    }
}

impl<T: Copy, const N: usize> From<[T; N]> for Categorical<T> {
    fn from(f: [T; N]) -> Self {
        let boxed_slice = Box::<[T]>::from(f.as_ref());
        Self::new(Arc::from(boxed_slice))
    }
}

/// Extension methods for `Sequence`.
pub trait SequenceExt: Sequence {
    /// Use when you want flex 'space-around' behavior (as opposed to 'space-bewteen' - the
    /// default).
    fn space_around(self) -> SpaceAround<Self>
    where
        Self: Sized;
}

impl<S: Sequence> SequenceExt for S {
    fn space_around(self) -> SpaceAround<Self> {
        SpaceAround(self)
    }
}

pub struct SpaceAround<S>(S);

impl<S> Ticker for SpaceAround<S>
where
    S: Sequence,
    for<'a> S::Item<'a>: fmt::Display,
{
    type TickIter<'a>
    where
        Self: 'a,
    = impl Iterator<Item = Tick>;

    fn len(&self, _axis_len: f64) -> usize {
        self.0.len()
    }

    fn ticks(&self, axis_len: f64) -> Self::TickIter<'_> {
        // gap between labels. We'll get NaN when self.len() == 0, but it doesn't matter
        // because the iterator will be empty
        let gap = axis_len / self.0.len() as f64;
        self.0.iter().enumerate().map(move |(idx, v)| Tick {
            pos: (idx as f64 + 0.5) * gap,
            label: v.to_string().into(),
        })
    }
}
