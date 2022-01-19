use crate::ticker::{Tick, Ticker};
use std::fmt;

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
    const INVALID: Self = Self {
        min: f64::INFINITY,
        max: f64::NEG_INFINITY,
    };

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
        let interval = Interval { min, max };
        if !interval.is_valid() {
            panic!("invalid interval: -∞ < {} < {} < ∞", min, max);
        }
        interval
    }

    /// Whether this interval is valid.
    ///
    /// An interval created with `new` will panic if invalid.
    pub fn is_valid(self) -> bool {
        self.min.is_finite() && self.max.is_finite() && self.min < self.max
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

    /// The middle of the interval
    pub fn center(self) -> f64 {
        (self.max + self.min) * 0.5
    }

    /// Extends this interval to be `factor` times the size, scaled about the center of the
    /// interval.
    pub fn scale_center(self, factor: f64) -> Self {
        let center = self.center();
        let min = (self.min - center) * factor + center;
        let max = (self.max - center) * factor + center;
        Interval::new(min, max)
    }

    /// Extends the interval to nice round numbers.
    pub fn to_rounded(self) -> Self {
        // log_10(2)
        const LOG10_2: f64 = 0.3010299956639812;
        // log_10(5)
        const LOG10_5: f64 = 0.6989700043360189;

        let log10size = self.size().log10() - 1.;
        let mut scale = log10size.floor();
        let rem = log10size - scale;
        if rem < LOG10_2 {
            // scale down to 2 on the previous multiple of 10
            scale += LOG10_2;
        } else if rem < LOG10_5 {
            // scale down to 5 on the previous multiple of 10
            scale += LOG10_5;
        } else {
            scale += 1.
        }
        let scale = 10.0f64.powf(scale);
        let min = self.min.div_euclid(scale) * scale;
        let max = (self.max + scale).div_euclid(scale) * scale;
        Interval::new(min, max)
    }

    /// Get the position between min and max of the given value (0. = min, 1. = max).
    pub fn t(&self, value: f64) -> f64 {
        (value - self.min) / (self.max - self.min)
    }

    pub fn ticker(self) -> IntervalTicker {
        IntervalTicker::new(self)
    }
}

impl FromIterator<f64> for Interval {
    /// Returns the smallest interval that contains all the values in `iter`.
    ///
    /// # Panics
    ///
    /// This function will panic if the iterator is empty, or if any of the values are +∞ or -∞.
    /// NaN values are skipped.
    fn from_iter<I>(iter: I) -> Self
    where
        I: IntoIterator<Item = f64>,
    {
        let mut ival = Self::default();
        ival.extend(iter.into_iter());
        ival
    }
}

impl Extend<f64> for Interval {
    fn extend<I>(&mut self, iter: I)
    where
        I: IntoIterator<Item = f64>,
    {
        for i in iter {
            *self = self.extend_to(i);
        }
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

// This doesn't return a valid interval, but does behave correctly when extending with an iterator.
impl Default for Interval {
    fn default() -> Self {
        Self::INVALID
    }
}

/// Wraps `Interval` and retains some calculations required for `impl Ticker`.
#[derive(Debug)]
pub struct IntervalTicker {
    interval: Interval,
    /// Distance between ticks, distinace to first tick, number of ticks (in units being displayed,
    /// not pixels)
    step_start_count: Option<(f64, f64, usize)>,
    /// 1D affine transform from number space to draw space (scale, translate)
    transform: Option<(f64, f64)>,
}

impl IntervalTicker {
    pub fn new(interval: Interval) -> Self {
        Self {
            interval,
            step_start_count: None,
            transform: None,
        }
    }
}

impl From<Interval> for IntervalTicker {
    fn from(interval: Interval) -> Self {
        Self::new(interval)
    }
}

impl Ticker for IntervalTicker {
    fn layout(&mut self, axis_len: f64) {
        // TODO This is a heuristic that should use the size of the font somehow.
        let max_count = (axis_len / (20. * 3.)) as usize;
        let step = calc_tick_spacing(self.interval, max_count);
        let start = calc_next_tick(self.interval.min(), step);
        // Rely on truncating behavior of `as usize`. TODO check the +1 is correct - I think it is
        // as we count fences but we want fence posts.
        let count = ((self.interval.max() - start) / step) as usize + 1;
        self.step_start_count = Some((step, start, count));

        let scale = axis_len / self.interval.size();
        // The axis always starts at 0, so we just need to remove the start value in value space.
        let translate = -self.interval.min() * scale;
        self.transform = Some((scale, translate));
    }

    fn len(&self) -> usize {
        self.step_start_count.expect("layout not called").2
    }

    fn get(&self, idx: usize) -> Option<Tick> {
        let (step, start, count) = self.step_start_count.expect("layout not called");
        let (scale, translate) = self.transform.unwrap();

        if idx >= count {
            return None;
        }

        let val = idx as f64 * step + start;
        Some(Tick {
            pos: val * scale + translate,
            label: val.to_string().into(),
        })
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

#[test]
fn test_interval_extend() {
    let mut ival: Interval = Default::default();
    ival.extend([1., 2., 3.]);
    assert_eq!(ival, Interval::new(1., 3.));
}
