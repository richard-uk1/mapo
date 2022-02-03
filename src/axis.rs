// TODO implement toPrecision from javascript - it gives better results.
use crate::{theme, ticker::Ticker};
use piet_common::{
    kurbo::{Line, Point, Rect, Size},
    Color, Error as PietError, Piet, PietTextLayout, RenderContext, Text, TextAttribute,
    TextLayout, TextLayoutBuilder,
};
use std::{fmt, ops::Deref};

const DEFAULT_LABEL_FONT_SIZE: f64 = 16.;

/// Denotes where the axis will be drawn, relative to the chart area.
///
/// This will affect the text direction of labels. You can use a `Direction::Left` axis vertically
/// by rotating it 90 degress, if this gives you the effect you want.
#[derive(Debug, Copy, Clone, PartialEq)]
pub enum LabelPosition {
    /// above or to the left
    Before,
    /// below or to the right
    After,
}

// # Plan
//
// A scale has ticks and labels. The implementation of a scale will supply all the ticks and labels
// (with the size in pixels as input). It will then be up to a wrapper to layout the labels and work
// out how many we can fit (and where they should actually be displayed). For now labels will be
// String only.

/// Axes must be drawn either vertically or horizontally.
///
/// To reverse the direction of the axis, reverse the ticker.
#[derive(Debug, Clone, Copy)]
pub enum Direction {
    Horizontal,
    Vertical,
}

/// A struct for retaining text layout information for an axis scale.
///
/// This struct knows everything it needs to draw the axis, ticks, and labels.
///
/// [matplotlib ticker](https://github.com/matplotlib/matplotlib/blob/master/lib/matplotlib/ticker.py#L2057)
/// is a good resource.
#[derive(Clone)]
pub struct Axis<T> {
    /// Whether the axis is vertical or horizontal.
    direction: Direction,
    /// Where the labels should be shown. Ticks will be drawn on the opposite side.
    label_pos: LabelPosition,
    /// An object that knows where the ticks should be drawn.
    ticker: T,

    // style

    // /// Axis/mark color
    label_font_size: f64,

    // retained
    is_layout_valid: bool,
    /// How long the axis will be
    axis_len: f64,
    /// Our computed text layouts for the tick labels.
    ///
    /// This is cached, and invalidated by clearing the vec. This way we
    /// can re-use the allocation. To see if cache is valid, check its
    /// length against `ticker.len()`.
    label_layouts: Vec<Label>,
    /// Which of the layouts we are actually going to draw.
    labels_to_draw: Vec<usize>,
}

impl<T: fmt::Debug> fmt::Debug for Axis<T> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_struct("Axis")
            .field("direction", &self.direction)
            .field("label_pos", &self.label_pos)
            .field("is_layout_valid", &self.is_layout_valid)
            .field("axis_len", &self.axis_len)
            .field("ticker", &self.ticker)
            .field("label_layouts", &self.label_layouts)
            .field("labels_to_draw", &self.labels_to_draw)
            .finish()
    }
}

impl<T: Ticker> Axis<T> {
    /// Create a new axis.
    pub fn new(direction: Direction, label_pos: LabelPosition, ticker: T) -> Self {
        Self {
            direction,
            label_pos,
            ticker,
            label_font_size: DEFAULT_LABEL_FONT_SIZE,

            is_layout_valid: false,
            axis_len: 0.,
            label_layouts: vec![],
            labels_to_draw: vec![],
        }
    }

    pub fn ticker(&self) -> &T {
        &self.ticker
    }

    pub fn set_ticker(&mut self, new_ticker: T) {
        self.ticker = new_ticker;
        self.is_layout_valid = false;
    }

    pub fn size(&self) -> Size {
        self.assert_layout();

        // Find the maximum width and height of any label
        let max_label_size = self.labels_to_draw().fold(Size::ZERO, |mut size, layout| {
            let run_size = layout.rect().size();
            if run_size.width > size.width {
                size.width = run_size.width;
            }
            if run_size.height > size.height {
                size.height = run_size.height;
            }
            size
        });

        match self.direction {
            Direction::Horizontal => {
                Size::new(self.axis_len, max_label_size.height + theme::MARGIN)
            }
            Direction::Vertical => Size::new(max_label_size.width + theme::MARGIN, self.axis_len),
        }
    }

    /// Call this before draw.
    pub fn layout(&mut self, axis_len: f64, rc: &mut Piet) -> Result<(), PietError> {
        self.is_layout_valid = true;
        self.axis_len = axis_len;
        self.ticker.layout(axis_len);
        self.build_label_layouts(rc)?;
        self.fit_labels();
        Ok(())
    }

