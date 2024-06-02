/*!
# GPU Accelerated Ray Tracer

This crate provides a GPU accelerated ray tracer implemented using the [wgpu](https://docs.rs/wgpu/latest/wgpu/) library.

## Overview

Ray tracing is a method for generating digital images by simulating how light would interact with objects in a virtual world. 
The path of light is traced by shooting rays into a scene and tracking them as they bounce between objects. 
Ray tracing is capable of producing very high-quality images that are physically accurate, but it is also computationally intensive.

This crate uses the [wgpu](https://docs.rs/wgpu/latest/wgpu/) library to accelerate the ray tracing process by utilizing the GPU. 
This allows it to generate high-quality images much faster than a CPU-only ray tracer.

## Feature Overview

- **GPU Acceleration**: Uses the power of the GPU to speed up the ray tracing process.
- **Physically Accurate**: Simulates the physics of light to produce realistic images.
- **Custom Scenes**: Allows you to configure the scene with different objects, materials, and lighting.
- **GUI Integration**: Integrates with the [egui](https://github.com/emilk/egui) library to provide a graphical user interface for exploring settings of the ray tracer.
- **Full 3D Camera Control**: Allows you to move the camera in 3D space to explore the scene from different angles.

## Modules

- [`raytracing_lib`](../raytracing_lib/index.html): Contains the main functionality for the ray tracer, including the `State` struct that manages the state of the application.
- [`scene`](../scene/index.html): Loads and manages the scene data, including the camera, objects, materials, and lighting.
- [`gui`](../gui/index.html): Provides a graphical user interface for the ray tracer using the `egui` library.
- [`wgpu_utils`](../wgpu_utils/index.html): Contains utility functions for working with the `wgpu` library.

## Usage

To use the ray tracer, you can create a `State` object and call its `run` method to start the ray tracing process.
The `State` object will handle setting up the window, initializing the GPU, and rendering the scene.

```rust no_run
// Import the `block_on` function from the `pollster` crate.
// This is used to block the current thread until the `run` function completes.
use pollster;

// Import the `run` function from the `raytracing_lib` crate.
use raytracing_lib::run;

// Entry point for the application.
//
// It then calls the `run` function and blocks until it completes.
// Since we are not passing any configuration file and instead using the default settings,
// we pass `None` as the argument to the `run` function.
fn main() {
    pollster::block_on(run(None));
}

```

## Examples

The `examples` directory contains several example scenes that demonstrate the capabilities of the ray tracer.
You can run these examples by running one of the following commands:

```sh
cargo run --example 0-one_sphere
cargo run --example 1-three_spheres
cargo run --example 2-obj_model
cargo run --example 3-gltf_model
cargo run --example 4-complex_material
cargo run --example 5-cornell_box
```

*/