//! # wgpu_utils
//!
//! `wgpu_utils` is a utility crate for working with the WebGPU API. It provides abstractions for buffers and GPU setup.
//!
//! ## Features
//!
//! - `BufferInitDescriptor`, `BindGroupDescriptor`, `BufferType`, `BindingResourceTemplate`: These types are used for managing GPU buffers.
//! - `setup_gpu`: This function is used to initialize the GPU.
//!
//! ## Examples
//!
//! ```rust ignore
//! use wgpu_utils::{BufferInitDescriptor, BindGroupDescriptor, BufferType, BindingResourceTemplate, setup_gpu};
//!
//! // Create a new BufferInitDescriptor
//! let buffer_init = BufferInitDescriptor {
//!     // initialization parameters
//! };
//!
//! // Create a new BindGroupDescriptor
//! let bind_group = BindGroupDescriptor {
//!     // initialization parameters
//! };
//!
//! // Initialize the GPU
//! let gpu = setup_gpu();
//! ```
//!

mod buffer;
mod gpu;


pub use buffer::{BufferInitDescriptor, BindGroupDescriptor, BufferType, BindingResourceTemplate};
pub use gpu::setup_gpu;