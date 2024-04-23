use winit::{event::*, event_loop::{ControlFlow, EventLoop}, keyboard::{Key, NamedKey}};

pub mod state;
pub mod renderer;
use state::State;

/// Entry point for the application.
fn main() {
    std::env::set_var("RUST_BACKTRACE", "1");
    pollster::block_on(run());
}

/// Starts the application.
///
/// It initializes the logger, creates the window, and starts the event loop.
pub async fn run() {
    cfg_if::cfg_if! {
        if #[cfg(target_arch = "wasm32")] {
            std::panic::set_hook(Box::new(console_error_panic_hook::hook));
            console_log::init_with_level(log::Level::Info).expect("Could't initialize logger");
        } else {
            env_logger::init();
        }
    }

    let event_loop = EventLoop::new().unwrap();
    let title = env!("CARGO_PKG_NAME");
    let builder = winit::window::WindowBuilder::new();
    let window = builder
        .with_title(title)
        .with_inner_size(winit::dpi::LogicalSize::new(1200.0, 800.0))
        .build(&event_loop)
        .unwrap();
        
    // ControlFlow::Poll continuously runs the event loop, 
    // even if the OS hasn't dispatched any events.
    event_loop.set_control_flow(ControlFlow::Poll);

    let mut state = State::new(window).await;
    let mut last_render_time = instant::Instant::now();

    // Start the event loop
    let _ = event_loop.run(move |event, elwt| {
        match event {
            Event::WindowEvent {
                ref event,
                window_id,
            } if window_id == state.window().id() && !state.input(event) => {
                // Handle window events that aren't related to the ui or camera
                match event {
                    // Close the window if requested by the user
                    WindowEvent::CloseRequested => {
                        elwt.exit();
                    }
                    // Close the window if the escape key is pressed                    
                    WindowEvent::KeyboardInput {
                        event:
                            KeyEvent {
                                state: ElementState::Pressed,
                                logical_key: key,
                                ..
                            },
                        ..
                    } => {
                        match key {
                            Key::Named(NamedKey::Escape) => elwt.exit(),
                            _ => {}
                        }
                    }
                    WindowEvent::RedrawRequested => {
                        let now = instant::Instant::now();
                        let dt = now - last_render_time;
                        last_render_time = now;
                        state.update(dt);
                        match state.render() {
                            Ok(_) => {}
                            // Reconfigure the surface if it's lost or outdated
                            Err(wgpu::SurfaceError::Lost | wgpu::SurfaceError::Outdated) => state.resize(state.size),
                            // The system is out of memory, we should probably quit
                            Err(wgpu::SurfaceError::OutOfMemory) => elwt.exit(),
                            // We're ignoring timeouts
                            Err(wgpu::SurfaceError::Timeout) => log::warn!("Surface timeout"),
                        }
                    }
                    WindowEvent::Resized(physical_size) => {
                        state.resize(*physical_size);
                    }
                    WindowEvent::ScaleFactorChanged  { scale_factor, .. } => {
                        // Log when the window scale factor changes
                        println!("Window={window_id:?} changed scale to {scale_factor}");
                    }
                    _ => {}
                };
            }
            Event::DeviceEvent {
                event: DeviceEvent::MouseMotion{ delta, },
                ..
            } => if state.mouse_pressed {
                state.camera_controller.process_mouse(delta.0, delta.1)
            }
            // Request a redraw bevore the system goes to idle
            Event::AboutToWait => {
                // Application update call
                state.window().request_redraw();
            },
            _ => ()
        }
    });
}