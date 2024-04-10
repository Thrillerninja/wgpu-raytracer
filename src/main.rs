use std::collections::VecDeque;
use winit::{event::*, event_loop::{ControlFlow, EventLoop}, keyboard::{Key, NamedKey}, window::Window};
use egui_wgpu::ScreenDescriptor;

#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::*;

mod wgpu_utils;
use wgpu_utils::setup_gpu;
use wgpu_utils::{BufferInitDescriptor, BindGroupDescriptor, BufferType, BindingResourceTemplate, create_new_buffer};

mod gui;
use gui::{EguiRenderer, gui};

mod camera;
use camera::Camera;

mod models;
mod texture;
mod config;

mod structs;
use structs::CameraUniform;

mod renderer;
use renderer::setup_camera;

use crate::{models::{load_exr, load_hdr, load_hdri}, renderer::{setup_bvh, setup_hdri, setup_textures, setup_tris_objects}, structs::Background};

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
    /// Constructs a new `State` instance.
    /// 
    /// This function initializes the gpu, sets up the camera and objects, sets up the render pipelines for raytracing, denoising and screen rendering, and initializes the GUI.
    /// # Arguments
    /// * `window` - A `Window` instance representing the window in which the state will be rendered.
    /// # Returns
    /// * `Self` - A new `State` instance.
    /// # Asynchronous
    /// This function is asynchronous and must be awaited.
    /// 
    /// # Gpu Setup
    /// The gpu setup involves creating an instance that serves as a handle to our GPU. It also sets up the surface, config, color buffer view, userconfig, and size.
    /// # Camera Setup
    /// The camera setup involves creating a camera, projection, camera controller, and camera uniform. It also creates a buffer to hold the camera data and a bind group for the camera.
    /// # Object Setup
    /// The object setup involves creating triangles, triangles uniform, materials, and textures. It also creates a buffer to hold the vertex data and a bind group for the objects.
    /// * # Sphere Setup
    /// * The sphere setup involves creating spheres uniform and a buffer to hold the sphere data. It also creates a bind group for the spheres.
    /// * # Triangle Setup
    /// * The triangle setup involves creating a buffer to hold the triangle data and a bind group for the triangles.
    /// * # Texture Setup
    /// * The texture setup involves creating a texture set and a buffer to hold the texture data. It also creates a bind group for the textures.
    /// # BVH Setup
    /// The BVH setup involves creating a buffer to hold the BVH nodes and a buffer to hold the primitive indices of the BVH nodes. It also creates a bind group for the BVH nodes.
    /// # Raytracing Setup
    /// The raytracing setup involves creating a buffer to hold the shader config data and a bind group for the shader config. It also creates a raytracing pipeline and a bind group for the raytracing pipeline. It loads the raytracing shader and creates a pipeline layout for raytracing.
    /// # Denoising Setup
    /// The denoising setup involves creating a denoising buffer and a bind group for it. It also passes camera info to the denoising shader and creates a buffer to hold the camera data for denoising. It also creates a buffer to hold the denoising pass number, a view for the denoising texture, a bind group descriptor for the denoising step, and a pipeline layout for denoising. Finally, it loads the denoising shader and creates a denoising pipeline.
    /// # Screen rendering Setup
    /// The screen rendering setup involves creating a sampler for transferring color data from render to screen texture. It also creates a bind group layout for the shader and a bind group for the screen rendering pipeline. It loads the screen shader and creates a screen pipeline layout.
    async fn new(window: Window) -> Self {
        //---------Setup Hardware---------
        let (window,
            device, 
            queue, 
            surface, 
            config, 
            color_buffer_view, 
            userconfig, 
            size) = setup_gpu(window).await;
        println!("Hardware initialized");

        //-------------Camera-------------
        // Create a camera with configured settings
        let (camera, 
            projection, 
            camera_controller, 
            camera_uniform) = setup_camera(&config, &userconfig);

        // Create a buffer to hold the camera data
        let camera_descriptor = BufferInitDescriptor::new(Some("Camera Buffer"), wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::COPY_SRC);
        let camera_buffer = create_new_buffer(&device, &[camera_uniform], camera_descriptor);

        // Create a bind group for pasing the camera data to the shader
        let mut camera_bind_group_descriptor = BindGroupDescriptor::new(
            Some("camera"),
            wgpu::ShaderStages::COMPUTE,
            vec![BufferType {
                ty: BindingResourceTemplate::BufferUniform(
                    camera_buffer.as_entire_binding()
                ),
                view_dimension: None,
            }]
        );
        let camera_bind_group = camera_bind_group_descriptor.generate_bind_group(&device);
        let camera_bind_group_layout = camera_bind_group_descriptor.layout.unwrap();
        println!("Camera ready");

        //============== Load Render Objects ==============
        //---------- Load Triangles(Vertecies) ----------
        let (triangles, 
            triangles_uniform, 
            materials, 
            textures) = setup_tris_objects(&userconfig);

        // Create a buffer to hold the vertex data of the triangles
        let vertex_buffer_descriptor = BufferInitDescriptor::new(Some("Vertex Buffer"), wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST);
        let vertex_buffer = create_new_buffer(&device, &triangles_uniform, vertex_buffer_descriptor);

        // --------- Load Spheres ---------
        // Load spheres amd store them as gpu compatible vector
        let spheres = match userconfig.spheres {
            Some(spheres) => {
                spheres
            }
            None => vec![]
        };
        
        // Create a buffer to hold the sphere data
        let sphere_buffer_descriptor = BufferInitDescriptor::new(Some("Sphere Buffer"), wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST);
        let sphere_buffer = create_new_buffer(&device, &spheres, sphere_buffer_descriptor);

        // ------ Combined Bind Group ---------
        // Create a bind group for the objects
        let mut object_bind_group_descriptor = BindGroupDescriptor::new(
            Some("object_bind_group"),
            wgpu::ShaderStages::COMPUTE,
            vec![
                BufferType {
                    ty: BindingResourceTemplate::BufferStorage(
                        vertex_buffer.as_entire_binding()
                    ),
                    view_dimension: None,
                },
                BufferType {
                    ty: BindingResourceTemplate::BufferStorage(
                        sphere_buffer.as_entire_binding()
                    ),
                    view_dimension: None,
                }
            ]
        );

        // Generate the object bind group & layout
        let object_bind_group = object_bind_group_descriptor.generate_bind_group(&device);
        let object_bind_group_layout = object_bind_group_descriptor.layout.unwrap();
        println!("Meshes ready");

        //-------------BVH---------------
        //-This only works for triangles-

        // Create a bvh for the triangles
        let (bvh_uniform, bvh_prim_indices) = setup_bvh(&triangles);
        
        // Store bvh nodes in a buffer as a array
        let bvh_descriptor = BufferInitDescriptor::new(Some("BVH Buffer"), wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST);
        let bvh_buffer = create_new_buffer(&device, &bvh_uniform, bvh_descriptor);

        // Store prim indices of the bvh nodes in a buffer as a array (these are needed for a tree traversal on the gpu)
        let bvh_indices_descriptor = BufferInitDescriptor::new(Some("BVH Prim Indices Buffer"), wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST);
        let bvh_prim_indices_buffer = create_new_buffer(&device, &bvh_prim_indices, bvh_indices_descriptor);

        // Send nodes and prim indices to the shader
        let mut bvh_bind_group_descriptor = BindGroupDescriptor::new(
            Some("bvh"),
            wgpu::ShaderStages::COMPUTE,
            vec![
                BufferType {
                    ty: BindingResourceTemplate::BufferStorage(
                        bvh_buffer.as_entire_binding()
                    ),
                    view_dimension: None,
                },
                BufferType {
                    ty: BindingResourceTemplate::BufferStorage(
                        bvh_prim_indices_buffer.as_entire_binding()
                    ),
                    view_dimension: None,
                }
            ]
        );

        // Generate the bvh bind group & layout
        let bvh_bind_group = bvh_bind_group_descriptor.generate_bind_group(&device);
        let bvh_bind_goup_layout = bvh_bind_group_descriptor.layout.unwrap();
        println!("BVH ready");

        //------Textures & Materials------
        // Create 3D textures with textures from config and glft or background hdri 
        let textures_buffer = setup_textures(&userconfig, textures, &device, &queue, &config);
        let background_texture = setup_hdri(&userconfig, &device, &queue, &config);

        // Create a buffer to hold the material data from config and glft
        let material_descriptor = BufferInitDescriptor::new(Some("Material Buffer"), wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST);
        let material_buffer = create_new_buffer(&device, &materials, material_descriptor);
        
        // Background
        let background = match userconfig.background {
            Some(background) => {
                background
            }
            None => Background::default()
        };
        // Create a buffer to hold the extra data for the background
        let background_descriptor = BufferInitDescriptor::new(Some("Background Buffer"), wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST);
        let background_buffer = create_new_buffer(&device, &[background], background_descriptor);

        // Create a sampler for all textures
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

        // Create a bind group for the textures, materials and background
        let textures_view = textures_buffer.create_view(&wgpu::TextureViewDescriptor::default());
        let background_texture_view = background_texture.create_view(&wgpu::TextureViewDescriptor::default());
        let mut texture_bind_group_descriptor = BindGroupDescriptor::new(
            Some("textures&materials"),
            wgpu::ShaderStages::COMPUTE,
            vec![
                BufferType {
                    ty: BindingResourceTemplate::Sampler(
                        wgpu::BindingResource::Sampler(&texture_sampler)
                    ),
                    view_dimension: None,
                },
                BufferType {
                    ty: BindingResourceTemplate::TextureView(
                        wgpu::BindingResource::TextureView(&textures_view)
                    ),
                    view_dimension: Some(wgpu::TextureViewDimension::D2Array),
                },
                BufferType {
                    ty: BindingResourceTemplate::BufferStorage(
                        material_buffer.as_entire_binding()
                    ),
                    view_dimension: None,
                },
                BufferType {
                    ty: BindingResourceTemplate::BufferStorage(
                        background_buffer.as_entire_binding()
                    ),
                    view_dimension: None,
                },
                BufferType {
                    ty: BindingResourceTemplate::TextureView(
                        wgpu::BindingResource::TextureView(&background_texture_view)
                    ),
                    view_dimension: Some(wgpu::TextureViewDimension::D2),
                }
            ]
        );

        // Generate the texture bind group & layout
        let texture_bind_group = texture_bind_group_descriptor.generate_bind_group(&device);
        let texture_bind_group_layout = texture_bind_group_descriptor.layout.unwrap();
        println!("Textures ready");

        //============= Shader&Pipeline Setup =============

        //--------Shader config-----------
        // Initialize shader config
        let shader_config = structs::ShaderConfig::default();
        // Create a buffer to hold the shader config data
        let shader_config_descriptor = BufferInitDescriptor::new(Some("Shader Config Buffer"), wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST);
        let shader_config_buffer =  create_new_buffer(&device, &[shader_config], shader_config_descriptor);

        // Create a bind group for pasing the shader config to the shader
        let mut shader_config_bind_group_descriptor = BindGroupDescriptor::new(
            Some("shader_config"),
            wgpu::ShaderStages::COMPUTE,
            vec![
                BufferType {
                    ty: BindingResourceTemplate::BufferUniform(
                        shader_config_buffer.as_entire_binding()
                    ),
                    view_dimension: None,
                }
            ]
        );
        // Generate the shader config bind group & layout
        let shader_config_bind_group = shader_config_bind_group_descriptor.generate_bind_group(&device);
        let shader_config_bind_group_layout = shader_config_bind_group_descriptor.layout.unwrap();
        println!("Shader config ready");

        //----------Raytracing-------------
        // Load the ray tracing shader
        let ray_generation_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Ray Generation Shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("raygen.wgsl").into()), // Replace with your actual shader source
        });

        // Create the bind group layout for the shader
        let mut raytracing_bind_group_descriptior = BindGroupDescriptor::new(
            Some("raytracing"),
            wgpu::ShaderStages::COMPUTE,
            vec![
                BufferType {
                    ty: BindingResourceTemplate::StorageTexture(
                        wgpu::BindingResource::TextureView(&color_buffer_view)
                    ),
                    view_dimension: Some(wgpu::TextureViewDimension::D2),
                }
            ]
        );

        // Generate the raytracing bind group & layout
        let raytracing_bind_group = raytracing_bind_group_descriptior.generate_bind_group(&device);
        let raytracing_bind_group_layout = raytracing_bind_group_descriptior.layout.unwrap();

        // Create the ray tracing pipeline layout
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
        // Create the ray tracing pipeline
        let ray_tracing_pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("Ray Tracing Pipeline"),
            layout: Some(&raytracing_pipeline_layout),
            module: &ray_generation_shader,
            entry_point: "main",
            }
        );
        println!("Raytracing shader&pipeline ready");

        //--------Denoising pass----------
        // Load the denoising shader
        let denoising_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Denoising Shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("denoising.wgsl").into()), // Replace with your actual shader source
        });

        // Define Texture to store the temporal denoising result to use it in the next frame again for temporal denoising
        let denoising_texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("Denoising Buffer"),
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
        // Create a view for the denoising texture
        let denoising_texture_view = denoising_texture.create_view(&wgpu::TextureViewDescriptor::default());

        // ~~~Pass camera info to denoising shader~~~
        let denoising_camera: Camera = camera.clone();
        let mut denoising_camera_uniform = CameraUniform::new();
        denoising_camera_uniform.update_view_proj(&denoising_camera, &projection);
        
        // Create a buffer to hold the camera data for the denoising shader so it can be used to detect significant scene change
        let denoising_camera_buffer_descriptor = BufferInitDescriptor::new(Some("Denoising Camera Data Buffer"), wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST);
        let denoising_camera_buffer = create_new_buffer(&device, &[denoising_camera_uniform], denoising_camera_buffer_descriptor);

        // Create a buffer to hold the denoising pass number so the correct denoising step (temporal or spatial) can be executed
        let denoising_pass_buffer_descriptor = BufferInitDescriptor::new(Some("Denoising Pass Buffer"), wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST);
        let denoising_pass_buffer = create_new_buffer(&device, &[0u32], denoising_pass_buffer_descriptor);

        // Create a bind group descriptor for denoising step
        let mut denoising_bind_group_descriptor = BindGroupDescriptor::new(
            Some("denoising"),
            wgpu::ShaderStages::COMPUTE,
            vec![
                BufferType {
                    ty: BindingResourceTemplate::StorageTexture(
                        wgpu::BindingResource::TextureView(&color_buffer_view),
                    ),
                    view_dimension: Some(wgpu::TextureViewDimension::D2),
                },
                BufferType {
                    ty: BindingResourceTemplate::StorageTexture(
                        wgpu::BindingResource::TextureView(&denoising_texture_view),
                    ),
                    view_dimension: Some(wgpu::TextureViewDimension::D2),
                },
                BufferType {
                    ty: BindingResourceTemplate::BufferUniform(
                        camera_buffer.as_entire_binding()
                    ),
                    view_dimension: None,
                },
                BufferType {
                    ty: BindingResourceTemplate::BufferUniform(
                        denoising_camera_buffer.as_entire_binding()
                    ),
                    view_dimension: None,
                },
                BufferType {
                    ty: BindingResourceTemplate::BufferUniform(
                        denoising_pass_buffer.as_entire_binding()
                    ),
                    view_dimension: None,
                }
            ]
        );
        // Generate the denoising bind group & layout
        let denoising_bind_group = denoising_bind_group_descriptor.generate_bind_group(&device);
        let denoising_bind_group_layout = denoising_bind_group_descriptor.layout.unwrap();

        // Create a pipeline layout for denoising
        let denoising_pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Denoising Pipeline Layout"),
            bind_group_layouts: &[
                &denoising_bind_group_layout],
            push_constant_ranges: &[],
        });

        // Create the denoising pipeline
        let denoising_pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("Denoising Pipeline"),
            layout: Some(&denoising_pipeline_layout),
            module: &denoising_shader,
            entry_point: "main", // Change to your actual entry point name
        });
        println!("Denoising shader&pipeline ready");

        //----------Transfer to screen-------------
        // Load the screen transfer shader
        let screen_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Screen Transfer Shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("screen-shader.wgsl").into()),
        });
        // Create a Sampler for trasfering color data from rendered texture to screen texture
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
        let mut screen_bind_group_descriptor = BindGroupDescriptor::new(
            Some("screen_transfer"),
            wgpu::ShaderStages::FRAGMENT,
            vec![
                BufferType {
                    ty: BindingResourceTemplate::Sampler(
                        wgpu::BindingResource::Sampler(&sampler)
                    ),
                    view_dimension: None,
                },
                BufferType {
                    ty: BindingResourceTemplate::TextureView(
                        wgpu::BindingResource::TextureView(&color_buffer_view)
                    ),
                    view_dimension: Some(wgpu::TextureViewDimension::D2),
                }
            ]
        );

        // Generate the screen bind group & layout
        let screen_bind_group = screen_bind_group_descriptor.generate_bind_group(&device);
        let screen_bind_group_layout = screen_bind_group_descriptor.layout.unwrap();    

        // Create the pipeline to display render result
        let screen_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("Screen Transfer Pipeline Layout"),
                bind_group_layouts: &[&screen_bind_group_layout],
                push_constant_ranges: &[],
            });
        
        let screen_render_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Screen Transfer Pipeline"),
            layout: Some(&screen_pipeline_layout),
            vertex: wgpu::VertexState {
                module: &screen_shader,
                entry_point: "vs_main", // Entrypoint for vertex shader
                buffers: &[],
            },
            fragment: Some(wgpu::FragmentState {
                module: &screen_shader,
                entry_point: "fs_main", // Entrypoint for fragment shader
                targets: &[
                    Some(wgpu::ColorTargetState {
                    format: config.format,
                    blend: Some(wgpu::BlendState::REPLACE),
                    write_mask: wgpu::ColorWrites::ALL,
                    })
                ],
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
        println!("Screen transfer shader&pipeline ready");


        //=============== GUI config (not directly in contact with wgpu) ===============
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