use winit::{event::*, event_loop::{ControlFlow, EventLoop}, keyboard::{Key, NamedKey}};

use raytracer::run; // Add the correct path to the lib module.

/// Entry point for the application.
///
/// It then calls the `run` function and blocks until it completes.
fn main() {
    pollster::block_on(run(None));
}