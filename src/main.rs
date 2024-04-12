use wgpu_raytracer::State;
use winit::{event::*, event_loop::{ControlFlow, EventLoop}, keyboard::{Key, NamedKey}};

fn main() {
    pollster::block_on(run());
}

#[cfg_attr(target_arch = "wasm32", wasm_bindgen(start))]
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
        .build(&event_loop)
        //Probably change the size here;
        .unwrap();

    #[cfg(target_arch = "wasm32")]
    {
        // Winit prevents sizing with CSS, so we have to set
        // the size manually when on web.
        use winit::dpi::PhysicalSize;
        window.set_inner_size(PhysicalSize::new(1920, 1080));

        use winit::platform::web::WindowExtWebSys;
        web_sys::window()
            .and_then(|win| win.document())
            .and_then(|doc| {
                let dst = doc.get_element_by_id("wasm-example")?;
                let canvas = web_sys::Element::from(window.canvas());
                dst.append_child(&canvas).ok()?;
                Some(())
            })
            .expect("Couldn't append canvas to document body.");
    }
        
    // ControlFlow::Poll continuously runs the event loop, even if the OS hasn't
    // dispatched any events.
    event_loop.set_control_flow(ControlFlow::Poll);

    let mut state = State::new(window).await;
    let mut last_render_time = instant::Instant::now();

    let _ = event_loop.run(move |event, elwt| {
        match event {
            Event::WindowEvent {
                ref event,
                window_id,
            } if window_id == state.window().id() && !state.input(event) => {
                // UI upadtes
                state.egui.handle_input(&mut state.window, &event);

                // Handle window events
                match event {
                    #[cfg(not(target_arch="wasm32"))]
                    WindowEvent::CloseRequested => {
                        elwt.exit();
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
                    WindowEvent::Resized(physical_size) => {
                        state.resize(*physical_size);
                    }
                    WindowEvent::ScaleFactorChanged  {scale_factor, .. } => {
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
            Event::AboutToWait => {
                // Application update code.
    
                // Queue a RedrawRequested event.
                //
                // You only need to call this if you've determined that you need to redraw in
                // applications which do not always need to. Applications that redraw continuously
                // can render here instead.
                state.window().request_redraw();
            },
            _ => ()
        }
    });
}