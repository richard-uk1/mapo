use std::{any::Any, fmt, sync::Arc};

/// A position on an axis that we can mark and label.
#[derive(Debug, Clone)]
pub struct Tick {
    /// The distance along the axis that the tick should be displayed, in `0..=axis_len`
    pub pos: f64,
    /// The label that should be displayed
    pub label: Arc<str>,
}

/// Object that implement can take an axis length (in device-independent pixels) and return a set
/// of marks (a.k.a. ticks) that mark significant points along the axis.
pub trait Ticker: fmt::Debug {
    /// Used during layout to calculate retained state based on axis size.
    ///
    /// Other methods are allowed to panic before this method is called.
    fn layout(&mut self, axis_len: f64);

    /// How many ticks are we going to draw.
    fn len(&self) -> usize;

    fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// An iterator over the ticks that should be drawn, with their position on the axis.
    fn ticks(&self) -> Box<dyn Iterator<Item = Tick> + '_> {
        Box::new((0..self.len()).map(|idx| self.get(idx).unwrap()))
    }

    /// Get the `idx`th tick
    ///
    /// This should return `Some` if `idx < Ticker::len(self)`, `None` otherwise.
    fn get(&self, idx: usize) -> Option<Tick>;

    fn as_any(&self) -> &dyn Any
    where
        Self: 'static;
}

pub trait TickerExt: Ticker {
    fn reverse(self) -> ReverseTicker<Self>
    where
        Self: Sized,
    {
        ReverseTicker {
            ticker: self,
            axis_len: None,
        }
    }
}

impl<T: Ticker> TickerExt for T {}

impl Ticker for Box<dyn Ticker> {
    fn layout(&mut self, axis_len: f64) {
        (**self).layout(axis_len)
    }

    fn len(&self) -> usize {
        (**self).len()
    }

    fn ticks(&self) -> Box<dyn Iterator<Item = Tick> + '_> {
        (**self).ticks()
    }

    fn get(&self, idx: usize) -> Option<Tick> {
        (**self).get(idx)
    }

    fn as_any(&self) -> &dyn Any {
        (**self).as_any()
    }
}

#[derive(Debug)]
pub struct ReverseTicker<T> {
    ticker: T,
    axis_len: Option<f64>,
}

impl<T: Ticker> Ticker for ReverseTicker<T> {
    fn layout(&mut self, axis_len: f64) {
        self.ticker.layout(axis_len);
        self.axis_len = Some(axis_len);
    }

    fn len(&self) -> usize {
        self.ticker.len()
    }

    fn ticks(&self) -> Box<dyn Iterator<Item = Tick> + '_> {
        Box::new((0..self.len()).map(move |idx| self.get(idx).unwrap()))
    }

    fn get(&self, idx: usize) -> Option<Tick> {
        let tick = self.ticker.get(idx)?;
        Some(Tick {
            label: tick.label,
            pos: self.axis_len.expect("format not called") - tick.pos,
        })
    }

    fn as_any(&self) -> &dyn Any
    where
        T: 'static,
    {
        self
    }
}
