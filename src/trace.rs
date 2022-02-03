use piet_common::{kurbo::Size, Error as PietError, Piet};
use std::any::Any;

/// A drawing that represents some data. Used inside the chart.
pub trait Trace: 'static {
    /// This function can be used to calculate things that depend on the size of the trace.
    fn layout(
        &mut self,
        #[allow(unused)] size: Size,
        #[allow(unused)] rc: &mut Piet,
    ) -> Result<(), PietError> {
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
    fn draw(&self, rc: &mut Piet);

    fn as_any(&mut self) -> &mut dyn Any;
}
