use mapo::{
    axis::{Axis, Direction, LabelPosition},
    histogram::Histogram,
    prelude::*,
    Categorical,
};
use piet::{
    kurbo::{Affine, Point, Rect, Size, Vec2},
    Color,
};
use piet_common::{Device, Piet, RenderContext};

const WIDTH: usize = 400;
const HEIGHT: usize = 600;

fn main() {
    let mut device = Device::new().unwrap();
    let mut bitmap = device.bitmap_target(WIDTH * 2, HEIGHT * 2, 2.0).unwrap();
    let mut rc = bitmap.render_context();

    rc.fill(
        Rect::from_origin_size(Point::ZERO, Size::new(WIDTH as f64, HEIGHT as f64)),
        &Color::WHITE,
    );
    draw(&mut rc);

    rc.finish().unwrap();
    std::mem::drop(rc);

    bitmap
        .save_to_file("temp-image.png")
        .expect("file save error");
}

fn draw(rc: &mut Piet) {
    /* todo move into seaparate example
    let mut axis = Axis::new(
        Direction::Left,
        LabelPosition::Before,
        WIDTH as f64 * 0.6,
        Categorical::new(vec![
            "first", "second", "third", "fourth", "fifth", "sixth", "seventh",
        ]),
    );
    axis.layout(rc).unwrap();
    axis.draw((WIDTH as f64 * 0.2, 100.), rc);
    let mut axis = Axis::new(
        Direction::Down,
        LabelPosition::After,
        HEIGHT as f64 * 0.6,
        Categorical::new(vec!["first", "second", "third", "fourth", "eggs"]).space_around(),
    );
    axis.layout(rc).unwrap();
    axis.draw((100., HEIGHT as f64 * 0.2), rc);
    */

    let hist_size = Size::new(WIDTH as f64 * 0.95, HEIGHT as f64 * 0.95);
    let hist_tl = Vec2::new(WIDTH as f64 * 0.025, HEIGHT as f64 * 0.025);
    let mut histogram = Histogram::<Categorical<&'static str>, _>::new(
        hist_size,
        ["first", "second", "third"],
        vec![12., 14., 10.],
    );

    rc.with_save(|rc| {
        rc.transform(Affine::translate(hist_tl));
        histogram.layout(rc).unwrap();
        histogram.draw(rc);
        Ok(())
    })
    .unwrap();
}
