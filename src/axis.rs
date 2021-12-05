// TODO implement toPrecision from javascript - it gives better results.
use crate::{
    theme,
    ticker::{Tick, Ticker},
    ArcStr, Range, Sequence,
};
use piet::{
    kurbo::{Affine, Line, Point, Rect, Size},
    Color, RenderContext, Text, TextAttribute, TextLayout, TextLayoutBuilder, TextStorage,
};
use std::{
    fmt::{self, Display},
    sync::Arc,
};
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
pub enum LabelPosition {
    /// above or to the left
    Before,
    /// below or to the right
    After,
}

// # Plan
//
// A scale has ticks and labels. The implementation of a scale will supply all the ticks and labels
// (with the size in pixels as input). It will then be up to a wrapper to layout the labels and work
// out how many we can fit (and where they should actually be displayed). For now labels will be
// String only.

/// Axes must be drawn either vertically or horizontally.
#[derive(Debug, Clone, Copy)]
pub enum Direction {
    Vertical,
    Horizontal,
}

/// A struct for retaining text layout information for an axis scale.
///
/// This struct knows everything it needs to draw the axis, ticks, and labels.
///
/// [matplotlib ticker](https://github.com/matplotlib/matplotlib/blob/master/lib/matplotlib/ticker.py#L2057)
/// is a good resource.
#[derive(Clone)]
pub struct Axis<T, RC: RenderContext> {
    /// Whether the axis is vertical or horizontal.
    direction: Direction,
    /// Where the labels should be shown. Ticks will be drawn on the opposite side.
    label_pos: LabelPosition,
    /// How long the axis will be
    axis_len: f64,
    /// An object that knows where the ticks should be drawn.
    ticker: T,

    // style

    // /// Axis/mark color
    // axis_color: Color,

    // retained
    /// Our computed text layouts for the tick labels.
    ///
    /// This is cached, and invalidated by clearing the vec. This way we
    /// can re-use the allocation. To see if cache is valid, check its
    /// length against `ticker.len()`.
    label_layouts: Vec<<RC as RenderContext>::TextLayout>,
    /// Which of the layouts we are actually going to draw.
    labels_to_draw: Vec<usize>,
}

impl<T: Ticker, RC: RenderContext> Axis<T, RC> {
    /// Create a new axis.
    pub fn new(direction: Direction, label_pos: LabelPosition, axis_len: f64, ticker: T) -> Self {
        assert!(axis_len >= 0.);
        Self {
            direction,
            label_pos,
            axis_len,
            ticker,
            label_layouts: vec![],
            labels_to_draw: vec![],
        }
    }

    pub fn set_ticker(mut self, new_ticker: T) -> Axis<T, RC> {
        self.label_layouts.clear();
        Axis {
            direction: self.direction,
            label_pos: self.label_pos,
            axis_len: self.axis_len,
            ticker: new_ticker,
            label_layouts: self.label_layouts,
            labels_to_draw: self.labels_to_draw,
        }
    }

    // Call this before draw.
    pub fn layout(&mut self, ctx: &mut RC) -> Result<(), piet::Error> {
        self.build_layouts(ctx)?;
        self.fit_labels();
        Ok(())
    }

    /// Draw the layout
    pub fn draw(&self, pos: Point, ctx: &mut RC) {
        if self.label_layouts.is_empty() && self.ticker.len(self.axis_len) != 0 {
            panic!("Must call `layout` before `draw`");
        }
        ctx.with_save(|ctx| {
            ctx.transform(Affine::translate(pos.to_vec2()));
            // axis line (extend to contain tick at edge)
            let axis_line = match self.direction {
                Direction::Horizontal => Line::new((-1., 0.), (self.axis_len + 1., 0.)),
                Direction::Vertical => Line::new((0., -1.), (0., self.axis_len + 1.)),
            };
            ctx.stroke(axis_line, &Color::BLACK, 2.);

            // ticks
            for tick in self.ticker.ticks(self.axis_len) {
                let tick_line = match (self.direction, self.label_pos) {
                    (Direction::Vertical, LabelPosition::Before) => {
                        // right
                        Line::new((0., tick.pos), (5., tick.pos))
                    }
                    (Direction::Vertical, LabelPosition::After) => {
                        // left
                        Line::new((0., tick.pos), (-5., tick.pos))
                    }
                    (Direction::Horizontal, LabelPosition::Before) => {
                        // below
                        Line::new((tick.pos, 0.), (tick.pos, 5.))
                    }
                    (Direction::Horizontal, LabelPosition::After) => {
                        // above
                        Line::new((tick.pos, 0.), (tick.pos, -5.))
                    }
                };
                ctx.stroke(tick_line, &Color::grey8(80), 1.);
            }

            // labels
            for idx in self.labels_to_draw.iter().copied() {
                let layout = &self.label_layouts[idx];
                let tick = self.ticks().nth(idx).unwrap();
                let pos = match (self.direction, self.label_pos) {
                    (Direction::Vertical, LabelPosition::Before) => {
                        // left
                        Point::new(
                            -layout.size().width - 5.,
                            tick.pos - layout.size().height * 0.5,
                        )
                    }
                    (Direction::Vertical, LabelPosition::After) => {
                        // right
                        todo!()
                    }
                    (Direction::Horizontal, LabelPosition::Before) => {
                        // above
                        todo!()
                    }
                    (Direction::Horizontal, LabelPosition::After) => {
                        // below
                        Point::new(tick.pos - layout.size().width * 0.5, 5.)
                    }
                };
                ctx.draw_text(layout, pos);
            }
            Ok(())
        })
        .unwrap()
    }

