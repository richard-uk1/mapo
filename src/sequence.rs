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
pub struct Categorical<'a, T: Clone> {
    categories: Cow<'a, [T]>,
}

impl<'a, T: Clone> Categorical<'a, T> {
    pub fn new(categories: impl Into<Cow<'a, [T]>>) -> Self {
        Categorical {
            categories: categories.into(),
        }
    }

    /// Get the categories
    pub fn categories(&self) -> &[T] {
        &self.categories[..]
    }

    /// Sets the categories. Returns old value.
    pub fn set_categories(&mut self, categories: impl Into<Cow<'a, [T]>>) -> Cow<'a, [T]> {
        mem::replace(&mut self.categories, categories.into())
    }
}

impl<'a, T: Clone> Sequence for Categorical<'a, T> {
    type Item<'b>
    where
        'a: 'b,
    = &'b T;
    type Iter<'b>
    where
        'a: 'b,
    = impl Iterator<Item = Self::Item<'b>> + 'b;

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
