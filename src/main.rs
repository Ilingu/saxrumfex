mod app;
mod graphics;
mod window;

use app::{AppArgs, AppState};
use std::sync::Arc;
use window::run;
use winit::event_loop::EventLoop;

pub fn main() {
    let app_args = AppArgs::parse().expect("Failed to parse args");

    let event_loop = EventLoop::new().unwrap();
    #[allow(unused_mut)]
    let mut builder = winit::window::WindowBuilder::new()
        .with_title("SAXRUMFEX")
        .with_inner_size(winit::dpi::LogicalSize::new(
            app_args.window_size,
            app_args.window_size,
        ))
        .with_resizable(false)
        .with_active(true);
    // .with_window_icon(window_icon);

    let state = AppState::new(app_args);

    let window = builder.build(&event_loop).unwrap();
    let window = Arc::new(window);

    env_logger::builder().format_timestamp_nanos().init();
    pollster::block_on(run(event_loop, window, state));
}
