use crate::{prelude::*, Chart, Interval, Trace};
use piet_common::{
    kurbo::{Circle, Size},
    Color, Error as PietError, Piet, RenderContext,
};
use std::{any::Any, sync::Arc};

pub struct Scatter {
    inner: Chart,
}

impl Scatter {
    pub fn new(values: impl Into<Arc<[(f64, f64)]>>) -> Self {
        let values = values.into();
        let (x_interval, y_interval): (Interval, Interval) = values.iter().copied().unzip();
        let (x_interval, y_interval) = (x_interval.to_rounded(), y_interval.to_rounded());
        let trace = ScatterTrace::new(values, x_interval, y_interval);
        Self {
            inner: Chart::new()
                .with_left_axis(y_interval.ticker().reverse())
                .with_left_grid(Default::default())
                .with_bottom_axis(x_interval.ticker())
                .with_bottom_grid(Default::default())
                .with_trace(trace),
        }
    }

    pub fn layout(&mut self, size: Size, rc: &mut Piet) -> Result<(), PietError> {
        self.inner.layout(size, rc)
    }

    pub fn draw(&self, rc: &mut Piet) {
        self.inner.draw(rc)
    }

    pub fn set_values(&mut self, new_values: impl Into<Arc<[(f64, f64)]>>) {
        let trace: &mut ScatterTrace = self.inner.traces_mut().next().unwrap();
        trace.set_values(new_values.into());
    }
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

    pub fn set_x_interval(&mut self, new_range: Interval) {
        self.x_range = new_range;
    }

    pub fn set_y_interval(&mut self, new_range: Interval) {
        self.y_range = new_range;
    }

    pub fn set_values(&mut self, new_values: Arc<[(f64, f64)]>) {
        self.values = new_values;
    }
}

impl Trace for ScatterTrace {
    fn layout(&mut self, size: Size, _rc: &mut Piet) -> Result<(), PietError> {
        if self.size == Some(size) {
            return Ok(());
        }
        self.size = Some(size);
        Ok(())
    }

    fn size(&self) -> Size {
        self.size.unwrap()
    }

    fn draw(&self, rc: &mut Piet) {
        let size = self.size.unwrap();
        for (x, y) in self.values.iter().copied() {
            let pos_x = self.x_range.t(x) * size.width;
            // The y position is reversed (because we want 0 at the bottom, not the top)
            let pos_y = (1. - self.y_range.t(y)) * size.height;
            let dot = Circle::new((pos_x, pos_y), 2.);
            rc.fill(dot, &self.point_color);
        }
    }

    fn as_any(&mut self) -> &mut dyn Any {
        self
    }
}
