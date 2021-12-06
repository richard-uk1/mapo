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

// helpers

/// Returns gap between each scale tick, in terms of the y variable, that gives closest to the
/// requested `target_count` and is either 1, 2 or 5 ×10<sup>n</sup> for some n (hardcoded for now).
pub fn calc_tick_spacing(range: Range, target_count: usize) -> f64 {
    if target_count <= 1 || range.size() == 0. {
        // We don't support a number of ticks less than 2.
        return f64::NAN;
    }
    let too_many_10s = pow_10_just_too_many(range, target_count);
    debug_assert!(
        count_ticks_slow(range, too_many_10s) > target_count,
        "count_ticks({:?}, {}) > {}",
        range,
        too_many_10s,
        target_count
    );
    debug_assert!(
        count_ticks_slow(range, too_many_10s * 10.) <= target_count,
        "count_ticks({:?}, {}) = {} <= {}",
        range,
        too_many_10s * 10.,
        count_ticks_slow(range, too_many_10s * 10.),
        target_count
    );
    // try 2 * our power of 10 that gives too many
    if count_ticks(range, 2. * too_many_10s) <= target_count {
        return 2. * too_many_10s;
    }
    // next try 5 * our power of 10 that gives too many
    if count_ticks(range, 5. * too_many_10s) <= target_count {
        return 5. * too_many_10s;
    }
    debug_assert!(count_ticks(range, 10. * too_many_10s) <= target_count);
    // then it must be the next power of 10
    too_many_10s * 10.
}

/// Find a value of type 10<sup>x</sup> where x is an integer, such that ticks at that distance
/// would result in too many ticks, but ticks at 10<sup>x+1</sup> would give too few (or just
/// right). Returns spacing of ticks
fn pow_10_just_too_many(range: Range, num_ticks: usize) -> f64 {
    // -1 for fence/fence post
    let num_ticks = (num_ticks - 1) as f64;
    let ideal_spacing = range.size() / num_ticks;
    let spacing = (10.0f64).powf(ideal_spacing.log10().floor());
    // The actual value where the first tick will go (we need to work out if we lose too much space
    // at the ends and we end up being too few instead of too many)
    let first_tick = calc_next_tick(range.min(), spacing);
    // If when taking account of the above we still have too many ticks
    // we already -1 from num_ticks.
    if first_tick + num_ticks * spacing < calc_prev_tick(range.max(), spacing) {
        // then just return
        spacing
    } else {
        // else go to the next smaller power of 10
        spacing * 0.1
    }
}
