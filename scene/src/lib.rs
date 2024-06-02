//! # Scene
//!
//! This module contains all the scene related code. It includes the camera, models, and textures.
//!
//! ## Modules
//!
//! - `camera`: Contains the `Camera` struct and related functions for controlling the camera.
//! - `config`: Loads the configuration file and creates the scene outline.
//! - `models`: Contains the loading functions for different model types and the HDRI images.
//! - `structs`: Contains the structs for the scene objects like `Material`, `Sphere`, `Triangle`, etc.
//! - `texture`: Contains related functions for loading and managing textures on the gpu.
//!
//! ## Usage
//!
//! ```sh
//! // Create a new Camera
//! let camera = Camera::new();
//!
//! // Load a 3D model
//! let model = Model::load("path/to/model.obj");
//!
//! // Create a new Texture
//! let texture = Texture::new("path/to/texture.png");
//! ```
mod config;
mod structs;
mod models;
mod texture;
mod camera;

pub use config::{Config, Textureset};
pub use structs::{ShaderConfig, CameraUniform, Background, Material, Sphere, Triangle,
            BvhUniform, TriangleUniform};
pub use camera::{Camera, CameraController, Projection};
pub use texture::{create_texture, load_textures_from_image, scale_texture};
pub use models::{load_hdr, load_gltf, load_obj};