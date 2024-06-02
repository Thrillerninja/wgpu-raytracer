use raytracing_lib::run;

/// Entry point for the application.
///
/// It then calls the `run` function and blocks until it completes.
///
/// # Safety
/// Running this scene may overwhelm your system and cause system crashes.
/// Even a 3080 GPU struggled with this scene.
/// I do not recommend running this scene, there is no groundbraking new Feature to see here.
/// 
/// # Scene contents
/// This scene contains a gltf model of a city block.
/// The model has 40 Textures which make the programm very slow.
/// The 25587 triangles dont help either. (Higher tris numbers are possible, but not with as many assigned textures)
/// 
fn main() {
    std::env::set_var("RUST_BACKTRACE", "1"); //Keep this on to hav any Idead what happened if the GPU causes a crash.
    pollster::block_on(run(Some("examples/99-caution_max_scene/config.toml")));
}
