use crate::{
    axis::{Axis, Direction, LabelPosition},
    prelude::*,
    sequence::{Sequence, SpaceAround},
    theme,
    ticker::Ticker,
    Categorical, Interval, IntervalTicker, Trace,
};
use itertools::{izip, Either};
use piet::{
    kurbo::{Affine, Insets, Line, Point, Rect, Size, Vec2},
    Color, RenderContext,
};
use std::{f64::consts::FRAC_2_PI, fmt, sync::Arc};

/// A histogram
///
/// # Type parameters
///  - `S`: The type of the categories
///  - `RC`: the piet render context. This is used to create text layouts.
pub struct Histogram<S, RC: RenderContext> {
    /// for now always x axis
    category_axis: Axis<SpaceAround<S>, RC>,
    /// for now always y axis
    value_axis: Axis<IntervalTicker, RC>,
    /// The size of the chart area. Does not include axis labels etc.
    chart_size: Size,
    /// Histogram trace
    trace: HistogramTrace,
    /// The gap between barlines.
    ///
    /// Will be clamped to `(0, (bar_width - 5.))`, or `0` if that interval is empty.
    bar_spacing: f64,
    bar_color: Color,
}

impl<S, RC: RenderContext> Histogram<S, RC>
where
    S: Sequence,
    S::Item: fmt::Display,
{
    pub fn new(chart_size: Size, labels: impl Into<S>, values: impl Into<Arc<[f64]>>) -> Self {
        let labels = labels.into();
        let values = values.into();
        let values_interval: IntervalTicker = Interval::from_iter(values.iter().copied())
            .include_zero()
            .into();
        let out = Histogram {
            category_axis: Axis::new(
                Direction::Right,
                LabelPosition::After,
                chart_size.width,
                labels.space_around(),
            ),
            value_axis: Axis::new(
                Direction::Up,
                LabelPosition::Before,
                chart_size.height,
                values_interval,
            ),
            chart_size,
            trace: HistogramTrace::new(chart_size, values),
            bar_spacing: theme::BAR_SPACING,
            bar_color: theme::BAR_COLOR,
        };
        out
    }

    pub fn values(&self) -> &[f64] {
        self.trace.values()
    }

    /// This function returns the amount of extra space required to draw labels/axes/etc.
    ///
    /// # Panics
    ///
    /// This function will panic if `layout` has not been called.
    pub fn insets(&self) -> Insets {
        Insets {
            x0: self.value_axis.size().width,
            y0: 1., // for stroke thickness of 2.
            x1: 0.,
            y1: self.category_axis.size().height,
        }
    }

    /// The true size, taking into account labels.
    ///
    /// # Panics
    ///
    /// This function will panic if `layout` has not been called.
    pub fn size(&self) -> Size {
        (self.chart_size.to_rect() + self.insets()).size()
    }

    /// Gets the offset from the top-left of the chart area.
    fn offset(&self) -> Vec2 {
        let insets = self.insets();
        Vec2::new(insets.x0, insets.y0)
    }

    /// Call this to layout text labels etc.
    ///
    /// This function must be called before `draw`, both after creation and after any parameters
    /// change.
    pub fn layout(&mut self, rc: &mut RC) -> Result<(), piet::Error> {
        self.category_axis.layout(rc)?;
        self.value_axis.layout(rc)?;
        Ok(())
    }

    /// Helper function to draw the graph somewhere other than (0, 0).
    pub fn draw_at(&self, at: impl Into<Point>, rc: &mut RC) {
        rc.with_save(|rc| {
            rc.transform(Affine::translate(at.into().to_vec2() + self.offset()));
            self.draw(rc);
            Ok(())
        })
        .unwrap();
    }

    /// Draw the histogram at (0,0).
    ///
    /// # Panics
    ///
    /// Panics if `layout` was not called.
    pub fn draw(&self, rc: &mut RC) {
        self.draw_grid(rc);
        self.trace.draw(rc);
        self.category_axis.draw((0., self.chart_size.height), rc);
        self.value_axis.draw(Point::ZERO, rc);
    }

    /// Draw on the gridlines (only horizontal)
    fn draw_grid(&self, rc: &mut RC) {
        for tick in self.value_axis.ticks() {
            rc.stroke(
                Line::new((0., tick.pos), (self.chart_size.width, tick.pos)),
                &theme::GRID_COLOR,
                1.,
            );
        }
    }
}

