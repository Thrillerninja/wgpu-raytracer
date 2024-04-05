use std::collections::VecDeque;
use image::{DynamicImage, GenericImageView};
use wgpu::util::DeviceExt;
use wgpu::Features;
use winit::{
    event::*, event_loop::{ControlFlow, EventLoop}, keyboard::{Key, KeyCode, NamedKey}, window::{ResizeDirection, Window}
};
use rtbvh::*;
use egui_wgpu::ScreenDescriptor;

#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::*;

mod gpu;
use gpu::setup_gpu;

mod gui;
use gui::EguiRenderer;

mod gui_example;
use gui_example::gui;

mod camera;
use camera::Camera;

mod models;
use models::{load_obj, load_gltf, load_svg};

mod texture;
// use texture::{create_textureset, load_texture_set, load_texture_set_from_images};
use texture::{create_texture, load_textures, load_textures_from_image, scale_texture};

mod structs;
use structs::{CameraUniform, TriangleUniform, SphereUniform, BvhUniform, ShaderConfig};
use structs::{Material, Sphere, Triangle};

mod config;
use config::Config;

mod renderer;
use renderer::setup_camera;

use crate::{models::load_hdri, renderer::{setup_bvh, setup_spheres, setup_textures, setup_tris_objects, setup_hdri}};


struct State<'a>{
    window: Window,
    surface: wgpu::Surface<'a>,
    device: wgpu::Device,
    queue: wgpu::Queue,
    config: wgpu::SurfaceConfiguration,
    size: winit::dpi::PhysicalSize<u32>,
    //Antialiasing Sample Textures
    denoising_camera_buffer: wgpu::Buffer,
    denoising_pass_buffer: wgpu::Buffer,
    denoising_bind_group: wgpu::BindGroup,
    denoising_pipeline: wgpu::ComputePipeline,
    //Raytracing
    shader_config_bind_group: wgpu::BindGroup,
    shader_config_buffer: wgpu::Buffer,
    ray_tracing_pipeline: wgpu::ComputePipeline,
    raytracing_bind_group: wgpu::BindGroup,
    screen_render_pipeline: wgpu::RenderPipeline,
    screen_bind_group: wgpu::BindGroup,
    //Camera
    camera: camera::Camera,
    projection: camera::Projection,
    camera_controller: camera::CameraController,
    camera_uniform: CameraUniform,
    camera_buffer: wgpu::Buffer,
    camera_bind_group: wgpu::BindGroup,
    mouse_pressed: bool,
    //Objects
    object_bind_group: wgpu::BindGroup,
    bvh_bind_group: wgpu::BindGroup,
    //Textures
    texture_bind_group: wgpu::BindGroup,
    //GUI
    egui: gui::EguiRenderer,
    fps: VecDeque<f32>,
}

impl<'a> State<'a>{  
    async fn new(window: Window) -> Self {
        // The instance is a handle to our GPU
        let (window,
            device, 
            queue, 
            surface, 
            config, 
            color_buffer_view, 
            mut userconfig, 
            size) = setup_gpu(window).await;

        //----------Camera-------------
        let (camera, projection, camera_controller, camera_uniform) = setup_camera(&config, &userconfig);

        let camera_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Camera Buffer"),
            contents: bytemuck::cast_slice(&[camera_uniform]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::COPY_SRC,
        });

