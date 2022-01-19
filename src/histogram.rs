use crate::{prelude::*, theme, Categorical, Chart, GridStyle, Interval, IntervalTicker, Trace};
use itertools::izip;
use piet::{
    kurbo::{Rect, Size},
    Color, RenderContext,
};
use std::{f64::consts::FRAC_2_PI, fmt, sync::Arc};

pub fn histogram<L, RC: RenderContext>(
    labels: impl Into<Categorical<L>>,
    values: impl Into<Arc<[f64]>>,
) -> Chart<RC>
where
    L: Clone + fmt::Debug + fmt::Display + 'static,
{
    let labels = labels.into();
    let values = values.into();
    let values_interval = Interval::from_iter(values.iter().copied())
        .include_zero()
        .to_rounded();
    let bars_trace = HistogramTrace::new(values).with_y_max(values_interval.max());
    Chart::new()
        .with_left_axis(values_interval.ticker().reverse())
        .with_left_grid(GridStyle::default())
        .with_bottom_axis(labels.space_around_ticker())
        .with_trace(bars_trace)
}
/// Create a histogram from `(label, frequency)` pairs.
pub fn histogram_from_pairs<L, RC>(data: impl IntoIterator<Item = (L, f64)>) -> Chart<RC>
where
    RC: RenderContext,
    L: fmt::Display + fmt::Debug + Clone + 'static,
{
    let data = data.into_iter();
    let mut labels = Vec::with_capacity(data.size_hint().0);
    let mut values = Vec::with_capacity(data.size_hint().0);
    for (label, value) in data {
        labels.push(label);
        values.push(value);
    }
    histogram(labels, values)
}

/// How to draw the bars of the histogram.
pub struct HistogramTrace {
    /// The values of the bars.
    ///
    /// Not public because we have retained state that depends on them.
    values: Arc<[f64]>,
    /// The maximum value to use for y
    ///
    /// The maximum value in `values` would be a sensible choice.
    y_max: Option<f64>,
    /// The width of each bar.
    pub bar_width: Option<f64>,
    /// The color to draw the bars.
    pub bar_color: Color,

    // Retained
    /// The size of the chart area.
    pub size: Option<Size>,
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
            bar_width: None,
            bar_color: theme::BAR_COLOR,
            values,
            y_max: None,
            size: None,
            positions: None,
            full_value: None,
        }
    }

    /// Sets the maximum value of the y axis.
    ///
    /// Defaults to the largest value in `values`.
    pub fn with_y_max(mut self, y_max: f64) -> Self {
        self.y_max = Some(y_max);
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
        let full_value = self.y_max.unwrap_or(self.full_value.unwrap());
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
