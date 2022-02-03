use crate::{
    axis::{Axis, Direction, LabelPosition},
    theme, Ticker, Trace,
};
use piet_common::{
    kurbo::{Affine, Line, Point, Rect, Size},
    Color, Error as PietError, Piet, RenderContext,
};
use std::any::Any;

/// A chart.
///
/// # Type parameters
///  - `RC`: the piet render context. This is used to create text layouts.
pub struct Chart {
    /// An optional axis above the chart.
    top_axis: Option<Axis<Box<dyn Ticker>>>,
    top_grid: Option<GridStyle>,
    /// An optional axis below the chart.
    bottom_axis: Option<Axis<Box<dyn Ticker>>>,
    bottom_grid: Option<GridStyle>,
    /// An optional axis left of the chart.
    left_axis: Option<Axis<Box<dyn Ticker>>>,
    left_grid: Option<GridStyle>,
    /// An optional axis right of the chart.
    right_axis: Option<Axis<Box<dyn Ticker>>>,
    right_grid: Option<GridStyle>,
    /// Histogram trace
    traces: Vec<Box<dyn Trace>>,

    // Retained
    /// The size that everything should fit in (inc. axes).
    size: Option<Size>,
    /// The chart area.
    ///
    /// Only valid after call to `layout`.
    chart_area: Option<Rect>,
}

impl Chart {
    pub fn new() -> Self {
        Chart {
            top_axis: None,
            top_grid: None,
            bottom_axis: None,
            bottom_grid: None,
            left_axis: None,
            left_grid: None,
            right_axis: None,
            right_grid: None,
            traces: vec![],
            size: None,
            chart_area: None,
        }
    }

    pub fn with_top_axis(mut self, ticker: impl Ticker + 'static) -> Self {
        let axis = Axis::new(
            Direction::Horizontal,
            LabelPosition::Before,
            Box::new(ticker) as Box<dyn Ticker>,
        );
        self.top_axis = Some(axis);
        self
    }

    pub fn with_top_grid(mut self, style: GridStyle) -> Self {
        self.top_grid = Some(style);
        self
    }

    pub fn set_top_grid(&mut self, style: GridStyle) -> &mut Self {
        self.top_grid = Some(style);
        self
    }

    pub fn with_bottom_axis(mut self, ticker: impl Ticker + 'static) -> Self {
        let axis = Axis::new(
            Direction::Horizontal,
            LabelPosition::After,
            Box::new(ticker) as Box<dyn Ticker>,
        );
        self.bottom_axis = Some(axis);
        self
    }

    pub fn with_bottom_grid(mut self, style: GridStyle) -> Self {
        self.bottom_grid = Some(style);
        self
    }

    pub fn set_bottom_grid(&mut self, style: GridStyle) -> &Self {
        self.bottom_grid = Some(style);
        self
    }

    pub fn with_left_axis(mut self, ticker: impl Ticker + 'static) -> Self {
        let axis = Axis::new(
            Direction::Vertical,
            LabelPosition::Before,
            Box::new(ticker) as Box<dyn Ticker>,
        );
        self.left_axis = Some(axis);
        self
    }

    pub fn with_left_grid(mut self, style: GridStyle) -> Self {
        self.left_grid = Some(style);
        self
    }

    pub fn set_left_grid(&mut self, style: GridStyle) -> &mut Self {
        self.left_grid = Some(style);
        self
    }

    pub fn with_right_axis(mut self, ticker: impl Ticker + 'static) -> Self {
        let axis = Axis::new(
            Direction::Vertical,
            LabelPosition::After,
            Box::new(ticker) as Box<dyn Ticker>,
        );
        self.right_axis = Some(axis);
        self
    }

    pub fn with_right_grid(mut self, style: GridStyle) -> Self {
        self.right_grid = Some(style);
        self
    }

    pub fn set_right_grid(&mut self, style: GridStyle) -> &mut Self {
        self.right_grid = Some(style);
        self
    }

    pub fn with_trace(mut self, trace: impl Trace + 'static) -> Self {
        self.traces.push(Box::new(trace));
        self
    }

    pub fn traces_mut<T: Trace>(&mut self) -> impl Iterator<Item = &mut T> {
        self.traces
            .iter_mut()
            .filter_map(|trace| trace.as_any().downcast_mut())
    }

    /// # Panics
    ///
    /// Will panic if `layout` has not been called.
    pub fn size(&self) -> Size {
        self.size.unwrap()
    }

    /// Lay out the axes and calculate the chart area available.
    ///
    /// Once the chart area has been calculated, each trace will have its `layout` method called.
    ///
    /// This function must be called before `draw`, both after creation and after anything changes.
    pub fn layout(&mut self, size: Size, rc: &mut Piet) -> Result<(), PietError> {
        self.size = Some(size);
        // Loop until our layout fits.
        // The initial guess is the whole area (we know this will be too big, bug it gives a first
        // estimate for the axis sizes.
        let mut chart_size = size;
        // We abuse labelled loops so we can run some code if the for loop finishes before a
        // solution has been found.
        'found_height: loop {
            // We expect this loop to complete after 2 loops.
            for _ in 0..10 {
                // Lay out the axes at the current size.
                self.layout_axes(chart_size, rc)?;
                // This size contains the space we need for the axes
                let axis_size = self.axis_size();
                if axis_size.height + chart_size.height < size.height
                    && axis_size.width + chart_size.width < size.width
                {
                    // we've found a valid chart size
                    break 'found_height;
                }
                // Chart size is still too big, try shrinking it to what would have fit with the
                // current axes, minus a small delta to try to take fp accuracy out of the equation.
                chart_size.height = size.height - axis_size.height - 1e-8;
                chart_size.width = size.width - axis_size.width - 1e-8;
            }
            // We didn't find a solution, so warn and just draw as best we can
            // TODO make a log msg
            eprintln!("We didn't find a valid chart size, so the chart may overflow");
            chart_size *= 0.9;
            self.layout_axes(chart_size, rc)?;
            break;
        }

