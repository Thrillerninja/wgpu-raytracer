use scene::Config;
use wgpu::Features;
use winit::window::Window;


pub async fn setup_gpu<'a> (window: Window, config_path: &str) -> (Window, wgpu::Device, wgpu::Queue, wgpu::Surface<'a> , wgpu::SurfaceConfiguration, wgpu::TextureView, Config, winit::dpi::PhysicalSize<u32>) {
    
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
    
    let userconfig = Config::new(config_path);

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


#[cfg(test)]
mod tests {
    use winit_test::winit::event_loop::EventLoopWindowTarget;
    use pollster::block_on;
    use super::*;

    // #[warn(dead_code)]  //Suppresses warning for unused function since it is used by the winit testing framework below
    fn _test_setup_gpu(elwt: &EventLoopWindowTarget<()>) {
        let window = winit::window::WindowBuilder::new()
            .with_inner_size(winit::dpi::PhysicalSize::new(800, 600))
            .build(&elwt)
            .unwrap();

        let (window, device, _queue, _surface, config, _color_buffer_view, _userconfig, size) = block_on(setup_gpu(window, "config.toml"));

        assert_eq!(config.width, 800);  //Checks if config is set correctly
        assert_eq!(config.height, 600);
        assert_eq!(size.width, 800);    //Checks if size is set correctly
        assert_eq!(size.height, 600);
        assert_eq!(window.inner_size().width, 800); //Checks if window size is set correctly
        assert_eq!(window.inner_size().height, 600);
        assert_eq!(device.limits().max_bind_groups, 6); //Checks if custom limits are set
    }

    winit_test::main!(_test_setup_gpu);

}