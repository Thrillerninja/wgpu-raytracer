//! # GUI Crate
//!
//! This crate provides a graphical user interface (GUI) for the raytracer. It uses the `egui` library for rendering the GUI.
//!
//! ## Modules
//!
//! - `gui`: Contains the [`EguiRenderer`](gui/src/gui.rs) struct which is responsible for rendering the GUI.
//! - `gui_structure`: Defines the [`GuiConfig`](gui/src/gui_structure.rs) struct which holds the configuration for the GUI and the `gui` function which is the main function for rendering the GUI.
//! - `gui_raytracing_settings`: Contains the [`raytracing_settings_gui`](gui/src/gui_raytracing_settings.rs) function which renders the GUI for the raytracing settings.
//! - `gui_denoising_settings`: Contains the [`denoising_settings_gui`](gui/src/gui_denoising_settings.rs) function which renders the GUI for the denoising settings.
//!
//! ## Usage
//!
//! To use this crate, you need to create an instance of `EguiRenderer` and call its `render` method in your main loop. You also need to create an instance of `GuiConfig` and pass it to the `gui` function along with an `egui::Context` and your `ShaderConfig`.
//!
//! ```rust
//! let mut egui_renderer = EguiRenderer::new(/*...*/);
//! let mut gui_config = GuiConfig::default();
//! let mut shader_config = ShaderConfig::default();
//!
//! loop {
//!     let ui = egui_renderer.begin_frame(/*...*/);
//!     gui(&ui, /*...*/, &mut gui_config, &mut shader_config);
//!     egui_renderer.end_frame(/*...*/);
//! }
//! ```
//!
//! You can also open the raytracing settings and denoising settings GUIs by setting `ray_settings_open` and `denoise_settings_open` in `GuiConfig` to `true`, respectively.
//!
//! ```rust
//! gui_config.ray_settings_open = true;
//! gui_config.denoise_settings_open = true;
//! ```
//!
//! The GUI will automatically update when these values change.
//!
//! ## Features
//!
//! - FPS counter with color coding based on performance.
//! - Raytracing settings GUI for adjusting various raytracing parameters.
//! - Denoising settings GUI for adjusting various denoising parameters.
//! - Frame limiting with an option for unlimited framerate.

mod gui;
mod gui_structure;
mod gui_raytracing_settings;
mod gui_denoising_settings;

pub use gui::EguiRenderer;
pub use gui_structure::{GuiConfig, gui};
pub use gui_raytracing_settings::raytracing_settings_gui;
pub use gui_denoising_settings::denoising_settings_gui;