        let chart_tl = Point::new(
            self.left_axis
                .as_ref()
                .map(|axis| axis.size().width)
                .unwrap_or(0.),
            self.top_axis
                .as_ref()
                .map(|axis| axis.size().height)
                .unwrap_or(0.),
        );
        self.chart_area = Some(Rect::from_origin_size(chart_tl, chart_size));
        for trace in &mut self.traces {
            trace.layout(chart_size, rc)?;
        }

        //println!("{:#?}", self.left_axis);
        //println!("{:#?}", self.bottom_axis);
        Ok(())
    }

    /// Lays out the axes for a given chart size.
    fn layout_axes(&mut self, chart_size: Size, rc: &mut Piet) -> Result<(), PietError> {
        if let Some(axis) = &mut self.top_axis {
            axis.layout(chart_size.width, rc)?;
        }
        if let Some(axis) = &mut self.bottom_axis {
            axis.layout(chart_size.width, rc)?;
        }
        if let Some(axis) = &mut self.left_axis {
            axis.layout(chart_size.height, rc)?;
        }
        if let Some(axis) = &mut self.right_axis {
            axis.layout(chart_size.height, rc)?;
        }
        Ok(())
    }

    fn axis_size(&self) -> Size {
        Size {
            width: self
                .left_axis
                .as_ref()
                .map(|axis| axis.size().width)
                .unwrap_or(0.)
                + self
                    .right_axis
                    .as_ref()
                    .map(|axis| axis.size().width)
                    .unwrap_or(0.),
            height: self
                .top_axis
                .as_ref()
                .map(|axis| axis.size().height)
                .unwrap_or(0.)
                + self
                    .bottom_axis
                    .as_ref()
                    .map(|axis| axis.size().height)
                    .unwrap_or(0.),
        }
    }

    /// Draw the histogram at (0,0).
    ///
    /// # Panics
    ///
    /// Panics if `layout` was not called.
    pub fn draw(&self, rc: &mut Piet) {
        //self.draw_grid(rc);
        let chart_area = self.chart_area.unwrap();

        // Draw gridlines
        self.draw_grid(chart_area, rc);

        // draw the chart data first, so the axes are on top
        rc.with_save(|rc| {
            rc.transform(Affine::translate(chart_area.origin().to_vec2()));
            for trace in &self.traces {
                trace.draw(rc);
            }
            Ok(())
        })
        .unwrap();
        // now draw axes
        // top
        if let Some(axis) = self.top_axis.as_ref() {
            rc.with_save(|rc| {
                rc.transform(Affine::translate((chart_area.x0, 0.)));
                axis.draw(rc);
                Ok(())
            })
            .unwrap();
        }
        // bottom
        if let Some(axis) = self.bottom_axis.as_ref() {
            rc.with_save(|rc| {
                rc.transform(Affine::translate((chart_area.x0, chart_area.y1)));
                axis.draw(rc);
                Ok(())
            })
            .unwrap();
        }
        // left
        if let Some(axis) = self.left_axis.as_ref() {
            rc.with_save(|rc| {
                rc.transform(Affine::translate((0., chart_area.y0)));
                axis.draw(rc);
                Ok(())
            })
            .unwrap();
        }
        // right
        if let Some(axis) = self.right_axis.as_ref() {
            rc.with_save(|rc| {
                rc.transform(Affine::translate((chart_area.x1, chart_area.y0)));
                axis.draw(rc);
                Ok(())
            })
            .unwrap();
        }
    }

    /// Draw on the gridlines.
    fn draw_grid(&self, chart_area: Rect, rc: &mut Piet) {
        // left
        if let (Some(axis), Some(style)) = (&self.left_axis, &self.left_grid) {
            for tick in axis.ticker().ticks() {
                let pos = tick.pos + chart_area.y0;
                rc.stroke(
                    Line::new((chart_area.x0, pos), (chart_area.x1, pos)),
                    &style.color,
                    style.stroke_width,
                );
            }
        }
        // right
        if let (Some(axis), Some(style)) = (&self.right_axis, &self.right_grid) {
            for tick in axis.ticker().ticks() {
                let pos = tick.pos + chart_area.y0;
                rc.stroke(
                    Line::new((chart_area.x0, pos), (chart_area.x1, pos)),
                    &style.color,
                    style.stroke_width,
                );
            }
        }
        // top
        if let (Some(axis), Some(style)) = (&self.top_axis, &self.top_grid) {
            for tick in axis.ticker().ticks() {
                let pos = tick.pos + chart_area.x0;
                rc.stroke(
                    Line::new((pos, chart_area.y0), (pos, chart_area.y1)),
                    &style.color,
                    style.stroke_width,
                );
            }
        }
        // bottom
        if let (Some(axis), Some(style)) = (&self.bottom_axis, &self.bottom_grid) {
            for tick in axis.ticker().ticks() {
                let pos = tick.pos + chart_area.x0;
                rc.stroke(
                    Line::new((pos, chart_area.y0), (pos, chart_area.y1)),
                    &style.color,
                    style.stroke_width,
                );
            }
        }
    }
}

impl Default for Chart {
    fn default() -> Self {
        Self::new()
    }
}

pub struct GridStyle {
    pub stroke_width: f64,
    pub color: Color,
}

impl Default for GridStyle {
    fn default() -> Self {
        Self {
            stroke_width: 1.,
            color: theme::GRID_COLOR,
        }
    }
}
