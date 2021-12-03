use crate::Range;
use std::{borrow::Cow, fmt, sync::Arc};

/// The discrete analogue of `Ranged`.
pub trait Sequence {
    type Display<'a>: fmt::Display
    where
        Self: 'a;

    /// The number of items in the sequence.
    fn len(&self) -> usize;

    /// How you want to format the value at `idx`.
    ///
    /// This function should always return `Some` if `idx` < `self.len()`
    fn display(&self, idx: usize) -> Option<Self::Display<'_>>;
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
    type Display<'a> = f64;

    fn len(&self) -> usize {
        // ignoring overflow for now
        ((self.range.max() - self.range.min()) / self.step).floor() as usize
    }

    fn display(&self, idx: usize) -> Option<Self::Display<'_>> {
        let val = self.range.min() + idx as f64 * self.step;
        if val > self.range.max() {
            None
        } else {
            Some(val)
        }
    }
}

#[derive(Clone)]
pub struct Categorical<'a, T: Clone> {
    categories: Cow<'a, [T]>,
    display: Arc<dyn Fn(&T, &mut fmt::Formatter) -> fmt::Result>,
}

impl<T: Clone + fmt::Debug> fmt::Debug for Categorical<'_, T> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let mut d = f.debug_tuple("Categorical");
        for field in &*self.categories {
            d.field(field);
        }
        d.finish()
    }
}

// TODO make this type anonymous when we can `impl Trait` in trait signature.
pub struct CategoricalDisplay<'a, T: Clone> {
    inner: &'a Categorical<'a, T>,
    idx: usize,
}

impl<'a, T: Clone> fmt::Display for CategoricalDisplay<'a, T> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        (self.inner.display)(&self.inner.categories[self.idx], f)
    }
}

impl<'a, T: Clone> Sequence for Categorical<'a, T> {
    type Display<'b>
    where
        'a: 'b,
    = CategoricalDisplay<'b, T>;

    fn len(&self) -> usize {
        self.categories.len()
    }

    fn display(&self, idx: usize) -> Option<Self::Display<'_>> {
        if self.categories.get(idx).is_some() {
            Some(CategoricalDisplay { inner: self, idx })
        } else {
            None
        }
    }
}
