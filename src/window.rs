use std::sync::Arc;

use winit::{
    event::{Event, KeyEvent, WindowEvent},
    event_loop::EventLoop,
    keyboard::{KeyCode, PhysicalKey},
    window::Window,
};

use crate::{app::AppState, graphics::WgpuContext};

pub async fn run(event_loop: EventLoop<()>, window: Arc<Window>, cell_number: u32) {
    let (width, height) = {
        let win_size = window.inner_size();
        (win_size.width, win_size.height)
    };

    let state = AppState::new(width, height, cell_number);
    let mut wgpu_context = WgpuContext::new(window, &state).await;

    let main_window_id = wgpu_context.window.id();
    event_loop
        .run(move |event, target| match event {
            Event::WindowEvent { window_id, event } if window_id == main_window_id => match event {
                WindowEvent::CloseRequested => target.exit(),
                WindowEvent::KeyboardInput {
                    event:
                        KeyEvent {
                            physical_key: PhysicalKey::Code(keycode),
                            ..
                        },
                    ..
                } => {
                    match keycode {
                        KeyCode::KeyS => {
                            // toggle app pause
                            wgpu_context.window.request_redraw();
                        }
                        KeyCode::Escape => target.exit(),
                        _ => {}
                    }
                }
                WindowEvent::RedrawRequested => {
                    // aquire new frame
                    let frame = match wgpu_context.surface.get_current_texture() {
                        Ok(frame) => frame,
                        // If we timed out, just try again
                        Err(wgpu::SurfaceError::Timeout) => wgpu_context
                            .surface
                            .get_current_texture()
                            .expect("Failed to acquire next surface texture!"),
                        Err(
                            // If the surface is outdated, or was lost, reconfigure it.
                            wgpu::SurfaceError::Outdated | wgpu::SurfaceError::Lost,
                        ) => {
                            wgpu_context
                                .surface
                                .configure(&wgpu_context.device, &wgpu_context.surface_config);
                            wgpu_context
                                .surface
                                .get_current_texture()
                                .expect("Failed to acquire next surface texture!")
                        }
                        // If OutOfMemory happens we're quiting
                        Err(wgpu::SurfaceError::OutOfMemory) => return target.exit(),
                    };
                    // create frame view
                    let view = frame.texture.create_view(&wgpu::TextureViewDescriptor {
                        format: Some(wgpu_context.surface_config.view_formats[0]),
                        ..wgpu::TextureViewDescriptor::default()
                    });

                    // do the necessary computation to render the frame
                    wgpu_context.render(&view, &state);

                    // show frame
                    frame.present();
                    // draw next frame
                    wgpu_context.window.request_redraw();
                }

                WindowEvent::Resized(_) => {
                    // should not be reachable in theory
                    eprintln!("[FATAL]: App does not support resize.");
                    target.exit()
                }
                _ => {}
            },
            _ => {}
        })
        .unwrap();
}
