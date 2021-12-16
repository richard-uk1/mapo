use crate::ticker::{Tick, Ticker};
use std::{cell::RefCell, fmt};

/// An [interval](https://en.wikipedia.org/wiki/Interval_(mathematics)) of real numbers.
///
/// Maintains invariants: `-∞ < min < max < ∞`.
///
/// Because this is for continuous data, we ignore whether the interval is closed or open.
#[derive(Copy, Clone, PartialEq)]
pub struct Interval {
    min: f64,
    max: f64,
}

impl fmt::Debug for Interval {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}..{}", self.min, self.max)
    }
}

impl Interval {
    /// Construct a interval from its min and max
    ///
    /// # Panics
    ///
    /// This function, and all constructors (e.g. `From`) will panic unless
    /// `-∞ < min < max < ∞`.
    #[inline]
    pub fn new(min: f64, max: f64) -> Self {
        assert!(
            min.is_finite() && max.is_finite() && min < max,
            "-∞ < {} < {} < ∞",
            min,
            max
        );
        Interval { min, max }
    }

    /// Get the `(lower bound, upper bound)` a.k.a. `(min, max)` of the interval.
    #[inline]
    pub fn as_tuple(self) -> (f64, f64) {
        (self.min, self.max)
    }

    /// The lower bound of the interval.
    #[inline]
    pub fn min(&self) -> f64 {
        self.min
    }

    /// Set the lower end of the interval.
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

    /// Set the upper end of the interval.
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

    /// Extend the interval to include `val`.
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

    /// Extend this interval to include 0.
    #[inline]
    pub fn include_zero(self) -> Self {
        self.extend_to(0.)
    }

    /// Extends the interval to nice round numbers.
    pub fn to_rounded(self) -> Self {
        todo!()
    }

    /// Returns the smallest interval that contains all the values in `iter`.
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
        Interval::new(min, max)
    }
}

impl From<(f64, f64)> for Interval {
    fn from((min, max): (f64, f64)) -> Self {
        Self::new(min, max)
    }
}

impl From<Interval> for (f64, f64) {
    fn from(interval: Interval) -> (f64, f64) {
        (interval.min(), interval.max())
    }
}

impl From<std::ops::Range<f64>> for Interval {
    fn from(range: std::ops::Range<f64>) -> Self {
        Self::new(range.start, range.end)
    }
}

/// Wraps `Interval` and retains some calculations required for `impl Ticker`.
#[derive(Debug)]
pub struct IntervalTicker {
    interval: Interval,
    step: RefCell<Option<f64>>,
}

impl IntervalTicker {
    pub fn interval(&self) -> Interval {
        self.interval
    }

    pub fn set_interval(&mut self, interval: Interval) {
        self.interval = interval;
        *self.step.borrow_mut() = None;
    }

    /// Calculates and caches the step size.
    fn step(&self, axis_len: f64) -> f64 {
        // For now, we are going to assume that the interval is vertical, and that the text height is
        // 20px. TODO drop the assumptions
        //
        // (*3): leave at least same-sized gap above and below label.
        let max_count = (axis_len / (20. * 3.)) as usize;
        *self
            .step
            .borrow_mut()
            .get_or_insert_with(|| calc_tick_spacing(self.interval, max_count))
    }
}

impl From<Interval> for IntervalTicker {
    fn from(interval: Interval) -> Self {
        IntervalTicker {
            interval,
            step: RefCell::new(None),
        }
    }
}

impl Ticker for IntervalTicker {
    type TickIter = TickIter;

    fn len(&self, axis_len: f64) -> usize {
        let step = self.step(axis_len);
        let start = calc_next_tick(self.interval.min(), step);
        // Rely on truncating behavior of `as usize`.
        ((self.interval.max() - start) / step) as usize
    }

    fn ticks(&self, axis_len: f64) -> Self::TickIter {
        let step = self.step(axis_len);
        let start = calc_next_tick(self.interval.min(), step);
        TickIter {
            pos: start,
            step,
            interval: self.interval,
            axis_len,
        }
    }
}

pub struct TickIter {
    pos: f64,
    step: f64,
    interval: Interval,
    axis_len: f64,
}

impl Iterator for TickIter {
    type Item = Tick;
    fn next(&mut self) -> Option<Self::Item> {
        if self.pos > self.interval.max() {
            return None;
        }
        let next = self.pos;
        self.pos += self.step;

        let t = (next - self.interval.min()) / self.interval.size();
        Some(Tick {
            pos: self.axis_len * t,
            label: next.to_string().into(),
        })
    }