        let camera_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                entries: &[wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                }],
                label: Some("camera_bind_group_layout"),
            });

        let camera_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &camera_bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: camera_buffer.as_entire_binding(),
            }],
            label: Some("camera_bind_group"),
        });
        println!("Camera ready");

        //----------Anit-Aliasing-------------
        // Inside the State struct, add a denoising buffer and a bind group for it.
        // denoising buffer and bind group
        let denoising_texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("denoising Buffer"),
            view_formats: &[config.format], // Use the same format as the color buffer
            size: wgpu::Extent3d {
                width: config.width,
                height: config.height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: config.format, // Use the same format as the color buffer
            usage: wgpu::TextureUsages::TEXTURE_BINDING
                | wgpu::TextureUsages::COPY_DST
                | wgpu::TextureUsages::STORAGE_BINDING
                | wgpu::TextureUsages::COPY_SRC,
        });
        let denoising_buffer_view = denoising_texture.create_view(&wgpu::TextureViewDescriptor::default());

        // Pass camera info to denoising shader
        let denoising_camera: Camera = camera.clone();

        let mut denoising_camera_uniform = CameraUniform::new();
        denoising_camera_uniform.update_view_proj(&denoising_camera, &projection);

        let denoising_camera_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Denoising Camera Buffer"),
            contents: bytemuck::cast_slice(&[denoising_camera_uniform]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        // Small uniform buffer for denoising pass number indicator
        let denoising_pass_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Denoising Pass Buffer"),
            contents: bytemuck::cast_slice(&[0u32]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        let denoising_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                entries: &[
                    wgpu::BindGroupLayoutEntry {
                        binding: 0, // This should match the binding number in the shader
                        visibility: wgpu::ShaderStages::COMPUTE,
                        ty: wgpu::BindingType::StorageTexture {
                        access: wgpu::StorageTextureAccess::ReadWrite,
                        format: config.format, // Match the texture format in the shader
                        view_dimension: wgpu::TextureViewDimension::D2,
                        },
                        count: None,            
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 1, // This should match the binding number in the shader
                        visibility: wgpu::ShaderStages::COMPUTE,
                        ty: wgpu::BindingType::StorageTexture {
                        access: wgpu::StorageTextureAccess::ReadWrite,
                        format: config.format, // Match the texture format in the shader
                        view_dimension: wgpu::TextureViewDimension::D2,
                        },
                        count: None,            
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 2,
                        visibility: wgpu::ShaderStages::COMPUTE,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Uniform,
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 3,
                        visibility: wgpu::ShaderStages::COMPUTE,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Uniform,
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 4,
                        visibility: wgpu::ShaderStages::COMPUTE,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Uniform,
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                ],
                label: Some("denoising_bind_group_layout"),
            });

        let denoising_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &denoising_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0, // This should match the binding number in the shader
                    resource: wgpu::BindingResource::TextureView(&color_buffer_view),
                },
                wgpu::BindGroupEntry {
                    binding: 1, // This should match the binding number in the shader
                    resource: wgpu::BindingResource::TextureView(&denoising_buffer_view),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: camera_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: denoising_camera_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 4,
                    resource: denoising_pass_buffer.as_entire_binding(),
                },
            ],
            label: Some("denoising_bind_group"),
        });

        // Create a pipeline layout for denoising denoising
        let denoising_pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Denoising Pipeline Layout"),
            bind_group_layouts: &[
                &denoising_bind_group_layout],
            push_constant_ranges: &[],
        });

        // Load your denoising denoising shader
        let denoising_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Denoising Shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("denoising.wgsl").into()), // Replace with your actual shader source
        });

        // Create a denoising denoising pipeline
        let denoising_pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("Denoising Pipeline"),
            layout: Some(&denoising_pipeline_layout),
            module: &denoising_shader,
            entry_point: "main", // Change to your actual entry point name
        });


        //----------Objects-------------
        let (triangles, 
            triangles_uniform, 
            materials, 
            textures) = setup_tris_objects(&userconfig);

        
        // Create a buffer to hold the vertex data
        let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Vertex Buffer"),
            contents: bytemuck::cast_slice(&triangles_uniform),
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
        });

        // Create Textureset
        let textures_buffer = setup_textures(&userconfig, textures, &device, &queue, &config);

        // ---------Spheres-------------
        let spheres_uniform = setup_spheres(&userconfig);
        // Spheres to Uniform buffer compatible type                   
        
        // Create a buffer to hold the sphere data
        let sphere_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Vertex Buffer"),
            contents: bytemuck::cast_slice(&spheres_uniform),
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
        });

        // -------Combined Objects----------
        // Create a bind group layout for the shader
        let object_bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0, // This should match the binding number in the shader for object data
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: true },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,            
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1, // This should match the binding number in the shader for object data
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: true },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,            
                }
            ],
            label: Some("object_bind_group_layout"),
        });

        // Create a bind group using the layout and the buffers
        let object_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &object_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0, // This should match the binding number in the shader for object data
                    resource: vertex_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1, // This should match the binding number in the shader for object data
                    resource: sphere_buffer.as_entire_binding(),
                }
            ],
            label: Some("object_bind_group"),
        });
        println!("Objects ready");


        //-------------BVH---------------
        
        let (bvh_uniform, bvh_prim_indices) = setup_bvh(&triangles);
        
        // Store bvh nodes in a buffer as a array
        let bvh_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("BVH Buffer"),
            contents: bytemuck::cast_slice(&bvh_uniform),
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
        });

        // Store prim indices of the bvh nodes in a buffer as a array
        let bvh_prim_indices_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("BVH Prim Indices Buffer"),
            contents: bytemuck::cast_slice(&bvh_prim_indices),
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
        });

        // Send nodes and prim indices to the shader
        let bvh_bind_goup_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0, // This should match the binding number in the shader for object data
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: true },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,            
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1, // This should match the binding number in the shader for object data
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: true },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,            
                }
            ],
            label: Some("bvh_bind_group_layout"),
        });

        let bvh_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &bvh_bind_goup_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0, // This should match the binding number in the shader for object data
                    resource: bvh_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1, // This should match the binding number in the shader for object data
                    resource: bvh_prim_indices_buffer.as_entire_binding(),
                }
            ],
            label: Some("bvh_bind_group"),
        });
        println!("BVH ready");

        //----------Textures-------------
        let background_texture = setup_hdri(&userconfig, &device, &queue, &config);

        let material_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Material Buffer"),
            contents: bytemuck::cast_slice(&materials),
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
        });

        let background_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Background Buffer"),
            contents: bytemuck::cast_slice(&[userconfig.background]),
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
        });

        let texture_bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Texture {
                        multisampled: false,
                        view_dimension: wgpu::TextureViewDimension::D2Array,
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 2,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: true },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,         
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 3,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: true },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,         
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 4,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Texture {
                        multisampled: false,
                        view_dimension: wgpu::TextureViewDimension::D2,
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                    },
                    count: None,
                }
            ],
            label: Some("texture_bind_group_layout"),
        });

        let texture_sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("Sampler"),
            address_mode_u: wgpu::AddressMode::Repeat,
            address_mode_v: wgpu::AddressMode::Repeat,
            address_mode_w: wgpu::AddressMode::Repeat,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Nearest,
            mipmap_filter: wgpu::FilterMode::Nearest,
            anisotropy_clamp: 1,
            ..Default::default()
        });

        let texture_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &texture_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::Sampler(&texture_sampler),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::TextureView(&textures_buffer.create_view(&wgpu::TextureViewDescriptor::default())),
                },
                wgpu::BindGroupEntry {
                    binding: 2, 
                    resource: material_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 3, 
                    resource: background_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 4,
                    resource: wgpu::BindingResource::TextureView(&background_texture.create_view(&wgpu::TextureViewDescriptor::default())),
                }
            ],
            label: Some("texture_bind_group"),
        });
        println!("Textures ready");
        
        //----------Raytracing-------------
        let shader_config = structs::ShaderConfig::default();
        // Raytracing config via uniform buffer
        let shader_config_bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
            ],
            label: Some("shader_config"),
        });

        let shader_config_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Shader config Buffer"),
            contents: bytemuck::cast_slice(&[shader_config]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        let shader_config_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &shader_config_bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: shader_config_buffer.as_entire_binding(),
            }],
            label: Some("shader_config_bind_group"),
        });


        // Create a bind group layout for the shader
        let raytracing_bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0, // This should match the binding number in the shader
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::StorageTexture {
                    access: wgpu::StorageTextureAccess::WriteOnly,
                    format: config.format, // Match the texture format in the shader
                    view_dimension: wgpu::TextureViewDimension::D2,
                    },
                    count: None,            
                }
                ],
            label: Some("raytracing_bind_group_layout")});
        
        // Create a bind group using the layout and the texture view
        let raytracing_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &raytracing_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0, // This should match the binding number in the shader
                    resource: wgpu::BindingResource::TextureView(&color_buffer_view),
                }
            ],
            label: Some("raytracing_bind_group"),
        });

                // Load your ray tracing shaders (ray generation, intersection, etc.)
        let ray_generation_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Ray Generation Shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("raygen.wgsl").into()), // Replace with your actual shader source
        });


        // Create a ray tracing pipeline layout
        let raytracing_pipeline_layout =
        device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Ray Tracing Pipeline Layout"),
            bind_group_layouts: &[
                &shader_config_bind_group_layout,
                &raytracing_bind_group_layout,
                &camera_bind_group_layout,
                &object_bind_group_layout,
                &texture_bind_group_layout,
                &bvh_bind_goup_layout,
            ],
            push_constant_ranges: &[],
        });

        let ray_tracing_pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("Ray Tracing Pipeline"),
            layout: Some(&raytracing_pipeline_layout),
            module: &ray_generation_shader,
            entry_point: "main", // Change to your actual entry point name
            }
        );
        println!("Raytracing shader ready");


        //----------Transfer to screen-------------
        //Create a Sampler for trasfering color data from render to screen texture
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("Sampler"),
            address_mode_u: wgpu::AddressMode::Repeat,
            address_mode_v: wgpu::AddressMode::Repeat,
            address_mode_w: wgpu::AddressMode::Repeat,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Nearest,
            mipmap_filter: wgpu::FilterMode::Nearest,
            anisotropy_clamp: 1,
            ..Default::default()
        });

        // Create a bind group layout for the shader
        let screen_bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                entries: &[
                    wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 1,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Texture {
                            multisampled: false,
                            view_dimension: wgpu::TextureViewDimension::D2,
                            sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        },
                        count: None,
                    },
                ],
                label: Some("screen_bind_group_layout"),
            });
        
        let screen_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &screen_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::Sampler(&sampler),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::TextureView(&color_buffer_view),
                },
            ],
            label: Some("screen_bind_group"),
        });
    

        // Create screen pipeline to display render result
        let screen_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Fragment Shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("screen-shader.wgsl").into()),
        });

        let screen_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("Render Pipeline Layout"),
                bind_group_layouts: &[&screen_bind_group_layout],
                push_constant_ranges: &[],
            });
        
        let screen_render_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Render Pipeline"),
            layout: Some(&screen_pipeline_layout),
            vertex: wgpu::VertexState {
                module: &screen_shader,
                entry_point: "vs_main", // Add your vertex shader entry point here
                buffers: &[],
            },
            fragment: Some(wgpu::FragmentState {
                module: &screen_shader,
                entry_point: "fs_main",
                targets: &[Some(wgpu::ColorTargetState {
                    format: config.format,
                    blend: Some(wgpu::BlendState::REPLACE),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            depth_stencil: None,
            multisample: wgpu::MultisampleState {
                count: 1,
                mask: !0,
                alpha_to_coverage_enabled: false,
            },
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                strip_index_format: None,
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: None,
                // Setting this to anything other than Fill requires Features::POLYGON_MODE_LINE
                // or Features::POLYGON_MODE_POINT
                polygon_mode: wgpu::PolygonMode::Fill,
                // Requires Features::DEPTH_CLIP_CONTROL
                unclipped_depth: false,
                // Requires Features::CONSERVATIVE_RASTERIZATION
                conservative: false,
            },
            // If the pipeline will be used with a multiview render pass, this
            // indicates how many array layers the attachments will have.
            multiview: None,
        });

        //----------- GUI -------------        
        let egui = EguiRenderer::new(
            &device,       // wgpu Device
            config.format, // TextureFormat
            None,          // this can be None
            1,             // samples
            &window,       // winit Window
        );

        let fps: VecDeque<f32> = VecDeque::with_capacity(100);
        
        Self {
            surface,
            device,
            queue,
            config,
            window,
            size,
            denoising_camera_buffer,
            denoising_pass_buffer,
            denoising_bind_group,
            denoising_pipeline,
            shader_config_bind_group,
            shader_config_buffer,
            ray_tracing_pipeline,
            raytracing_bind_group,
            screen_render_pipeline,
            screen_bind_group,
            camera,
            projection,
            camera_controller,
            camera_buffer,
            camera_bind_group,
            camera_uniform,
            mouse_pressed: false,
            object_bind_group,
            bvh_bind_group,
            texture_bind_group,
            egui,
            fps
        }
    }

    pub fn window(&self) -> &Window {
        &self.window
    }

    fn resize(&mut self, new_size: winit::dpi::PhysicalSize<u32>) {
        if new_size.width > 0 && new_size.height > 0 {
            self.projection.resize(new_size.width, new_size.height);
            self.size = new_size;
            self.config.width = new_size.width;
            self.config.height = new_size.height;
            self.surface.configure(&self.device, &self.config);
        }
    }

    fn input(&mut self, event: &WindowEvent) -> bool {
        match event {
            WindowEvent::KeyboardInput {
                event:
                    KeyEvent {
                        logical_key: key,
                        state,
                        ..
                    },
                ..
            } => self.camera_controller.process_keyboard(key, state),
            WindowEvent::MouseWheel { delta, .. } => {
                self.camera_controller.process_scroll(delta);
                true
            }
            WindowEvent::MouseInput {
                button: MouseButton::Left,
                state,
                ..
            } => {
                self.mouse_pressed = *state == ElementState::Pressed;
                true
            }
            _ => false,
        }
    }

    fn update(&mut self, dt: std::time::Duration) {
        self.camera_controller.update_camera(&mut self.camera, dt);
        self.camera_uniform.update_view_proj(&self.camera, &self.projection);
        self.camera_uniform.update_frame();
        self.queue.write_buffer(
            &self.camera_buffer,
            0,
            bytemuck::cast_slice(&[self.camera_uniform]),
        );

        // ---------FPS---------
        // println!("FPS: {}", 1.0 / dt.as_secs_f32());

        // If fps is empty fill with the first value
        if self.fps.is_empty() {
            for i in 0..100 {
                self.fps.push_back(1.0 / dt.as_secs_f32());
            }
        }
        self.fps.push_front(1.0 / dt.as_secs_f32());
        self.fps.truncate(100);
    }

    fn render(&mut self) -> Result<(), wgpu::SurfaceError> {
        // Get the current output texture from the surface
        let output = self.surface.get_current_texture()?;
    
        // Create a view for the output texture
        let view = output
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());
    
        // Create a command encoder
        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Render Encoder"),
            });
            
        // println!(
        //     "Camera Position: {:?}, Camera Quaternion: {:?}",
        //     self.camera.position,
        //     self.camera.rotation
        // );
    
        //----------Raytracing pass----------
        {
            // Start a compute pass for ray tracing
            let mut compute_pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("Ray Tracing Pass"),
                timestamp_writes: None,
            });
    
            // Set ray tracing pipeline and bind group
            compute_pass.set_pipeline(&self.ray_tracing_pipeline);
            compute_pass.set_bind_group(0, &self.shader_config_bind_group, &[]);
            compute_pass.set_bind_group(1, &self.raytracing_bind_group, &[]);
            compute_pass.set_bind_group(2, &self.camera_bind_group, &[]);
            compute_pass.set_bind_group(3, &self.object_bind_group, &[]);
            compute_pass.set_bind_group(4, &self.texture_bind_group, &[]);
            compute_pass.set_bind_group(5, &self.bvh_bind_group, &[]);
    
            // Dispatch workgroups for ray tracing (adjust dimensions as needed)
            compute_pass.dispatch_workgroups(
                (self.config.width + 7) / 8,
                (self.config.height + 7) / 8,
                1
            );
        }


        //----------1. Denoising pass----------
        {
            self.queue.write_buffer(
                &self.denoising_pass_buffer,
                0,
                bytemuck::cast_slice(&[0u32]),
            );

            let mut denoise_pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("1. Denoising Pass"),
                timestamp_writes: None,
            });
    
            // Set denoising pipeline and bind group
            denoise_pass.set_pipeline(&self.denoising_pipeline);
            denoise_pass.set_bind_group(0, &self.denoising_bind_group, &[]);
    
            // Dispatch workgroups for denoising (adjust dimensions as needed)
            denoise_pass.dispatch_workgroups(
                (self.config.width + 7) / 8,
                (self.config.height + 7) / 8,
                1
            );
        }

        // Submit the command encoder for the 1st pass
        self.queue.submit(std::iter::once(encoder.finish()));

        // Create a new command encoder for the 2nd denoising pass
        let mut encoder2 = self.device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("Render Encoder 2"),
        });

        //----------2. Denoising pass----------
        // Set denoising pass number to 1
        self.queue.write_buffer(
            &self.denoising_pass_buffer,
            0,
            bytemuck::cast_slice(&[1u32]),
        );

        // Perform 2. denoising pass
        {
            let mut denoise_pass = encoder2.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("2. Denoising Pass"),
                timestamp_writes: None,
            });
    
            // Set denoising pipeline and bind group
            denoise_pass.set_pipeline(&self.denoising_pipeline);
            denoise_pass.set_bind_group(0, &self.denoising_bind_group, &[]);
    
            // Dispatch workgroups for denoising (adjust dimensions as needed)
            denoise_pass.dispatch_workgroups(
                (self.config.width + 7) / 8,
                (self.config.height + 7) / 8,
                1
            );
        }

        // Submit the command encoder for the 1st pass
        self.queue.submit(std::iter::once(encoder2.finish()));

        // Create a new command encoder for the 2nd denoising pass
        let mut encoder3 = self.device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("Render Encoder 3"),
        });
    
        // Render pass
        {
            // Begin a render pass
            let mut render_pass = encoder3.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Render Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color {
                            r: 0.1,
                            g: 0.2,
                            b: 0.3,
                            a: 1.0,
                        }),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                occlusion_query_set: None,
                timestamp_writes: None,
            });
    
            // Set the screen rendering pipeline and bind group
            render_pass.set_pipeline(&self.screen_render_pipeline);
            render_pass.set_bind_group(0, &self.screen_bind_group, &[]);
    
            // Draw using the render pass (adjust the range as needed)
            render_pass.draw(0..6, 0..1);
        }
        self.queue.write_buffer(
            &self.denoising_camera_buffer,
            0,
            bytemuck::cast_slice(&[self.camera_uniform]),
        );
    
        // Draw the GUI
        let screen_descriptor = ScreenDescriptor {
            size_in_pixels: [self.config.width, self.config.height],
            pixels_per_point: self.window().scale_factor() as f32,
        };

        self.egui.draw(
            &self.device,
            &self.queue,
            &mut encoder3,
            &self.window,
            &view,
            screen_descriptor,
            |ui| gui(ui, &self.fps),
        );

        self.queue.submit(std::iter::once(encoder3.finish()));
        output.present();
    
        Ok(())
    }    
}