impl<L, RC> Histogram<Categorical<L>, RC>
where
    RC: RenderContext,
    L: fmt::Display + fmt::Debug + Clone + 'static,
{
    /// Create a histogram from `(label, frequency)` pairs.
    pub fn from_pairs(chart_size: Size, data: impl IntoIterator<Item = (L, f64)>) -> Self {
        let data = data.into_iter();
        let mut labels = Vec::with_capacity(data.size_hint().0);
        let mut values = Vec::with_capacity(data.size_hint().0);
        for (label, value) in data {
            labels.push(label);
            values.push(value);
        }
        Histogram::new(chart_size, labels, values)
    }
}

/// How to draw the bars of the histogram.
pub struct HistogramTrace {
    /// The size of the chart area.
    pub size: Size,
    /// The width of each bar.
    pub bar_width: Option<f64>,
    /// The color to draw the bars.
    pub bar_color: Color,
    /// The values of the bars.
    values: Arc<[f64]>,
    /// The positions of the center of the bars.
    ///
    /// Defaults to evenly spaced bars.
    positions: Option<Arc<[f64]>>,
    /// The value that corresponds to the full chart.
    ///
    /// Defaults to the maximum of `values`.
    pub full_value: Option<f64>,
}

impl HistogramTrace {
    /// A bar-chart style trace.
    ///
    /// # Panics
    ///
    /// This function will panic if the lengths of `positions` and `values` are not equal.
    pub fn new(size: Size, values: impl Into<Arc<[f64]>>) -> Self {
        let values = values.into();
        HistogramTrace {
            size,
            bar_width: None,
            bar_color: theme::BAR_COLOR,
            values,
            positions: None,
            full_value: None,
        }
    }

    fn values(&self) -> &[f64] {
        &self.values
    }

    pub fn set_positions(&mut self, positions: impl Into<Arc<[f64]>>) {
        let positions = positions.into();
        assert_eq!((&*positions).len(), (&*self.values).len());
        self.positions = Some(positions.into());
    }
}

impl Trace for HistogramTrace {
    fn draw<RC: RenderContext>(&self, rc: &mut RC) {
        let full_value = self
            .full_value
            .unwrap_or_else(|| self.values.iter().copied().reduce(f64::max).unwrap_or(1.));

        // The amount of space to fit each bar in.
        let bar_gap = self.size.width / self.values.len() as f64;

        let bar_width = self.bar_width.unwrap_or_else(|| {
            // This function seems to give good results (scale factor chosen by eye)
            const SCALE_F: f64 = 0.004;
            let bar_factor = 1. - ((SCALE_F * bar_gap).atan() * FRAC_2_PI);

            bar_factor * bar_gap
        });
        let bar_width_2 = bar_width * 0.5;

        let positions = match &self.positions {
            Some(i) => Either::Left(i.iter().copied()),
            None => {
                let gap = self.size.width / (self.values.len() as f64);
                Either::Right((0..self.values.len()).map(move |cnt| gap * (0.5 + cnt as f64)))
            }
        };

        for (&val, pos) in izip!(&*self.values, positions) {
            let bar = Rect {
                x0: pos - bar_width_2,
                y0: self.size.height * (1. - val / full_value),
                x1: pos + bar_width_2,
                y1: self.size.height,
            };
            rc.fill(bar, &self.bar_color.clone().with_alpha(0.8));
            rc.stroke(bar, &self.bar_color, 2.);
        }
    }
}
