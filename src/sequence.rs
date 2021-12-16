use crate::ticker::{Tick, Ticker};
use crate::Interval;
use std::{borrow::Cow, fmt, iter, mem, sync::Arc};

/// Because we layout all labels, we should have some cap for when there are so many it will affect
/// perf.  The number should be high enough that you couldn't possibly want more.
const MAX_LABELS: usize = 100;

/// The discrete analogue of `Interval`.
pub trait Sequence: fmt::Debug {
    type Item;
    type Iter: Iterator<Item = Self::Item>;

    /// The number of items in the sequence.
    fn len(&self) -> usize;

    /// The value of the sequence at `idx`.
    ///
    /// This function should always return `Some` if `idx` < `self.len()`
    fn get(&self, idx: usize) -> Option<Self::Item>;

    /// Returns an iterator over the values.
    fn iter(&self) -> Self::Iter;
}

impl<S> Ticker for S
where
    S: Sequence,
    S::Item: fmt::Display,
{
    type TickIter = SequenceTickIter<S>;

    fn len(&self, _axis_len: f64) -> usize {
        Sequence::len(self)
    }

    fn ticks(&self, axis_len: f64) -> Self::TickIter {
        // gap between labels. We'll get NaN when self.len() == 1, but it doesn't matter
        // because the iterator will be empty TODO this isn't true.
        SequenceTickIter {
            gap: axis_len / (self.len() as f64 - 1.),
            inner: self.iter().enumerate(),
        }
    }
}

pub struct SequenceTickIter<S: Sequence> {
    gap: f64,
    inner: iter::Enumerate<S::Iter>,
}

impl<S> Iterator for SequenceTickIter<S>
where
    S: Sequence,
    S::Item: fmt::Display,
{
    type Item = Tick;

    fn next(&mut self) -> Option<Self::Item> {
        let (idx, v) = self.inner.next()?;
        Some(Tick {
            pos: idx as f64 * self.gap,
            label: v.to_string().into(),
        })
    }
}

/// A numeric sequence
#[derive(Debug, Copy, Clone)]
pub struct Numeric {
    interval: Interval,
    step: f64,
}

impl Numeric {
    /// Construct a numeric sequence.
    ///
    /// # Panics
    ///
    /// Panics if `0 < step < ∞` is not true.
    pub fn from_interval_step(interval: Interval, step: f64) -> Self {
        assert!(step.is_finite() && step > 0., "0 < {} < ∞", step);
        Self { interval, step }
    }

    /// Construct a numeric sequence.
    ///
    /// # Panics
    ///
    /// Panics if `0 < step < ∞` or `-∞ < min < max < ∞` is not true.
    pub fn new(min: f64, max: f64, step: f64) -> Self {
        Self::from_interval_step(Interval::new(min, max), step)
    }

    /// Get the interval for this sequence.
    pub fn interval(&self) -> Interval {
        self.interval
    }

    /// Get the minimum value for this sequence.
    pub fn min(&self) -> f64 {
        self.interval.min()
    }

    /// Get the maximum value for this sequence.
    ///
    /// Note that this might be different from `self.interval().max()` if `self.interval()`'s
    /// length is not an exact multiple of `self.step()`.
    pub fn max(&self) -> f64 {
        let last_idx = (self.interval.size() / self.step).floor();
        self.interval.min() + self.step * last_idx
    }

    /// Get the step value for this sequence.
    pub fn step(&self) -> f64 {
        self.step
    }
}

impl Sequence for Numeric {
    type Item = f64;
    type Iter = NumericIter;

    fn len(&self) -> usize {
        // ignoring overflow for now
        ((self.interval.max() - self.interval.min()) / self.step).floor() as usize
    }

    fn get(&self, idx: usize) -> Option<Self::Item> {
        let val = self.interval.min() + idx as f64 * self.step;
        if val > self.interval.max() {
            None
        } else {
            Some(val)
        }
    }

    fn iter(&self) -> Self::Iter {
        NumericIter::new(*self)
    }
}

pub struct NumericIter {
    inner: Numeric,
    next: f64,
}

impl NumericIter {
    fn new(inner: Numeric) -> Self {
        NumericIter {
            inner,
            next: inner.interval.min(),
        }
    }
}

impl Iterator for NumericIter {
    type Item = f64;

    fn next(&mut self) -> Option<Self::Item> {
        if self.next > self.inner.interval.max() {
            return None;
        }
        let out = self.next;
        self.next += self.inner.step;
        Some(out)
    }
}

/// A list of categories.
///
/// For this list to be used as an axis, the categories (`T`) should implement `Clone` and
/// `Display`. It's recommended to keep the cost of cloning `T` cheap (possibly using reference
/// counting).
#[derive(Debug, Clone)]
pub struct Categorical<T> {
    categories: Arc<[T]>,
}

impl<T> Categorical<T> {
    /// Create a new list of categories.
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

impl<T: fmt::Debug + Clone + 'static> Sequence for Categorical<T> {
    type Item = T;
    type Iter = CategoricalIter<T>;

    fn len(&self) -> usize {
        self.categories.len()
    }

    fn get(&self, idx: usize) -> Option<Self::Item> {
        self.categories.get(idx).cloned()
    }

    fn iter(&self) -> Self::Iter {
        CategoricalIter {
            inner: self.categories.clone(),
            idx: 0,
        }
    }
}

pub struct CategoricalIter<T> {
    inner: Arc<[T]>,
    idx: usize,
}

impl<T: Clone> Iterator for CategoricalIter<T> {
    type Item = T;
    fn next(&mut self) -> Option<Self::Item> {
        let out = self.inner.get(self.idx)?;
        self.idx += 1;
        Some((*out).clone())
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

#[derive(Debug)]
pub struct SpaceAround<S>(S);

impl<S> std::ops::Deref for SpaceAround<S> {
    type Target = S;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<S> Ticker for SpaceAround<S>
where
    S: Sequence,
    S::Item: fmt::Display,
{
    type TickIter = SpaceAroundTickIter<S>;

    fn len(&self, _axis_len: f64) -> usize {
        self.0.len()
    }

    fn ticks(&self, axis_len: f64) -> Self::TickIter {
        // gap between labels. We'll get NaN when self.len() == 0, but it doesn't matter
        // because the iterator will be empty
        let gap = axis_len / self.0.len() as f64;
        SpaceAroundTickIter {
            gap: axis_len / self.0.len() as f64,
            inner: self.0.iter().enumerate(),
        }
    }
}

// We have to do this because we have to name the type (until type_alias_impl_trait lands)
pub struct SpaceAroundTickIter<S: Sequence> {
    gap: f64,
    inner: iter::Enumerate<S::Iter>,
}

impl<S> Iterator for SpaceAroundTickIter<S>
where
    S: Sequence,
    S::Item: fmt::Display,
{
    type Item = Tick;

    fn next(&mut self) -> Option<Self::Item> {
        let (idx, v) = self.inner.next()?;
        Some(Tick {
            pos: (idx as f64 + 0.5) * self.gap,
            label: v.to_string().into(),
        })
    }
}