fn main() {
    pollster::block_on(run());
}

#[cfg_attr(target_arch = "wasm32", wasm_bindgen(start))]
pub async fn run() {
    cfg_if::cfg_if! {
        if #[cfg(target_arch = "wasm32")] {
            std::panic::set_hook(Box::new(console_error_panic_hook::hook));
            console_log::init_with_level(log::Level::Info).expect("Could't initialize logger");
        } else {
            env_logger::init();
        }
    }

    let event_loop = EventLoop::new().unwrap();
    let title = env!("CARGO_PKG_NAME");
    let builder = winit::window::WindowBuilder::new();
    let window = builder
        .with_title(title)
        .build(&event_loop)
        //Probably change the size here;
        .unwrap();

    #[cfg(target_arch = "wasm32")]
    {
        // Winit prevents sizing with CSS, so we have to set
        // the size manually when on web.
        use winit::dpi::PhysicalSize;
        window.set_inner_size(PhysicalSize::new(1920, 1080));

        use winit::platform::web::WindowExtWebSys;
        web_sys::window()
            .and_then(|win| win.document())
            .and_then(|doc| {
                let dst = doc.get_element_by_id("wasm-example")?;
                let canvas = web_sys::Element::from(window.canvas());
                dst.append_child(&canvas).ok()?;
                Some(())
            })
            .expect("Couldn't append canvas to document body.");
    }
        
    // ControlFlow::Poll continuously runs the event loop, even if the OS hasn't
    // dispatched any events.
    event_loop.set_control_flow(ControlFlow::Poll);

    let mut state = State::new(window).await;
    let mut last_render_time = instant::Instant::now();

    let _ = event_loop.run(move |event, elwt| {
        match event {
            Event::WindowEvent {
                ref event,
                window_id,
            } if window_id == state.window().id() && !state.input(event) => {
                // UI upadtes
                state.egui.handle_input(&mut state.window, &event);

                // Handle window events
                match event {
                    #[cfg(not(target_arch="wasm32"))]
                    WindowEvent::CloseRequested => {
                        elwt.exit();
                    }
                    WindowEvent::RedrawRequested => {
                        let now = instant::Instant::now();
                        let dt = now - last_render_time;
                        last_render_time = now;
                        state.update(dt);
                        match state.render() {
                            Ok(_) => {}
                            // Reconfigure the surface if it's lost or outdated
                            Err(wgpu::SurfaceError::Lost | wgpu::SurfaceError::Outdated) => state.resize(state.size),
                            // The system is out of memory, we should probably quit
                            Err(wgpu::SurfaceError::OutOfMemory) => elwt.exit(),
                            // We're ignoring timeouts
                            Err(wgpu::SurfaceError::Timeout) => log::warn!("Surface timeout"),
                        }
                    }
                    WindowEvent::KeyboardInput {
                        event:
                            KeyEvent {
                                state: ElementState::Pressed,
                                logical_key: key,
                                ..
                            },
                        ..
                    } => {
                        match key {
                            Key::Named(NamedKey::Escape) => elwt.exit(),
                            _ => {}
                        }
                    }
                    WindowEvent::Resized(physical_size) => {
                        state.resize(*physical_size);
                    }
                    WindowEvent::ScaleFactorChanged  {scale_factor, .. } => {
                        println!("Window={window_id:?} changed scale to {scale_factor}");
                    }
                    _ => {}
                };
            }
            Event::DeviceEvent {
                event: DeviceEvent::MouseMotion{ delta, },
                ..
            } => if state.mouse_pressed {
                state.camera_controller.process_mouse(delta.0, delta.1)
            }
            Event::AboutToWait => {
                // Application update code.
    
                // Queue a RedrawRequested event.
                //
                // You only need to call this if you've determined that you need to redraw in
                // applications which do not always need to. Applications that redraw continuously
                // can render here instead.
                state.window().request_redraw();
            },
            _ => ()
        }
    });
}