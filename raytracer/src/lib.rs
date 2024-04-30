use winit::{event::*, event_loop::{ControlFlow, EventLoop}, keyboard::{Key, NamedKey}};

use crate::state::State;

/// Starts the application.
///
/// This function initializes the logger, creates the window, and starts the event loop.
/// It sets a panic hook for wasm32 targets and initializes the logger accordingly.
/// For non-wasm32 targets, it uses the `env_logger` crate to initialize the logger.
///
/// It creates a new event loop and a window with a specified title and size.
/// The event loop is set to continuously run, even if the OS hasn't dispatched any events.
///
/// A new `State` object is created for the window.
/// The event loop is then started, and it handles various window and device events, such as:
/// - Closing the window when requested by the user or when the escape key is pressed
/// - Updating and rendering the state when a redraw is requested
/// - Resizing the state when the window size changes
/// - Logging when the window scale factor changes
/// - Processing mouse motion events
/// - Requesting a redraw before the system goes to idle and limiting the frame rate
///
/// # Errors
///
/// This function will terminate the process if there is an error loading the HDRI file or the texture file.
pub async fn run(resource_path: Option<String>) {
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

    let mut state = State::new(window, resource_path).await;
    let mut last_render_time = instant::Instant::now();

    // Start the event loop
    let _ = event_loop.run(move |event, elwt| {
        match event {
            Event::WindowEvent {
                ref event,
                window_id,
            } if window_id == state.window.id() && !state.input(event) => {
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
                // Limit frame rate
                if state.gui_config.frame_limit != 0 {
                    let frame_time = instant::Instant::now() - last_render_time;
                    if frame_time < std::time::Duration::from_secs_f32(1.0 / state.gui_config.frame_limit as f32){
                        std::thread::sleep(std::time::Duration::from_secs_f32(1.0 / state.gui_config.frame_limit as f32) - frame_time);
                    }
                }
                state.window.request_redraw();
            },
            _ => ()
        }
    });
}