
use raytracing_lib::run;

/// Entry point for the application.
///
/// It then calls the `run` function and blocks until it completes.
fn main() {
    pollster::block_on(run(None));
}