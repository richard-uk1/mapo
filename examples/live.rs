use druid_shell::{
    kurbo::{Affine, Size, Vec2},
    piet::{Color, Piet, RenderContext},
    Application, Region, TimerToken, WinHandler, WindowBuilder, WindowHandle,
};
use mapo::scatter::Scatter;
use qu::ick_use::*;
use rand_distr::{Distribution, Normal};
use std::{any::Any, time::Duration};

const WIDTH: usize = 400;
const HEIGHT: usize = 600;
const POINTS: usize = 4_000;
const HERTZ: u64 = 24;

struct HelloState {
    chart: Scatter,
    handle: WindowHandle,
}

impl WinHandler for HelloState {
    fn connect(&mut self, handle: &WindowHandle) {
        self.handle = handle.clone();
        handle.request_timer(Duration::from_millis(1000 / HERTZ));
    }

    fn timer(&mut self, _token: TimerToken) {
        self.chart.set_values(random_scatter());
        self.handle.invalidate();
        self.handle
            .request_timer(Duration::from_millis(1000 / HERTZ));
    }

    fn prepare_paint(&mut self) {}

    fn paint(&mut self, piet: &mut Piet<'_>, _invalid: &Region) {
        let size = Size::new(WIDTH as f64 * 0.95, HEIGHT as f64 * 0.95);
        let tl = Vec2::new(WIDTH as f64 * 0.025, HEIGHT as f64 * 0.025);

        piet.clear(None, Color::WHITE);
        piet.with_save(|rc| {
            rc.transform(Affine::translate(tl));
            self.chart.layout(size, rc).unwrap();
            self.chart.draw(rc);
            Ok(())
        })
        .unwrap();
        piet.finish().unwrap();
    }

    fn request_close(&mut self) {
        self.handle.close();
    }

    fn destroy(&mut self) {
        Application::global().quit()
    }

    fn as_any(&mut self) -> &mut dyn Any {
        self
    }
}

#[qu::ick]
fn main() -> Result {
    let app = Application::new()?;
    let mut builder = WindowBuilder::new(app.clone());
    builder.set_size(Size::new(WIDTH as f64, HEIGHT as f64));
    builder.resizable(false);

    builder.set_handler(Box::new(HelloState {
        chart: Scatter::new(random_scatter()),
        handle: WindowHandle::default(),
    }));
    builder.set_title("Mapo Live Example");

    let window = builder.build()?;
    window.show();
    app.run(None);
    Ok(())
}

fn random_scatter() -> Vec<(f64, f64)> {
    let normal = Normal::new(10., 8.).unwrap();
    let rng = &mut rand::thread_rng();
    (0..POINTS)
        .map(|_| (normal.sample(rng), normal.sample(rng)))
        .collect()
}