    fn nth(&mut self, n: usize) -> Option<Self::Item> {
        if self.pos > self.interval.max() {
            return None;
        }
        self.pos += self.step * n as f64;
        self.next()
    }
}
// helpers

/// Returns gap between each scale tick, in terms of the y variable, that gives closest to the
/// requested `target_count` and is either 1, 2 or 5 ×10<sup>n</sup> for some n (hardcoded for now).
pub fn calc_tick_spacing(interval: Interval, target_count: usize) -> f64 {
    if target_count <= 1 || interval.size() == 0. {
        // We don't support a number of ticks less than 2.
        return f64::NAN;
    }
    let too_many_10s = pow_10_just_too_many(interval, target_count);
    debug_assert!(
        count_ticks_slow(interval, too_many_10s) > target_count,
        "count_ticks({:?}, {}) > {}",
        interval,
        too_many_10s,
        target_count
    );
    debug_assert!(
        count_ticks_slow(interval, too_many_10s * 10.) <= target_count,
        "count_ticks({:?}, {}) = {} <= {}",
        interval,
        too_many_10s * 10.,
        count_ticks_slow(interval, too_many_10s * 10.),
        target_count
    );
    // try 2 * our power of 10 that gives too many
    if count_ticks(interval, 2. * too_many_10s) <= target_count {
        return 2. * too_many_10s;
    }
    // next try 5 * our power of 10 that gives too many
    if count_ticks(interval, 5. * too_many_10s) <= target_count {
        return 5. * too_many_10s;
    }
    debug_assert!(count_ticks(interval, 10. * too_many_10s) <= target_count);
    // then it must be the next power of 10
    too_many_10s * 10.
}

/// Find a value of type 10<sup>x</sup> where x is an integer, such that ticks at that distance
/// would result in too many ticks, but ticks at 10<sup>x+1</sup> would give too few (or just
/// right). Returns spacing of ticks
fn pow_10_just_too_many(interval: Interval, num_ticks: usize) -> f64 {
    // -1 for fence/fence post
    let num_ticks = (num_ticks - 1) as f64;
    let ideal_spacing = interval.size() / num_ticks;
    let spacing = (10.0f64).powf(ideal_spacing.log10().floor());
    // The actual value where the first tick will go (we need to work out if we lose too much space
    // at the ends and we end up being too few instead of too many)
    let first_tick = calc_next_tick(interval.min(), spacing);
    // If when taking account of the above we still have too many ticks
    // we already -1 from num_ticks.
    if first_tick + num_ticks * spacing < calc_prev_tick(interval.max(), spacing) {
        // then just return
        spacing
    } else {
        // else go to the next smaller power of 10
        spacing * 0.1
    }
}

/// Get the location of the first tick of the given spacing after the value.
///
/// Used to find the first tick to display.
#[inline]
pub fn calc_next_tick(v: f64, spacing: f64) -> f64 {
    // `v <-> next tick`
    let v_tick_diff = v.rem_euclid(spacing);
    if v_tick_diff == 0. {
        v
    } else {
        v - v_tick_diff + spacing
    }
}

/// Get the location of the first tick of the given spacing before the value.
///
/// Used to find the last tick to display.
#[inline]
pub fn calc_prev_tick(v: f64, spacing: f64) -> f64 {
    // `prev tick <-> v`
    let v_tick_diff = v.rem_euclid(spacing);
    if v_tick_diff == spacing {
        v
    } else {
        v - v_tick_diff
    }
}

/// Count the number of ticks between min and max using the given step, aligning the ticks to
/// sensible values.
#[inline]
fn count_ticks(interval: Interval, tick_step: f64) -> usize {
    let start = calc_next_tick(interval.min(), tick_step);
    let end = calc_prev_tick(interval.max(), tick_step);
    ((end - start) / tick_step).floor() as usize + 1 // fence/fencepost
}

/// An alternate way to calculate the number of ticks. Used for testing.
#[inline]
fn count_ticks_slow(interval: Interval, tick_step: f64) -> usize {
    let mut start = calc_next_tick(interval.min(), tick_step);
    let end = calc_prev_tick(interval.max(), tick_step);
    let mut tick_count = 1;
    while start <= end {
        tick_count += 1;
        start += tick_step;
    }
    // correct for overshoot
    tick_count - 1
}
