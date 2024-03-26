use image::DynamicImage;
use wgpu::util::DeviceExt;
use wgpu::Features;
use winit::{
    event::*,
    event_loop::{ControlFlow, EventLoop},
    window::Window,
};
use rtbvh::*;

#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::*;

mod camera;
use camera::Camera;

mod models;
use models::{load_obj, load_gltf, load_svg};

mod texture;
use texture::{create_textureset, load_texture_set, load_texture_set_from_images};

mod structs;
use structs::{CameraUniform, TriangleUniform, SphereUniform, BvhUniform};
use structs::{Material, Sphere, Triangle};

mod config;
use config::Config;

use crate::texture::{create_texture, load_textures, load_textures_from_image};

struct State {
    window: Window,
    surface: wgpu::Surface,
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
}

async fn hardware_launch(window: &Window) -> (wgpu::Surface, wgpu::Device, wgpu::Queue, wgpu::Adapter) {
    // The instance is a handle to our GPU
    let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
        backends: wgpu::Backends::DX12,
        dx12_shader_compiler: Default::default(),
    });

    let surface = unsafe { instance.create_surface(window) }.unwrap();


    let adapter = instance
        .request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::HighPerformance,
            compatible_surface: Some(&surface),
            force_fallback_adapter: false,
        })
        .await
        .unwrap();
    
    println!("{}", adapter.get_info().name);

    let (device, queue) = adapter
        .request_device(
            &wgpu::DeviceDescriptor {
                features: Features::TEXTURE_ADAPTER_SPECIFIC_FORMAT_FEATURES,
                label: None,
                limits: wgpu::Limits {
                    max_bind_groups: 5,
                    ..Default::default()
                }
            },
            None,
        )
        .await
        .unwrap();

    (surface, device, queue, adapter)
}

impl State {  
    async fn new(window: Window) -> Self {
        let (surface, device, queue, adapter) = hardware_launch(&window).await;

        let surface_caps = surface.get_capabilities(&adapter);
        
        let size = window.inner_size();

        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: wgpu::TextureFormat::Rgba8Unorm,
            width: size.width,
            height: size.height,
            present_mode: surface_caps.present_modes[0],
            alpha_mode: surface_caps.alpha_modes[0],
            view_formats: vec![],
        };
        surface.configure(&device, &config);     
        
