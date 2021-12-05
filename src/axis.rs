// TODO implement toPrecision from javascript - it gives better results.
use crate::{theme, Range};
use piet::{
    kurbo::{Line, Point, Rect},
    PietText, RenderCtx, TextStorage,
};
use std::fmt::{self, Display};
use to_precision::FloatExt as _;

const SCALE_TICK_MARGIN: f64 = 5.;
/// The absolute minimum label width. This is used to bound the number of labels at the start of
/// our layout algorithm (where labels are removed until the remaining ones fit).
const MIN_LABEL_WIDTH: f64 = 5.;

/// Denotes where the axis will be drawn, relative to the chart area.
///
/// This will affect the text direction of labels. You can use a `Direction::Left` axis vertically
/// by rotating it 90 degress, if this gives you the effect you want.
#[derive(Debug, Copy, Clone, PartialEq)]
pub enum Position {
    Top,
    Left,
    Bottom,
    Right,
}

impl Position {
    /// How many labels can we fit. It's a guess
    fn max_labels(self, bounds: Rect) -> usize {
        match self {
            Direction::X => (bounds.width() / 100.).floor() as usize + 1,
            Direction::Y => (bounds.height() / 40.).floor() as usize + 1,
        }
    }

    fn label_position(self, bounds: Rect, t: f64, size: Size, margin: f64) -> Point {
        let p = self.position(bounds, t);
        match self {
            Direction::X => Point::new(p - 0.5 * size.width, bounds.y1 + SCALE_TICK_MARGIN),
            Direction::Y => Point::new(
                bounds.x0 - size.width - SCALE_TICK_MARGIN,
                p - 0.5 * size.height,
            ),
        }
    }

    fn position(self, bounds: Rect, t: f64) -> f64 {
        match self {
            Direction::X => bounds.x0 + t * bounds.width(),
            Direction::Y => bounds.y1 - t * bounds.height(),
        }
    }

    fn axis_line(self, Rect { x0, y0, x1, y1 }: Rect) -> Line {
        match self {
            Direction::X => Line::new((x0, y1), (x1, y1)),
            Direction::Y => Line::new((x0, y0), (x0, y1)),
        }
    }
}

/// A struct for retaining text layout information for an axis scale.
///
/// [matplotlib ticker](https://github.com/matplotlib/matplotlib/blob/master/lib/matplotlib/ticker.py#L2057)
/// is a good resource.
#[derive(Debug, Clone)]
pub struct Scale {
    pos: Position,
    /// (min, max) the range of the data we are graphing. Can overspill if you want gaps at the
    /// top/bottom, or include 0 if you want.
    data_range: Range,
    /// The graph area
    graph_bounds: Rect,
    /// Axis/mark color
    axis_color: KeyOrValue<Color>,
    // retained
    /// Our computed scale. The length is the computed number of scale ticks we should show. Format
    /// is `(data value, y-coordinate of the tick)`
    scale_ticker: Option<ContTicker>,
    /// Our computed text layouts for the tick labels.
    layouts: Option<Vec<PositionedLayout<ArcStr>>>,
    /// The max size of the layouts.
    max_layout: Option<Size>,
}

impl Scale {
    /// Create a new scale object.
    ///
    ///  - `data_range` is the range of the data, from lowest to highest.
    ///  - `graph_bounds` is the rectangle where the graph will be drawn. We will draw outside this
    ///    area a bit.
    pub fn new(data_range: impl Into<Range>, direction: Direction) -> Self {
        Scale {
            direction,
            data_range: data_range.into(),
            graph_bounds: Rect::ZERO,
            axis_color: theme::AXES_COLOR.into(),
            scale_ticker: None,
            layouts: None,
            max_layout: None,
        }
    }

    pub fn new_y(data_range: impl Into<Range>) -> Self {
        Self::new(data_range, Direction::Y)
    }

    pub fn new_x(data_range: impl Into<Range>) -> Self {
        Self::new(data_range, Direction::X)
    }

    pub fn set_direction(&mut self, d: Direction) {
        if self.direction != d {
            self.direction = d;
            self.invalidate();
        }
    }

    /// Helper function to make sure the range includes 0.
    pub fn include_zero(&mut self) {
        if self.data_range.extend_to(0.) {
            self.invalidate();
        }
    }

    /// Call this function during the parent's update cycle.
    pub fn update(&mut self, ctx: &mut UpdateCtx) -> bool {
        let mut needs_rebuild = false;
        if let Some(layouts) = self.layouts.as_mut() {
            for layout in layouts.iter_mut() {
                needs_rebuild |= layout.layout.needs_rebuild_after_update(ctx);
            }
        }
        needs_rebuild |= ctx.env_key_changed(&theme::AXES_COLOR);
        needs_rebuild
    }

