use piet::RenderContext;

/// A drawing that represents some data. Used inside the chart.
pub trait Trace {
    /// Draw the trace into the chart.
    ///
    /// The chart area will start at `(0, 0)` and finish at `(size.width, size.height)`.
    fn draw<RC: RenderContext>(&self, rc: &mut RC);
}
