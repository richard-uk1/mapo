//! Piet charts

//use druid::{kurbo::Rect, Color, Insets};
/*use piet::{
    kurbo::{Insets, Rect},
    Color,
};*/

pub mod axis;
pub mod prelude;
mod ticker;
//mod box_plot;
pub mod histogram;
//mod line_chart;
//mod pie_chart;
mod chart;
mod interval;
mod sequence;
pub mod theme;
mod trace;

pub use crate::{
    interval::{Interval, IntervalTicker},
    sequence::{Categorical, Numeric, Sequence, SequenceExt},
    ticker::{Tick, Ticker},
    trace::Trace,
};

type ArcStr = std::sync::Arc<str>;
/*
    box_plot::{BoxPlot, BoxPlotData, BoxPlotDataLens, BoxPlotDataLensBuilder},
    histogram::{Histogram, HistogramData, HistogramDataLens, HistogramDataLensBuilder},
    line_chart::{LineChart, LineChartData, LineChartDataLens, LineChartDataLensBuilder},
    pie_chart::{PieChart, PieChartData, PieChartDataLens, PieChartDataLensBuilder},
    theme::add_to_env,

const GRAPH_INSETS: Insets = Insets::new(-200.0, -100.0, -40.0, -60.0);

fn new_color(idx: usize) -> Color {
    let idx = idx as f64;
    // use a number that is fairly coprime with 360.
    Color::hlc(idx * 140.0, 50.0, 50.0)
}

/// Take a rect and shrink it to a square centered within the original rectangle.
fn square(input: Rect) -> Rect {
    let (width, height) = (input.width(), input.height());
    assert!(width >= 0.0 && height >= 0.0);
    if width == height {
        input
    } else if width < height {
        let half_overlap = 0.5 * (height - width);
        let y0 = input.y0 + half_overlap;
        let y1 = input.y1 - half_overlap;
        Rect::new(input.x0, y0, input.x1, y1)
    } else {
        let half_overlap = 0.5 * (width - height);
        let x0 = input.x0 + half_overlap;
        let x1 = input.x1 - half_overlap;
        Rect::new(x0, input.y0, x1, input.y1)
    }
}
*/