    /// Rebuild the retained state, as needed.
    pub fn rebuild_if_needed(&mut self, ctx: &mut PietText, env: &Env) {
        if self.scale_ticker.is_none() {
            self.layouts = None;
            self.scale_ticker = Some(ContTicker::new(
                self.data_range,
                self.direction.max_labels(self.graph_bounds),
            ));
        }
        if self.layouts.is_none() {
            self.layouts = Some(
                self.scale_ticker
                    .unwrap()
                    .into_iter()
                    .map(|tick| {
                        let mut layout =
                            TextLayout::from_text(format!("{}", tick.value.to_precision(5)));
                        layout.rebuild_if_needed(ctx, env);
                        let size = layout.size();
                        let mut layout = PositionedLayout {
                            position: self.direction.label_position(
                                self.graph_bounds,
                                tick.t,
                                layout.size(),
                                SCALE_TICK_MARGIN,
                            ),
                            layout,
                        };
                        layout.rebuild_if_needed(ctx, env);
                        layout
                    })
                    .collect(),
            );
        }
        self.rebuild_max_layout();
    }

    pub fn graph_bounds(&self) -> Rect {
        self.graph_bounds
    }

    pub fn set_graph_bounds(&mut self, graph_bounds: Rect) {
        let graph_bounds = graph_bounds.abs();
        if self.graph_bounds != graph_bounds {
            self.invalidate();
            self.graph_bounds = graph_bounds;
        }
    }

    pub fn set_axis_color(&mut self, color: impl Into<KeyOrValue<Color>>) {
        self.axis_color = color.into();
    }

    /// You must have build layouts before calling this
    pub fn max_layout(&self) -> Size {
        self.max_layout.unwrap()
    }

    fn invalidate(&mut self) {
        self.scale_ticker = None;
        self.layouts = None;
        self.max_layout = None;
    }

    /// Make sure the max layout is sync'd with the layouts.
    fn rebuild_max_layout(&mut self) {
        if self.max_layout.is_some() {
            // no need to rebuild
            return;
        }
        let mut max_width = 0.;
        let mut max_height = 0.;
        for layout in self.layouts.as_ref().unwrap() {
            let Size { width, height } = layout.layout.size();
            if width > max_width {
                max_width = width;
            }
            if height > max_height {
                max_height = height;
            }
        }
        self.max_layout = Some(Size {
            width: max_width,
            height: max_height,
        });
    }

    pub fn draw(&mut self, ctx: &mut RenderCtx, env: &Env, draw_axis: bool, draw_labels: bool) {
        // draw axis
        if draw_axis {
            let axis_brush = ctx.solid_brush(self.axis_color.resolve(env));
            ctx.stroke(self.direction.axis_line(self.graph_bounds), &axis_brush, 2.);
        }
        // draw tick labels
        if draw_labels {
            for layout in self.layouts.as_mut().unwrap().iter_mut() {
                layout.draw(ctx);
            }
        }
    }

    /// Convert a data point to a pixel location on this axis
    pub fn pixel_location(&self, v: f64) -> f64 {
        let (min, max) = self.data_range.into();
        let t = (v - min) / (max - min);
        self.direction.position(self.graph_bounds(), t)
    }
}

#[derive(Debug, Clone)]
pub struct PositionedLayout<T> {
    /// The position that the layout should be displayed.
    pub position: Point,
    pub layout: TextLayout<T>,
}

impl<T: TextStorage> PositionedLayout<T> {
    pub fn rebuild_if_needed(&mut self, ctx: &mut PietText, env: &Env) {
        self.layout.rebuild_if_needed(ctx, env);
    }
    pub fn draw(&mut self, ctx: &mut PaintCtx) {
        self.layout.draw(ctx, self.position)
    }
}

/// Able to return a sequence of locations along an axis where ticks should be displayed, and the
/// values that should be displayed there. For continuous scales
#[derive(Debug, Copy, Clone)]
pub struct ContTicker {
    data_range: Range,
    target_num_points: usize,
    // calculated
    spacing: f64,
}

impl ContTicker {
    pub fn new(data_range: Range, target_num_points: usize) -> Self {
        let spacing = calc_tick_spacing(data_range, target_num_points);
        Self {
            data_range,
            target_num_points,
            spacing,
        }
    }

    fn first_tick(&self) -> f64 {
        match self.target_num_points {
            0 | 1 | 2 => self.data_range.min(),
            n => calc_next_tick(self.data_range.min(), self.spacing),
        }
    }
}

// Ticker is `Copy`able, so pass by value to iter.
impl IntoIterator for ContTicker {
    type IntoIter = ContTickerIter;
    type Item = Tick<f64>;

    fn into_iter(self) -> Self::IntoIter {
        ContTickerIter {
            inner: self,
            next_tick: 0,
        }
    }
}

