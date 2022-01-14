use crate::{
    axis::{Axis, Direction, LabelPosition},
    prelude::*,
    sequence::{Sequence, SpaceAround},
    theme, Categorical, Interval, IntervalTicker, Trace,
};
use itertools::izip;
use piet::{
    kurbo::{Affine, Line, Point, Rect, Size},
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
    /// The size of the whole chart
    size: Size,
    /// Histogram trace
    bars: HistogramTrace,
}

impl<S, RC: RenderContext> Histogram<S, RC>
where
    S: Sequence,
    S::Item: fmt::Display,
{
    pub fn new(size: Size, labels: impl Into<S>, values: impl Into<Arc<[f64]>>) -> Self {
        let labels = labels.into();
        let values = values.into();
        let values_interval: IntervalTicker = Interval::from_iter(values.iter().copied())
            .include_zero()
            .into();
        let bars = HistogramTrace::new(values);
        let out = Histogram {
            category_axis: Axis::new(
                Direction::Right,
                LabelPosition::After,
                size.width,
                labels.space_around(),
            ),
            value_axis: Axis::new(
                Direction::Up,
                LabelPosition::Before,
                size.height,
                values_interval,
            ),
            size,
            bars,
        };
        out
    }

    pub fn values(&self) -> &[f64] {
        self.bars.values()
    }

    pub fn set_bar_color(&mut self, color: Color) {
        self.bars.bar_color = color;
    }

    /*
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
    */

    /// The true size, taking into account labels.
    ///
    /// # Panics
    ///
    /// This function will panic if `layout` has not been called.
    pub fn size(&self) -> Size {
        self.size
    }

    /*
    /// Gets the offset from the top-left of the chart area.
    fn offset(&self) -> Vec2 {
        let insets = self.insets();
        Vec2::new(insets.x0, insets.y0)
    }
    */

    /// Call this to layout text labels etc.
    ///
    /// This function must be called before `draw`, both after creation and after any parameters
    /// change.
    ///
    /// The `layout` function is split out from the `draw` function because text layout creation
    /// can fail. This way, the draw function has no failure paths (providing `layout` gets
    /// called).
    pub fn layout(&mut self, rc: &mut RC) -> Result<(), piet::Error> {
        // Loop until our layout fits.
        // The initial guess is the whole area (we know this will be too big, bug it gives a first
        // estimate for the axis sizes.
        let mut chart_size = self.size;
        // We abuse labelled loops so we can run some code if the for loop finishes before a
        // solution has been found.
        'found_height: loop {
            // We expect this loop to complete after 2 loops.
            for _ in 0..10 {
                // Lay out the axes at the current size.
                self.layout_axes(chart_size, rc)?;
                // This size contains the space we need for the axes
                let axis_size = Size {
                    width: self.value_axis.size().width,
                    height: self.category_axis.size().height,
                };
                if axis_size.height + chart_size.height < self.size.height
                    && axis_size.width + chart_size.width < self.size.width
                {
                    // we've found a valid chart size
                    break 'found_height;
                }
                // Chart size is still too big, try shrinking it to what would have fit with the
                // current axes, minus a small delta to try to take fp accuracy out of the equation.
                chart_size.height = self.size.height - axis_size.height - 1e-8;
                chart_size.width = self.size.width - axis_size.width - 1e-8;
            }
            // We didn't find a solution, so warn and just draw as best we can
            // TODO make a log msg
            eprintln!("We didn't find a valid chart size, so the chart may overflow");
            chart_size *= 0.9;
            self.layout_axes(chart_size, rc)?;
            break;
        }
        self.bars.layout(chart_size, rc)?;
        Ok(())
    }

    /// Lays out the axes for a given chart size.
    fn layout_axes(&mut self, chart_size: Size, rc: &mut RC) -> Result<(), piet::Error> {
        self.category_axis.set_axis_len(chart_size.width);
        self.category_axis.layout(rc)?;
        self.value_axis.set_axis_len(chart_size.height);
        self.value_axis.layout(rc)?;
        Ok(())
    }

    /// Helper function to draw the graph somewhere other than (0, 0).
    pub fn draw_at(&self, at: impl Into<Point>, rc: &mut RC) {
        rc.with_save(|rc| {
            rc.transform(Affine::translate(at.into().to_vec2()));
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
        rc.with_save(|rc| {
            rc.transform(Affine::translate((self.value_axis.size().width, 0.)));
            self.bars.draw(rc);
            rc.with_save(|rc| {
                rc.transform(Affine::translate((0., self.bars.size().height)));
                self.category_axis.draw(rc);
                Ok(())
            })?;
            Ok(())
        })
        .unwrap();
        self.value_axis.draw(rc);
    }

    /// Draw on the gridlines (only horizontal)
    fn draw_grid(&self, rc: &mut RC) {
        let width = self.value_axis.size().width;
        for tick in self.value_axis.ticks() {
            rc.stroke(
                Line::new((width, tick.pos), (self.size.width, tick.pos)),
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
    pub size: Option<Size>,
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
    pub fn new(values: impl Into<Arc<[f64]>>) -> Self {
        let values = values.into();
        HistogramTrace {
            size: None,
            bar_width: None,
            bar_color: theme::BAR_COLOR,
            values,
            positions: None,
            full_value: None,
        }
    }

    /// Get the numeric values of the bars in this histogram.
    fn values(&self) -> &[f64] {
        &self.values
    }

    /// Specify where you want the bars to be positioned.
    ///
    /// Might bin this method.
    pub fn set_positions(&mut self, positions: impl Into<Arc<[f64]>>) {
        let positions = positions.into();
        assert_eq!((&*positions).len(), (&*self.values).len());
        self.positions = Some(positions.into());
    }
}

impl Trace for HistogramTrace {
    fn size(&self) -> Size {
        self.size.unwrap()
    }

    fn layout<RC: RenderContext>(&mut self, size: Size, _rc: &mut RC) -> Result<(), piet::Error> {
        if self.size == Some(size) {
            return Ok(());
        }
        self.size = Some(size);
        if self.full_value.is_none() {
            self.full_value = Some(self.values.iter().copied().reduce(f64::max).unwrap_or(1.));
        }
        if self.bar_width.is_none() {
            const SCALE_F: f64 = 0.004;
            let bar_gap = size.width / self.values.len() as f64;
            let bar_factor = 1. - ((SCALE_F * bar_gap).atan() * FRAC_2_PI);
            self.bar_width = Some(bar_factor * bar_gap);
        }
        if self.positions.is_none() {
            let gap = size.width / (self.values.len() as f64);
            self.positions = Some(
                (0..self.values.len())
                    .map(move |cnt| gap * (0.5 + cnt as f64))
                    .collect(),
            );
        }
        Ok(())
    }

    fn draw<RC: RenderContext>(&self, rc: &mut RC) {
        let size = self.size.unwrap();
        let full_value = self.full_value.unwrap();
        let bar_width = self.bar_width.unwrap();
        let bar_width_2 = bar_width * 0.5;
        let positions = self.positions.as_ref().unwrap().iter().copied();

        for (&val, pos) in izip!(&*self.values, positions) {
            let bar = Rect {
                x0: pos - bar_width_2,
                y0: size.height * (1. - val / full_value),
                x1: pos + bar_width_2,
                y1: size.height,
            };
            rc.fill(bar, &self.bar_color.clone().with_alpha(0.8));
            rc.stroke(bar, &self.bar_color, 2.);
        }
    }
}
