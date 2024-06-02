use raytracing_lib::run;

/// Entry point for the application.
///
/// It then calls the `run` function and blocks until it completes.
fn main() {
    std::env::set_var("RUST_BACKTRACE", "1"); //Sometimes the GPU causes a crash, if this isnt set only a way to short nonsense error message is shown. Left it in here since the possiblility for a crsh rises in this example.
    pollster::block_on(run(Some("examples/5-cornell_box/config.toml")));
}