#[derive(Debug, Copy, Clone, PartialEq)]
/// The position at which a tick should be drawn.
pub struct Tick<T> {
    /// The distance along the axis that the value should be displayed at.
    pub t: f64,
    /// the value that should be displayed.
    pub value: T,
}

impl<T> Tick<T> {
    pub fn new(t: f64, value: T) -> Self {
        Self { t, value }
    }
}

impl<T: Display> Display for Tick<T> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        Display::fmt(&self.value, f)
    }
}

/// Iterator over ContTicker (continuous scale)
pub struct ContTickerIter {
    inner: ContTicker,
    next_tick: usize,
}

impl Iterator for ContTickerIter {
    type Item = Tick<f64>;

    fn next(&mut self) -> Option<Self::Item> {
        match self.inner.target_num_points {
            0 => None,
            1 => match self.next_tick {
                0 => {
                    self.next_tick += 1;
                    Some(Tick::new(0., self.inner.data_range.min()))
                }
                _ => None,
            },
            2 => match self.next_tick {
                0 => {
                    self.next_tick += 1;
                    Some(Tick::new(0., self.inner.data_range.min()))
                }
                1 => {
                    self.next_tick += 1;
                    Some(Tick::new(1., self.inner.data_range.max()))
                }
                _ => None,
            },
            n => {
                let value = self.inner.first_tick() + (self.next_tick as f64) * self.inner.spacing;
                let (min, max) = self.inner.data_range.into();
                let t = (value - min) / (max - min);
                if t <= 1. {
                    self.next_tick += 1;
                    Some(Tick::new(t, value))
                } else {
                    None
                }
            }
        }
    }
}

/// Returns gap between each scale tick, in terms of the y variable, that gives closest to the
/// requested `target_count` and is either 1, 2 or 5 ×10<sup>n</sup> for some n (hardcoded for now).
///
/// `max_value` is the maximum value that will be graphed, and `target_count` is the maximum number
/// of increments of the y axis scale we want.
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

/// Get the location of the first tick of the given spacing after the value.
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

/// Count the number of ticks between min and max using the given step
#[inline]
fn count_ticks(range: Range, tick_step: f64) -> usize {
    let start = calc_next_tick(range.min(), tick_step);
    let end = calc_prev_tick(range.max(), tick_step);
    ((end - start) / tick_step).floor() as usize + 1 // fence/fencepost
}

/// Returns (min, max) of the vector.
///
/// NaNs are propogated.
#[inline]
pub fn data_as_range(data: impl Iterator<Item = f64>) -> Range {
    let mut min = f64::INFINITY;
    let mut max = f64::NEG_INFINITY;
    for v in data {
        if v.is_nan() {
            return (f64::NAN, f64::NAN).into();
        }
        if v < min {
            min = v;
        }
        if v > max {
            max = v;
        }
    }
    (min, max).into()
}

#[inline]
fn count_ticks_slow(range: Range, tick_step: f64) -> usize {
    let mut start = calc_next_tick(range.min(), tick_step);
    let end = calc_prev_tick(range.max(), tick_step);
    let mut tick_count = 1;
    while start <= end {
        tick_count += 1;
        start += tick_step;
    }
    // correct for overshoot
    tick_count - 1
}

#[test]
fn test_prev_tick() {
    for (val, step, expected) in vec![(1., 1., 1.), (1., 2., 0.), (-0.5, 1., -1.)] {
        assert_eq!(calc_prev_tick(val, step), expected);
    }
}

#[test]
fn test_next_tick() {
    for (val, step, expected) in vec![(1., 1., 1.), (1., 2., 2.)] {
        assert_eq!(calc_next_tick(val, step), expected);
    }
}

#[test]
fn test_pow_10_just_too_many() {
    for (min, max, num_ticks) in vec![
        (0., 100., 10),
        (-9., 109., 10),
        (-9., 99., 10),
        (1., 10., 2),
        (0.0001, 0.0010, 10),
    ] {
        let range = Range::new(min, max);
        let step = pow_10_just_too_many(range, num_ticks);
        debug_assert!(
            count_ticks_slow(range, step) > num_ticks,
            "count_ticks({:?}, {}) = {} > {}",
            range,
            step,
            count_ticks_slow(range, step),
            num_ticks
        );
        debug_assert!(
            count_ticks_slow(range, step * 10.) <= num_ticks,
            "count_ticks({:?}, {}) = {} <= {}",
            range,
            step * 10.,
            count_ticks_slow(range, step),
            num_ticks
        );
    }
}

#[test]
fn test_count_ticks() {
    for (min, max, step) in vec![(1., 10., 2.)] {
        let r = Range::new(min, max);
        assert_eq!(count_ticks(r, step), count_ticks_slow(r, step));
    }
}
