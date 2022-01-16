use crate::{
    axis::{Axis, Direction, LabelPosition},
    prelude::*,
    sequence::{Sequence, SpaceAround},
    theme, Categorical, Interval, IntervalTicker, Ticker, Trace,
};
use itertools::izip;
use piet::{
    kurbo::{Affine, Line, Point, Rect, Size},
    Color, RenderContext,
};
use std::{f64::consts::FRAC_2_PI, fmt, sync::Arc};

/// A chart.
///
/// # Type parameters
///  - `RC`: the piet render context. This is used to create text layouts.
pub struct Chart<RC: RenderContext> {
    // An optional axis above the chart.
    top_axis: Option<Axis<Box<dyn Ticker>, RC>>,
    // An optional axis below the chart.
    bottom_axis: Option<Axis<Box<dyn Ticker>, RC>>,
    // An optional axis left of the chart.
    left_axis: Option<Axis<Box<dyn Ticker>, RC>>,
    // An optional axis right of the chart.
    right_axis: Option<Axis<Box<dyn Ticker>, RC>>,
    /// Histogram trace
    traces: Vec<Box<dyn Trace<RC>>>,
    /// The size that everything should fit in (inc. axes).
    size: Size,
}

impl<RC: RenderContext> Chart<RC> {
    pub fn new(size: Size) -> Self {
        Chart {
            top_axis: None,
            bottom_axis: None,
            left_axis: None,
            right_axis: None,
            traces: vec![],
            size,
        }
    }

    pub fn with_top_axis(self, direction: Direction, ticker: impl Ticker) -> Self {
        let axis = Axis::new(
            direction,
            LabelPosition::Before,
            self.size.width,
            Box::new(ticker),
        );
        self.top_axis = Some(axis);
        self
    }

    pub fn with_bottom_axis(self, direction: Direction, ticker: impl Ticker) -> Self {
        let axis = Axis::new(
            direction,
            LabelPosition::After,
            self.size.width,
            Box::new(ticker),
        );
        self.bottom_axis = Some(axis);
        self
    }

    pub fn with_left_axis(self, direction: Direction, ticker: impl Ticker) -> Self {
        let axis = Axis::new(
            direction,
            LabelPosition::Before,
            self.size.height,
            Box::new(ticker),
        );
        self.left_axis = Some(axis);
        self
    }

    pub fn with_right_axis(self, direction: Direction, ticker: impl Ticker) -> Self {
        let axis = Axis::new(
            direction,
            LabelPosition::After,
            self.size.height,
            Box::new(ticker),
        );
        self.right_axis = Some(axis);
        self
    }

    pub fn with_trace(self, trace: impl Trace<RC>) -> Self {
        self.traces.push(trace);
    }

    pub fn size(&self) -> Size {
        self.size
    }

    /// Lay out the axes and calculate the chart area available.
    ///
    /// Once the chart area has been calculated, each trace will have its `layout` method called.
    ///
    /// This function must be called before `draw`, both after creation and after anything changes.
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
                let axis_size = self.axis_size();
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
        for trace in self.traces {
            trace.layout(chart_size, rc)?;
        }
        Ok(())
    }

    /// Lays out the axes for a given chart size.
    fn layout_axes(&mut self, chart_size: Size, rc: &mut RC) -> Result<(), piet::Error> {
        if let Some(axis) = &mut self.top_axis {
            axis.set_axis_len(chart_size.width);
            axis.layout(rc)?;
        }
        if let Some(axis) = &mut self.bottom_axis {
            axis.set_axis_len(chart_size.width);
            axis.layout(rc)?;
        }
        if let Some(axis) = &mut self.left_axis {
            axis.set_axis_len(chart_size.height);
            axis.layout(rc)?;
        }
        if let Some(axis) = &mut self.right_axis {
            axis.set_axis_len(chart_size.height);
            axis.layout(rc)?;
        }
        Ok(())
    }

    fn axis_size(&self) -> Size {
        Size {
            width: self.left_axis.map(|axis| axis.size().width).unwrap_or(0.)
                + self.right_axis.map(|axis| axis.size().width).unwrap_or(0.),
            height: self.top_axis.map(|axis| axis.size().height).unwrap_or(0.)
                + self
                    .bottom_axis
                    .map(|axis| axis.size().height)
                    .unwrap_or(0.),
        }
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

impl<RC: RenderContext> Trace<RC> for HistogramTrace {
    fn size(&self) -> Size {
        self.size.unwrap()
    }

    fn layout(&mut self, size: Size, _rc: &mut RC) -> Result<(), piet::Error> {
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

    fn draw(&self, rc: &mut RC) {
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
