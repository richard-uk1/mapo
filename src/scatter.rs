use crate::{prelude::*, Chart, Interval, IntervalTicker, Trace};
use piet::{
    kurbo::{Circle, Size},
    Color, RenderContext,
};
use std::sync::Arc;

pub fn scatter<RC: RenderContext>(values: impl Into<Arc<[(f64, f64)]>>) -> Chart<RC> {
    let values = values.into();
    let (x_interval, y_interval): (Interval, Interval) = values.iter().copied().unzip();
    let (x_interval, y_interval) = (x_interval.to_rounded(), y_interval.to_rounded());
    let trace = ScatterTrace::new(values, x_interval, y_interval);
    Chart::new()
        .with_left_axis(y_interval.ticker().reverse())
        .with_left_grid(Default::default())
        .with_bottom_axis(x_interval.ticker())
        .with_bottom_grid(Default::default())
        .with_trace(trace)
}

/// How to draw the bars of the scatter.
pub struct ScatterTrace {
    /// The values of the bars.
    ///
    /// Not public because we have retained state that depends on them.
    values: Arc<[(f64, f64)]>,
    /// The range that x values should be shown over
    x_range: Interval,
    /// The range that y values should be shown over
    y_range: Interval,
    /// Point color TODO make this more customizable (e.g. custom renderer)
    point_color: Color,

    // Retained
    /// The size of the chart area.
    pub size: Option<Size>,
    // /// The positions of the center of each point on the scatter.
    //positions: Option<Arc<[(f64, f64)]>>,
}

impl ScatterTrace {
    /// A scatter trace
    pub fn new(values: impl Into<Arc<[(f64, f64)]>>, x_range: Interval, y_range: Interval) -> Self {
        ScatterTrace {
            x_range,
            y_range,
            point_color: Color::BLUE.with_alpha(0.4),

            size: None,
            values: values.into(),
            //positions: None,
        }
    }

    /// Get the numeric values of the bars in this scatter.
    pub fn values(&self) -> &[(f64, f64)] {
        &self.values
    }
}

impl<RC: RenderContext> Trace<RC> for ScatterTrace {
    fn layout(&mut self, size: Size, _rc: &mut RC) -> Result<(), piet::Error> {
        if self.size == Some(size) {
            return Ok(());
        }
        self.size = Some(size);
        Ok(())
    }

    fn size(&self) -> Size {
        self.size.unwrap()
    }

    fn draw(&self, rc: &mut RC) {
        let size = self.size.unwrap();
        for (x, y) in self.values.iter().copied() {
            let pos_x = self.x_range.t(x) * size.width;
            let pos_y = self.y_range.t(y) * size.height;
            let dot = Circle::new((pos_x, pos_y), 2.);
            rc.fill(dot, &self.point_color);
        }
    }
}
