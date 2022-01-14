use piet::{kurbo::Size, RenderContext};

/// A drawing that represents some data. Used inside the chart.
pub trait Trace {
    /// This function can be used to calculate things that depend on the size of the trace.
    fn layout<RC: RenderContext>(
        &mut self,
        #[allow(unused)] size: Size,
        #[allow(unused)] rc: &mut RC,
    ) -> Result<(), piet::Error> {
        Ok(())
    }

    /// Returns the size that this trace was laid out for.
    ///
    /// This function is allowed to panic if `layout` hasn't been called.
    fn size(&self) -> Size;

    /// Draw the trace into the chart.
    ///
    /// The chart area will start at `(0, 0)` and finish at
    /// `(self.size().width, self.size().height)`.
    fn draw<RC: RenderContext>(&self, rc: &mut RC);
}
