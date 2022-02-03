use crate::{prelude::*, theme, Categorical, Chart, GridStyle, Interval, Trace};
use itertools::izip;
use piet_common::{
    kurbo::{Rect, Size},
    Color, Error as PietError, Piet, RenderContext,
};
use std::{any::Any, f64::consts::FRAC_2_PI, fmt, sync::Arc};

pub fn histogram<L>(labels: impl Into<Categorical<L>>, values: impl Into<Arc<[f64]>>) -> Chart
where
    L: Clone + fmt::Debug + fmt::Display + 'static,
{
    let labels = labels.into();
    let values = values.into();
    let values_interval = Interval::from_iter(values.iter().copied())
        .include_zero()
        .to_rounded();
    let bars_trace = HistogramTrace::new(values).with_y_range(values_interval);
    Chart::new()
        .with_left_axis(values_interval.ticker().reverse())
        .with_left_grid(GridStyle::default())
        .with_bottom_axis(labels.space_around_ticker())
        .with_trace(bars_trace)
}
/// Create a histogram from `(label, frequency)` pairs.
pub fn histogram_from_pairs<L>(data: impl IntoIterator<Item = (L, f64)>) -> Chart
where
    L: fmt::Display + fmt::Debug + Clone + 'static,
{
    let (labels, values): (Vec<L>, Vec<f64>) = data.into_iter().unzip();
    histogram(labels, values)
}

/// How to draw the bars of the histogram.
pub struct HistogramTrace {
    /// The values of the bars.
    ///
    /// Not public because we have retained state that depends on them.
    values: Arc<[f64]>,
    /// The width of each bar.
    pub bar_width: Option<f64>,
    /// The color to draw the bars.
    pub bar_color: Color,
    /// The maximum value to use for y
    ///
    /// The maximum value in `values` would be a sensible choice.
    y_range: Option<Interval>,

    // Retained
    /// The size of the chart area.
    pub size: Option<Size>,
    /// The positions of the center of the bars.
    ///
    /// Defaults to evenly spaced bars.
    positions: Option<Arc<[f64]>>,
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
            bar_width: None,
            bar_color: theme::BAR_COLOR,
            values,
            y_range: None,
            size: None,
            positions: None,
        }
    }

    /// Sets the maximum value of the y axis.
    ///
    /// Defaults to the largest value in `values`.
    pub fn with_y_range(mut self, y_range: Interval) -> Self {
        self.y_range = Some(y_range);
        self
    }

    /// Get the numeric values of the bars in this histogram.
    pub fn values(&self) -> &[f64] {
        &self.values
    }

    /// Specify where you want the bars to be positioned.
    ///
    /// Might bin this method.
    pub fn set_positions(&mut self, positions: impl Into<Arc<[f64]>>) {
        let positions = positions.into();
        assert_eq!((&*positions).len(), (&*self.values).len());
        self.positions = Some(positions);
    }
}

impl Trace for HistogramTrace {
    fn size(&self) -> Size {
        self.size.unwrap()
    }

    fn layout(&mut self, size: Size, _rc: &mut Piet) -> Result<(), PietError> {
        if self.size == Some(size) {
            return Ok(());
        }
        self.size = Some(size);
        if self.y_range.is_none() {
            self.y_range = Some(
                self.values
                    .iter()
                    .copied()
                    .collect::<Interval>()
                    .extend_to(0.),
            );
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

    fn draw(&self, rc: &mut Piet) {
        let size = self.size.unwrap();
        let y_range = self.y_range.unwrap();
        let bar_width = self.bar_width.unwrap();
        let bar_width_2 = bar_width * 0.5;
        let positions = self.positions.as_ref().unwrap().iter().copied();

        let zero = size.height * (1. - y_range.t(0.));
        for (&val, pos) in izip!(&*self.values, positions) {
            let bar = Rect {
                x0: pos - bar_width_2,
                y0: size.height * (1. - y_range.t(val)),
                x1: pos + bar_width_2,
                y1: zero,
            };
            rc.fill(bar, &self.bar_color.clone().with_alpha(0.8));
            rc.stroke(bar, &self.bar_color, 2.);
        }
    }

    fn as_any(&mut self) -> &mut dyn Any {
        self
    }
}
