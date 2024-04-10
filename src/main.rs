mod app;
mod graphics;
mod window;

use std::sync::Arc;
use window::run;
use winit::event_loop::EventLoop;

const WINDOW_SIZE: u32 = 1000;

pub fn main() {
    let event_loop = EventLoop::new().unwrap();
    #[allow(unused_mut)]
    let mut builder = winit::window::WindowBuilder::new()
        .with_title("SAXRUMFEX")
        .with_inner_size(winit::dpi::LogicalSize::new(WINDOW_SIZE, WINDOW_SIZE))
        .with_resizable(false)
        .with_active(true);
    // .with_window_icon(window_icon);

    let window = builder.build(&event_loop).unwrap();
    let window = Arc::new(window);
    env_logger::builder().format_timestamp_nanos().init();
    pollster::block_on(run(event_loop, window, 10000));
}
