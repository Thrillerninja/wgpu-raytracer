use scene::config::Config;

use wgpu::Features;
use winit::window::Window;


pub async fn setup_gpu<'a> (window: Window) -> (Window, wgpu::Device, wgpu::Queue, wgpu::Surface<'a> , wgpu::SurfaceConfiguration, wgpu::TextureView, Config, winit::dpi::PhysicalSize<u32>) {
    
    let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
        backends: wgpu::Backends::DX12,
        dx12_shader_compiler: Default::default(),
        gles_minor_version: wgpu::Gles3MinorVersion::Automatic,
        flags: wgpu::InstanceFlags::empty(),
    });

    // This unsafe is strictly nessesary for the GPU
    // It is not possible to create a surface without it
    // Its because of the way of communication with the gpu
    let surface_result = unsafe {
        instance.create_surface_unsafe(wgpu::SurfaceTargetUnsafe::from_window(&window).unwrap())
    };

    let surface = match surface_result {
        Ok(surface) => surface,
        Err(error) => {
            // Handle the error here
            panic!("Failed to create surface: {:?}", error);
        }
    };

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
                required_features: Features::TEXTURE_ADAPTER_SPECIFIC_FORMAT_FEATURES,
                label: None,
                required_limits: wgpu::Limits {
                    max_bind_groups: 6, // Not every old GPU supports more than 4 bind groups, 
                                        // but should be no problem today. Either way, it makes the buffers better structured
                    ..Default::default()
                }
            },
            None,
        )
        .await
        .unwrap();

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
        desired_maximum_frame_latency: 10,
    };
    surface.configure(&device, &config);     
    
    let userconfig = Config::new();

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

    return (window, device, queue, surface, config, color_buffer_view, userconfig, size)
}