        let mut userconfig = Config::new();
        //----------Color Buffer-------------
        // Create a color texture with a suitable sRGB format
        let color_texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("Storage Texture"),
            view_formats: &[config.format], // Use sRGB format for storage
            size: wgpu::Extent3d {
                width: config.width,
                height: config.height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: config.format, // Use sRGB format
            usage: wgpu::TextureUsages::TEXTURE_BINDING
                | wgpu::TextureUsages::COPY_DST
                | wgpu::TextureUsages::STORAGE_BINDING
                | wgpu::TextureUsages::COPY_SRC,
        });
        
        
        let color_buffer_view = color_texture.create_view(&wgpu::TextureViewDescriptor::default());

        //----------Camera-------------
        let camera = camera::Camera::new(userconfig.camera_position, cgmath::Deg(userconfig.camera_rotation[0]), cgmath::Deg(userconfig.camera_rotation[1]));
        let projection =
            camera::Projection::new(config.width, config.height, cgmath::Deg(userconfig.camera_fov), userconfig.camera_near_far[0], userconfig.camera_near_far[1]);
        let camera_controller = camera::CameraController::new(4.0, 1.6);

        let mut camera_uniform = CameraUniform::new();
        camera_uniform.update_view_proj(&camera, &projection);

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
            bind_group_layouts: &[&denoising_bind_group_layout],
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
        // Load SVG UV mapping file
        let tris_uv_mapping = match load_svg(userconfig.triangle_svg_uv_mapping_path){
            Err(error) => {
                // Handle the error
                eprintln!("Error loading SVG file: {:?}", error);
                std::process::exit(1);
            }
            Ok(data) => data,
        };   
        for i in 0..tris_uv_mapping.len(){
            println!("UV: {},{} {},{} {},{} ", tris_uv_mapping[i][0][0], tris_uv_mapping[i][0][1], tris_uv_mapping[i][1][0], tris_uv_mapping[i][1][1], tris_uv_mapping[i][2][0], tris_uv_mapping[i][2][1]);
        }

        let mut triangles: Vec<Triangle> = Vec::new();
        let mut materials: Vec<Material> = Vec::new();
        let mut textures: Vec<DynamicImage> = Vec::new();
        // Add materials from config to materials
        materials.append(&mut userconfig.materials);
        println!("Config Sphere count: {}", userconfig.spheres.len());
        println!("Config Material count: {}", materials.len());
        

        // --------Triangles-------------
        // Load OBJ file
        if userconfig.obj_path != "" {
            let (mut obj_triangles,mut obj_materials) = match load_obj(userconfig.obj_path) {
                Err(error) => {
                    // Handle the error
                    eprintln!("Error loading OBJ file: {:?}", error);
                    std::process::exit(1);
                }
                Ok(data) => data,
            };   
            println!("OBJ Triangle count: {}", triangles.len());
            triangles.append(&mut obj_triangles);
            materials.append(&mut obj_materials);
        }

        // Load GLTF file and add to triangles and materials
        if  userconfig.gltf_path != "" {
            let (mut gltf_triangles, mut gltf_materials, mut gltf_textures) = match load_gltf(userconfig.gltf_path, materials.len() as i32, userconfig.textures.len() as i32) {
                Err(error) => {
                    // Handle the error
                    eprintln!("Error loading GLTF file: {:?}", error);
                    std::process::exit(1);
                }
                Ok(data) => data,
            };
            println!("GLTF Triangle count: {}", gltf_triangles.len());
            println!("GLTF Material count: {}", gltf_materials.len());
            triangles.append(&mut gltf_triangles);
            materials.append(&mut gltf_materials);
            textures.append(&mut gltf_textures);
        }

        // Triangles and UV to Uniform buffer
        let mut triangles_uniform: Vec<TriangleUniform> = Vec::new();
        let triangles_count = triangles.len() as i32;
        let times = triangles_count / tris_uv_mapping.len() as i32;

        println!("fill Triangle  buffer");
        for i in 0..triangles_count as usize {
            triangles_uniform.push(TriangleUniform::new(triangles[i], tris_uv_mapping[i % tris_uv_mapping.len()].clone(), times));
        }
        
        // Create a buffer to hold the vertex data
        let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Vertex Buffer"),
            contents: bytemuck::cast_slice(&triangles_uniform),
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
        });

        
        // Load textures from files into a textureset
        let mut textures_buffer = create_texture(&device, &config, 1024, 1024, 30);    //30 = max numer of textures
        let mut texture_count = 0;

        // Add textures from config to textureset
        for i in 0..userconfig.textures.len(){
            for j in 0..3{  //userconfig.textures[i].len(){
                match load_textures(&queue, textures_buffer, &userconfig.textures[i][j], i as i32) {
                    Err(error) => {
                        // Handle the error
                        eprintln!("Error loading texture file: {:?}", error);
                        std::process::exit(1);
                    }
                    Ok(data) => {
                        textures_buffer = data;
                        texture_count += 1;
                    }	
                }
            }
        }

        // Add textures from GLTF to textureset
        for i in 0..textures.len(){
            match load_textures_from_image(&queue, textures_buffer, &textures[i], texture_count + i as i32) {
                Err(error) => {
                    // Handle the error
                    eprintln!("Error loading texture file: {:?}", error);
                    std::process::exit(1);
                }
                Ok(data) => {
                    textures_buffer = data;
                    texture_count += 1;
                }	
            }
        }
        // println!("Texture array size: {}x{}x{} with {} entries", textureset.diffuse.size().width, textureset.diffuse.size().height, textureset.diffuse.size().depth_or_array_layers, texture_count);
        println!("Textures ready ({})", texture_count);

        // ---------Spheres-------------
        // Spheres to Uniform buffer compatible type                                 
        let mut spheres_uniform: Vec<SphereUniform> = Vec::new();
        for sphere in userconfig.spheres.iter(){
            spheres_uniform.push(SphereUniform::new(*sphere));
        }
        
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
        
        // Build BVH for triangles
        println!("AABB generation   0%");
        let aabbs = triangles.iter().map(|t| t.aabb()).collect::<Vec<Aabb>>();
        println!("AABB generation 100%");

        //Add Sphere AABBs
        // for sphere in userconfig.spheres.iter(){
        //     aabbs.push(sphere.aabb());               # Doesnt work because the bvh can only take one type of Data
        // }

        let prim_per_leaf = Some(std::num::NonZeroUsize::new(1).expect("NonZeroUsize creation failed"));
        let primitives = triangles.as_slice();

        let builder = Builder {
            aabbs: Some(aabbs.as_slice()),
            primitives: primitives,
            primitives_per_leaf: prim_per_leaf,
        };
        println!("BVH Builder created");

        // Choose one of these algorithms:
        //let bvh = builder.construct_locally_ordered_clustered().unwrap();
        //let bvh = builder.construct_binned_sah().unwrap();
        let bvh = builder.construct_binned_sah().unwrap();
        println!("BVH generated");

        // Validate the BVH tree
        if bvh.validate(triangles.len()) {
            println!("BVH is valid");
        } else {
            println!("BVH is invalid");
        }

        let raw = bvh.into_raw();
        println!("BVH transformed to raw data");

        //convert format of bvh nodes to uniform buffer compativble
        let mut bvh_uniform: Vec<BvhUniform> = vec![];
        for i in 0..raw.0.len(){
            bvh_uniform.push(BvhUniform::new(&raw.0[i]));
        }
        
        // Store bvh nodes in a buffer as a array
        let bvh_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("BVH Buffer"),
            contents: bytemuck::cast_slice(&bvh_uniform),
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
        });

        // Store prim indices of the bvh nodes in a buffer as a array
        let bvh_prim_indices: Vec<f32> = raw.1.iter().map(|x| *x as f32).collect();
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
        let material_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Material Buffer"),
            contents: bytemuck::cast_slice(&materials),
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
            ],
            label: Some("texture_bind_group"),
        });
        println!("Textures ready");
        
        //----------Raytracing-------------

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
                input:
                    KeyboardInput {
                        virtual_keycode: Some(key),
                        state,
                        ..
                    },
                ..
            } => self.camera_controller.process_keyboard(*key, *state),
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
        println!("FPS: {}", 1.0 / dt.as_secs_f32());
        self.camera_controller.update_camera(&mut self.camera, dt);
        self.camera_uniform.update_view_proj(&self.camera, &self.projection);
        self.camera_uniform.update_frame();
        self.queue.write_buffer(
            &self.camera_buffer,
            0,
            bytemuck::cast_slice(&[self.camera_uniform]),
        );
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
            });
    
            // Set ray tracing pipeline and bind group
            compute_pass.set_pipeline(&self.ray_tracing_pipeline);
            compute_pass.set_bind_group(0, &self.raytracing_bind_group, &[]);
            compute_pass.set_bind_group(1, &self.camera_bind_group, &[]);
            compute_pass.set_bind_group(2, &self.object_bind_group, &[]);
            compute_pass.set_bind_group(3, &self.texture_bind_group, &[]);
            compute_pass.set_bind_group(4, &self.bvh_bind_group, &[]);
    
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
                        store: true,
                    },
                })],
                depth_stencil_attachment: None,
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
    
        // Submit the command encoder
        self.queue.submit(std::iter::once(encoder3.finish()));
    
        // Present the frame
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

    let event_loop = EventLoop::new();
    let title = env!("CARGO_PKG_NAME");
    let window = winit::window::WindowBuilder::new()
        .with_title(title)
        .build(&event_loop)
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

    let mut state = State::new(window).await; // NEW!
    let mut last_render_time = instant::Instant::now();
    event_loop.run(move |event, _, control_flow| {
        *control_flow = ControlFlow::Poll;
        match event {
            Event::MainEventsCleared => state.window().request_redraw(),
            Event::DeviceEvent {
                event: DeviceEvent::MouseMotion{ delta, },
                ..
            } => if state.mouse_pressed {
                state.camera_controller.process_mouse(delta.0, delta.1)
            }
            Event::WindowEvent {
                ref event,
                window_id,
            } if window_id == state.window().id() && !state.input(event) => {
                match event {
                    #[cfg(not(target_arch="wasm32"))]
                    WindowEvent::CloseRequested
                    | WindowEvent::KeyboardInput {
                        input:
                            KeyboardInput {
                                state: ElementState::Pressed,
                                virtual_keycode: Some(VirtualKeyCode::Escape),
                                ..
                            },
                        ..
                    } => *control_flow = ControlFlow::Exit,
                    WindowEvent::Resized(physical_size) => {
                        state.resize(*physical_size);
                    }
                    WindowEvent::ScaleFactorChanged { new_inner_size, .. } => {
                        state.resize(**new_inner_size);
                    }
                    _ => {}
                }
            }
            Event::RedrawRequested(window_id) if window_id == state.window().id() => {
                let now = instant::Instant::now();
                let dt = now - last_render_time;
                last_render_time = now;
                state.update(dt);
                match state.render() {
                    Ok(_) => {}
                    // Reconfigure the surface if it's lost or outdated
                    Err(wgpu::SurfaceError::Lost | wgpu::SurfaceError::Outdated) => state.resize(state.size),
                    // The system is out of memory, we should probably quit
                    Err(wgpu::SurfaceError::OutOfMemory) => *control_flow = ControlFlow::Exit,
                    // We're ignoring timeouts
                    Err(wgpu::SurfaceError::Timeout) => log::warn!("Surface timeout"),
                }
            }
            _ => {}
        }
    });
}