use winit::{event::*, event_loop::{ControlFlow, EventLoop}, keyboard::{Key, NamedKey}};

pub mod state;
pub mod helper;
use crate::lib::run;

/// Entry point for the application.
///
/// It then calls the `run` function and blocks until it completes.
fn main() {
    std::env::set_var("RUST_BACKTRACE", "1");
    std::env::set_var("CARGO_CACHE", "1");
    pollster::block_on(lib::run(None));
}