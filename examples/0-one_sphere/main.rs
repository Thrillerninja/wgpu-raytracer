use winit::{event::*, event_loop::{ControlFlow, EventLoop}, keyboard::{Key, NamedKey}};
use raytracer::lib::run;
/// Entry point for the application.
///
/// It then calls the `run` function and blocks until it completes.
fn main() {
    pollster::block_on(run(Some("Config.toml".to_string())));
}