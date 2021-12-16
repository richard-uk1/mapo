use crate::{
    axis::{Axis, Direction, LabelPosition},
    prelude::*,
    sequence::{Sequence, SpaceAround},
    theme,
    ticker::Ticker,
    Categorical, Interval, IntervalTicker,
};
use itertools::izip;
use piet::{
    kurbo::{Affine, Insets, Line, Point, Rect, Size, Vec2},
    Color, RenderContext,
};
use std::{fmt, sync::Arc};

/// A histogram
///
/// # Type parameters
///  - `CT`: a ticker of categories
///  - `VT`: an (optional) ticker of values
///  - `RC`: the piet render context. This is used to create text layouts.
pub struct Histogram<S, RC: RenderContext> {
    values: Arc<[f64]>,
    /// for now always x axis
    category_axis: Axis<SpaceAround<S>, RC>,
    /// for now always y axis
    value_axis: Axis<IntervalTicker, RC>,
    /// The size of the chart area. Does not include axis labels etc.
    chart_size: Size,
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
        let mut out = Histogram {
            values,
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
            bar_spacing: theme::BAR_SPACING,
            bar_color: theme::BAR_COLOR,
        };
        out.clamp_spacing();
        out
    }

    pub fn with_bar_spacing(mut self, bar_spacing: f64) -> Self {
        self.set_bar_spacing(bar_spacing);
        self
    }

    pub fn set_bar_spacing(&mut self, bar_spacing: f64) -> &mut Self {
        self.bar_spacing = bar_spacing;
        self.clamp_spacing();
        self
    }

    pub fn values(&self) -> &[f64] {
        &self.values[..]
    }

    /// This function returns the amount of extra space required to draw labels/axes/etc.
    ///
    /// # Panics
    ///
    /// This function will panic if `layout` has not been called.
    pub fn insets(&self) -> Insets {
        let value_axis_size = self.value_axis.size();
        let category_axis_size = self.category_axis.size();
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
            self.draw_grid(rc);
            self.draw_bars(rc);
            self.category_axis.draw((0., self.chart_size.height), rc);
            self.value_axis.draw(Point::ZERO, rc);
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
        self.draw_at(Point::ZERO, rc)
    }

    /// Draw on the bars that represent the data.
    fn draw_bars(&self, rc: &mut RC) {
        let bar_width_2 = (self.bar_slot_width() - self.bar_spacing) * 0.5;
        let val_max = self.value_axis.ticker().interval().max();
        for (val, tick) in self.values.iter().copied().zip(self.category_axis.ticks()) {
            let bar = Rect {
                x0: tick.pos - bar_width_2,
                y0: self.chart_size.height * (1. - val / val_max),
                x1: tick.pos + bar_width_2,
                y1: self.chart_size.height,
            };
            rc.fill(bar, &self.bar_color.clone().with_alpha(0.8));
            rc.stroke(bar, &self.bar_color, 2.);
        }
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

    /// Ensure that the spacing is set to something sensible.
    fn clamp_spacing(&mut self) {
        let slot_width = self.bar_slot_width();
        if self.bar_spacing > slot_width - 5. {
            self.bar_spacing = slot_width - 5.;
        }
        if self.bar_spacing < 0. {
            self.bar_spacing = 0.;
        }
    }

    fn bar_slot_width(&self) -> f64 {
        self.chart_size.width / self.values.len() as f64
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
