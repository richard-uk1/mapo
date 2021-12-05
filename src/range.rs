use std::fmt;

/// Maintains invariants: `-∞ < min <= max < ∞`.
///
/// Because this is for continuous data, we ignore whether the range is closed or open.
#[derive(Copy, Clone, PartialEq)]
pub struct Range {
    min: f64,
    max: f64,
}

impl fmt::Debug for Range {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}..{}", self.min, self.max)
    }
}

impl Range {
    /// Construct a range from its min and max
    ///
    /// # Panics
    ///
    /// This function, and all constructors (e.g. `From`) will panic unliness
    /// `-∞ < min <= max < ∞`.
    #[inline]
    pub fn new(min: f64, max: f64) -> Self {
        assert!(
            min.is_finite() && max.is_finite() && min <= max,
            "-∞ < {} <= {} < ∞",
            min,
            max
        );
        Range { min, max }
    }

    #[inline]
    pub fn as_tuple(self) -> (f64, f64) {
        (self.min, self.max)
    }

    #[inline]
    pub fn width(&self) -> f64 {
        self.max - self.min
    }

    #[inline]
    pub fn min(&self) -> f64 {
        self.min
    }

    /// Set the lower end of the range.
    ///
    /// # Panics
    ///
    /// Will panic if the update would not maintain the invariant `-∞ < min <= max < ∞`.
    #[inline]
    pub fn set_min(&mut self, min: f64) -> &mut Self {
        assert!(min.is_finite() && min <= self.max);
        self.min = min;
        self
    }

    #[inline]
    pub fn max(&self) -> f64 {
        self.max
    }

    /// Set the upper end of the range.
    ///
    /// # Panics
    ///
    /// Will panic if the update would not maintain the invariant `-∞ < min <= max < ∞`.
    #[inline]
    pub fn set_max(&mut self, max: f64) -> &mut Self {
        assert!(max.is_finite() && max >= self.min);
        self.max = max;
        self
    }

    #[inline]
    pub fn size(&self) -> f64 {
        self.max - self.min
    }

    /// Extend the range to include `val`.
    ///
    /// # Panics
    ///
    /// Panics if `val` is not finite.
    #[inline]
    pub fn extend_to(mut self, val: f64) -> Self {
        if !val.is_finite() {
            panic!("can only extend to a finite value");
        }
        if val < self.min {
            self.min = val;
        } else if val > self.max {
            self.max = val;
        }
        self
    }

    /// Extend this range to include 0.
    #[inline]
    pub fn include_zero(self) -> Self {
        self.extend_to(0.)
    }

    /// Extends the range to nice round numbers.
    pub fn to_rounded(self) -> Self {
        todo!()
    }

    /// Returns the smallest range that contains all the values in `iter`.
    ///
    /// # Panics
    ///
    /// This function will panic if the iterator is empty, or if any of the values are +∞ or -∞.
    /// NaN values are skipped.
    pub fn from_iter<I>(iter: I) -> Self
    where
        I: IntoIterator<Item = f64>,
    {
        let mut min = f64::INFINITY;
        let mut max = f64::NEG_INFINITY;
        for v in iter {
            if v < min {
                min = v;
            }
            if v > max {
                max = v;
            }
        }
        Range::new(min, max)
    }
}

impl From<(f64, f64)> for Range {
    fn from((min, max): (f64, f64)) -> Self {
        Self::new(min, max)
    }
}

impl From<Range> for (f64, f64) {
    fn from(range: Range) -> (f64, f64) {
        (range.min(), range.max())
    }
}

impl From<std::ops::Range<f64>> for Range {
    fn from(range: std::ops::Range<f64>) -> Self {
        Self::new(range.start, range.end)
    }
}
