use mapo::scatter::scatter;
use piet::{
    kurbo::{Affine, Point, Rect, Size, Vec2},
    Color,
};
use piet_common::{Device, Piet, RenderContext};
use rand_distr::{Distribution, Normal};

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
    let size = Size::new(WIDTH as f64 * 0.95, HEIGHT as f64 * 0.95);
    let tl = Vec2::new(WIDTH as f64 * 0.025, HEIGHT as f64 * 0.025);

    let normal = Normal::new(10., 8.).unwrap();
    let rng = &mut rand::thread_rng();
    let values: Vec<_> = (0..4)
        .map(|_| (normal.sample(rng), normal.sample(rng)))
        .collect();
    let mut chart = scatter(values);

    rc.with_save(|rc| {
        rc.transform(Affine::translate(tl));
        chart.layout(size, rc).unwrap();
        chart.draw(rc);
        Ok(())
    })
    .unwrap();
}