    /// Draw the layout
    pub fn draw(&self, rc: &mut Piet) {
        let Size { width, height } = self.size();

        // ticks
        for tick in self.ticker.ticks() {
            let tick_line = match (self.direction, self.label_pos) {
                (Direction::Vertical, LabelPosition::Before) => {
                    // left
                    Line::new((width - 5., tick.pos), (width, tick.pos))
                }
                (Direction::Vertical, LabelPosition::After) => {
                    // right
                    Line::new((0., tick.pos), (5., tick.pos))
                }
                (Direction::Horizontal, LabelPosition::Before) => {
                    // above
                    Line::new((tick.pos, height - 5.), (tick.pos, height))
                }
                (Direction::Horizontal, LabelPosition::After) => {
                    // below
                    Line::new((tick.pos, 0.), (tick.pos, 5.))
                }
            };
            rc.stroke(tick_line, &Color::grey8(80), 1.);
        }

        // axis line (extend to contain tick at edge)
        let axis_line = match (self.direction, self.label_pos) {
            (Direction::Horizontal, LabelPosition::Before) => {
                Line::new((-1., height), (width + 1., height))
            }
            (Direction::Horizontal, LabelPosition::After) => Line::new((-1., 0.), (width + 1., 0.)),
            (Direction::Vertical, LabelPosition::Before) => {
                Line::new((width, -1.), (width, height + 1.))
            }
            (Direction::Vertical, LabelPosition::After) => Line::new((0., -1.), (0., height + 1.)),
        };
        rc.stroke(axis_line, &Color::BLACK, 2.);

        // labels
        for label in self.labels_to_draw() {
            rc.draw_text(&label.layout, label.pos);
        }
    }

    fn build_label_layouts(&mut self, rc: &mut Piet) -> Result<(), PietError> {
        self.assert_layout();
        self.label_layouts.clear();

        if self.ticker.len() == 0 {
            // nothing to do
            return Ok(());
        }

        let text = rc.text();
        // 2 passes - one to create the layouts and find the largest layout size, second to
        // position the text.
        let mut largest = Size::ZERO;
        for tick in self.ticker.ticks() {
            let layout = text
                .new_text_layout(tick.label)
                .default_attribute(TextAttribute::FontSize(self.label_font_size))
                .build()?;
            let size = layout.size();
            if size.width > largest.width {
                largest.width = size.width;
            }
            if size.height > largest.height {
                largest.height = size.height;
            }
            self.label_layouts.push(Label {
                layout,
                pos: Point::ZERO,
            });
        }

        // 2nd pass to position labels
        for (pos, label) in self
            .ticker
            .ticks()
            .map(|tick| tick.pos)
            .zip(self.label_layouts.iter_mut())
        {
            let size = label.layout.size();

            let pos = match self.direction {
                Direction::Horizontal => {
                    let x = pos - size.width * 0.5;
                    let y = match self.label_pos {
                        // TODO assume all line-heights are the same for now
                        LabelPosition::Before => 0.,
                        LabelPosition::After => theme::MARGIN,
                    };
                    Point { x, y }
                }
                Direction::Vertical => {
                    let x = match self.label_pos {
                        // right-align
                        LabelPosition::Before => largest.width - size.width,
                        // left-align
                        LabelPosition::After => theme::MARGIN,
                    };
                    let y = pos - size.height * 0.5;
                    Point { x, y }
                }
            };
            label.pos = pos;
        }
        Ok(())
    }

    /// This function needs to be called every time anything affecting label
    /// positioning changes.
    fn fit_labels(&mut self) {
        // Start by trying to fit in all labels, then keep missing more out until
        // they will fit
        let mut step = 1;
        // the loop will never run iff `self.label_layouts.len() == 0`. The below
        // divides by 2, rounding up.
        while step <= (self.label_layouts.len() + 1) / 2 {
            self.labels_to_draw.clear();
            // TODO if the remainder is odd, put the gap in the middle, if even, split
            // it between the ends.
            self.labels_to_draw
                .extend((0..self.label_layouts.len()).step_by(step));
            if !self.do_layouts_overlap() {
                return;
            }
            step += 1;
        }
        // If we can't layout anything, then show nothing.
        println!("can't layout anything");
        self.labels_to_draw.clear();
    }

    /// Iterate over only those labels we will be drawing.
    fn labels_to_draw(&self) -> impl Iterator<Item = &'_ Label> {
        self.labels_to_draw
            .iter()
            .copied()
            .map(|idx| &self.label_layouts[idx])
    }

    /// Returns `true` if all the labels selected for drawing will fit without overlapping
    /// each other.
    ///
    /// # Panics
    ///
    /// Panics if the label layouts have not been built.
    fn do_layouts_overlap(&self) -> bool {
        #[cfg(debug_assertions)]
        self.assert_layout();
        // strictly speaking the positions of the labels are different depending on whether we are
        // before or after the axis, but the relative positions don't change, so we can combine
        // them.
        //
        // It is sufficient to test each label with the one preceeding it.
        let mut prev_rect: Option<Rect> = None;

        for label in self.labels_to_draw() {
            let rect = label.rect();
            if let Some(prev_rect) = prev_rect {
                if !prev_rect.intersect(rect).is_empty() {
                    return true;
                }
            }
            prev_rect = Some(rect);
        }
        false
    }

    fn assert_layout(&self) {
        if !self.is_layout_valid {
            panic!("layout not called");
        }
    }
}

/// The label's text layout with position information
#[derive(Clone)]
struct Label {
    pos: Point,
    layout: PietTextLayout,
}

impl Label {
    pub fn rect(&self) -> Rect {
        Rect::from_origin_size(self.pos, self.layout.size())
    }
}

impl Deref for Label {
    type Target = PietTextLayout;
    fn deref(&self) -> &Self::Target {
        &self.layout
    }
}

impl fmt::Debug for Label {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_struct("Label")
            .field("text", &self.layout.text())
            .field("area", &self.rect())
            .finish()
    }
}