    fn build_layouts(&mut self, ctx: &mut RC) -> Result<(), piet::Error> {
        if !self.label_layouts.is_empty() || self.ticker.len(self.axis_len) == 0 {
            // nothing to do
            return Ok(());
        }
        let text = ctx.text();
        for tick in self.ticker.ticks(self.axis_len) {
            let layout = text
                .new_text_layout(tick.label)
                .default_attribute(TextAttribute::FontSize(12.))
                .build()?;

            self.label_layouts.push(layout);
        }
        Ok(())
    }

    /// This function needs to be called every time anything affecting label
    /// positioning changes.
    fn fit_labels(&mut self) {
        // Start by trying to fit in all labels, then keep missing more out until
        // they will fit
        let mut step = 1;
        // the loop will never run iff `self.label_layouts.len() == 0`. The below
        // divides by 2, rounding up.
        while step <= (self.label_layouts.len() + 1) / 2 {
            self.labels_to_draw.clear();
            // TODO if the remainder is odd, put the gap in the middle, if even, split
            // it between the ends.
            self.labels_to_draw
                .extend((0..self.label_layouts.len()).step_by(step));
            if self.test_layouts_fit() {
                return;
            }
            step += 1;
        }
        // If we can't layout anything, then show nothing.
        self.labels_to_draw.clear();
    }

    /// Returns `true` if all the labels selected for drawing will fit without overlapping
    /// each other.
    ///
    /// # Panics
    ///
    /// Panics if the label layouts have not been built.
    fn test_layouts_fit(&self) -> bool {
        let mut prev_end = f64::NEG_INFINITY;
        match self.direction {
            Direction::Vertical => {
                for idx in self.labels_to_draw.iter().copied() {
                    let layout = &self.label_layouts[idx];
                    let size = layout.size();
                    let tick = self.ticks().nth(idx).unwrap();
                    if prev_end >= tick.pos - size.height * 0.5 {
                        return false;
                    }
                    prev_end = tick.pos + size.height * 0.5;
                }
            }
            Direction::Horizontal => {
                for idx in self.labels_to_draw.iter().copied() {
                    let layout = &self.label_layouts[idx];
                    let size = layout.size();
                    let tick = self.ticks().nth(idx).unwrap();
                    if prev_end >= tick.pos - size.width * 0.5 {
                        return false;
                    }
                    prev_end = tick.pos + size.width * 0.5;
                }
            }
        }
        true
    }

    fn ticks(&self) -> impl Iterator<Item = Tick> + '_ {
        self.ticker.ticks(self.axis_len)
    }

    /*
    pub fn set_direction(&mut self, d: Direction) {
        if self.direction != d {
            self.direction = d;
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
    */
}

/*
#[derive(Clone)]
pub enum SeriesKind<S = !> {
    Range(Range),
    Sequence(S),
}

impl SeriesKind {
    fn from_range(range: Range) -> Self {
        Self::Range (range,
            fmt: Arc::new(|v, f| write!(f, "{:.4}", v)),
        }
    }
}

impl<S> SeriesKind<S> {
    fn from_sequence(sequence: S) -> Self {
        Self::Sequence { inner: sequence }
    }
}

impl<S: Sequence> SeriesKind<S> {
    fn test() {}
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
*/
