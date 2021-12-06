use crate::{
    axis::{Axis, Direction, LabelPosition},
    range::Range,
    sequence::Categorical,
    theme,
};
use itertools::izip;
use piet::{kurbo::Size, RenderContext};
use std::{fmt, sync::Arc};

/// A histogram
///
/// # Type parameters
///  - `CT`: a ticker of categories
///  - `VT`: an (optional) ticker of values
///  - `RC`: the piet render context. This is used to create text layouts.
pub struct Histogram<C, RC: RenderContext> {
    values: Arc<[f64]>,
    /// for now always x axis
    category_axis: Axis<Categorical<C>, RC>,
    /// for now always y axis
    value_axis: Axis<Range, RC>,
    /// The size of the chart area. Does not include axis labels etc.
    chart_size: Size,
    /// The gap between barlines.
    ///
    /// Will be clamped to `(0, (bar_width - 5.))`, or `0` if that range is empty.
    bar_spacing: f64,
}

impl<C: 'static + fmt::Display, RC: RenderContext> Histogram<C, RC> {
    pub fn new(
        chart_size: Size,
        labels: impl Into<Categorical<C>>,
        values: impl Into<Arc<[f64]>>,
    ) -> Self {
        let labels = labels.into();
        let values = values.into();
        let values_range = Range::from_iter(values.iter().copied()).include_zero();
        let mut out = Histogram {
            values,
            category_axis: Axis::new(
                Direction::Horizontal,
                LabelPosition::After,
                chart_size.width,
                labels,
            ),
            value_axis: Axis::new(
                Direction::Vertical,
                LabelPosition::Before,
                chart_size.height,
                values_range,
            ),
            chart_size,
            bar_spacing: 10.,
        };
        out.clamp_spacing();
        out
    }

    pub fn values(&self) -> &[f64] {
        &self.values[..]
    }

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

    pub fn layout(&mut self, rc: &mut RC) -> Result<(), piet::Error> {
        self.category_axis.layout(rc)?;
        self.value_axis.layout(rc)?;
        Ok(())
    }

    pub fn draw(&self, rc: &mut RC) {
        self.category_axis.draw((0., self.chart_size.height), rc);
    }
}
/*

impl<Title, XLabel> Widget<HistogramData<Title, XLabel>> for Histogram<Title, XLabel>
where
    Title: TextStorage,
    XLabel: TextStorage,
{
    fn event(
        &mut self,
        ctx: &mut EventCtx,
        event: &Event,
        data: &mut HistogramData<Title, XLabel>,
        env: &Env,
    ) {
    }

    fn lifecycle(
        &mut self,
        ctx: &mut LifeCycleCtx,
        event: &LifeCycle,
        data: &HistogramData<Title, XLabel>,
        env: &Env,
    ) {
        match event {
            LifeCycle::WidgetAdded => {
                self.title_layout.set_text(data.title.clone());
                self.x_label_layout.set_text(data.x_axis_label.clone());
            }
            _ => (),
        }
    }

    fn update(
        &mut self,
        ctx: &mut UpdateCtx,
        old_data: &HistogramData<Title, XLabel>,
        data: &HistogramData<Title, XLabel>,
        env: &Env,
    ) {
        if !old_data.title.same(&data.title) {
            self.title_layout.set_text(data.title.clone());
        }
        if !old_data.x_axis_label.same(&data.x_axis_label) {
            self.x_label_layout.set_text(data.x_axis_label.clone());
        }
        if !old_data.x_axis.same(&data.x_axis) {
            self.x_axis_layouts = None;
            ctx.request_layout();
        }
        if ctx.env_key_changed(&theme::MARGIN)
            || ctx.env_key_changed(&theme::SCALE_MARGIN)
            || self.x_label_layout.needs_rebuild_after_update(ctx)
            || self.title_layout.needs_rebuild_after_update(ctx)
        {
            ctx.request_layout();
        }
    }

    fn layout(
        &mut self,
        ctx: &mut LayoutCtx,
        bc: &BoxConstraints,
        data: &HistogramData<Title, XLabel>,
        env: &Env,
    ) -> Size {
        let size = bc.max();
        self.rebuild_if_needed(ctx.text(), data, env, size);
        size
    }

    fn paint(&mut self, ctx: &mut PaintCtx, data: &HistogramData<Title, XLabel>, env: &Env) {
        let bg_brush = ctx.solid_brush(Color::hlc(0.0, 90.0, 0.0));
        let axes_brush = ctx.solid_brush(self.axis_color.resolve(env));
        let bar_brush = ctx.solid_brush(Color::hlc(0.0, 50.0, 50.0));
        let size = ctx.size();
        let bounds = size.to_rect();
        let graph_bounds = bounds.inset(GRAPH_INSETS);
        let max_data = *data.counts.iter().max().unwrap() as f64;
        let bar_spacing = self.bar_spacing.resolve(env);

        // data
        let data_len = data.counts.len() as f64;
        let (width, height) = (graph_bounds.width(), graph_bounds.height());
        let total_space = (data_len + 1.0) * bar_spacing;
        // give up if the area is too small.
        if total_space >= width {
            return;
        }
        let total_bar_width = width - total_space;
        let bar_width = total_bar_width / data_len;
        assert_eq!(bar_width * data_len + bar_spacing * (data_len + 1.0), width);
        ctx.with_save(|ctx| {
            ctx.transform(Affine::translate((
                graph_bounds.x0 + bar_spacing,
                graph_bounds.y0,
            )));
            for (idx, (count, label, label_layout)) in izip!(
                data.counts.iter().copied(),
                data.x_axis.iter().cloned(),
                self.x_axis_layouts.as_ref().unwrap()
            )
            .enumerate()
            {
                let idx = idx as f64;
                let start_x = width * idx / data_len;
                let end_x = start_x + bar_width;
                let mid_x = start_x + (end_x - start_x) * 0.5;

                // bar
                let end_y = (count as f64) * height / max_data;
                ctx.fill(
                    Rect::new(start_x, height - end_y, end_x, height),
                    &bar_brush,
                );

                // data label
                let label_width = label_layout.size().width;
                label_layout.draw(ctx, (mid_x - label_width * 0.5, height + 2.));
            }
        });

        // title
        let title_width = self.title_layout.size().width;
        self.title_layout
            .draw(ctx, ((size.width - title_width) * 0.5, 10.0));

        // x axis
        let x_axis = Line::new(
            (graph_bounds.x0 - 1.0, graph_bounds.y1),
            (graph_bounds.x1, graph_bounds.y1),
        );
        ctx.stroke(x_axis, &axes_brush, 2.0);
        let x_label_width = self.x_label_layout.size().width;
        self.x_label_layout.draw(
            ctx,
            ((size.width - x_label_width) * 0.5, size.height - 40.0),
        );

        // y axis
        self.y_scale.as_mut().unwrap().draw(ctx, env, true, true);
    }
}
*/
