use std::path::Path;
use image::{DynamicImage, GenericImageView};
use wgpu::{Device, Queue, Texture, TextureDescriptor, TextureDimension, TextureFormat, TextureUsages};

// Load an image from a file and return it as DynamicImage
fn load_image(file_path: &str) -> DynamicImage {
    image::open(file_path).expect("Failed to load image")
}

// Load textures from image files and store them in a texture array
pub fn load_textures_to_array(device: &Device, queue: &Queue, file_paths: Vec<&str>, config: wgpu::SurfaceConfiguration) -> wgpu::Texture {
    // Load images and calculate the size of the texture array
    let images: Vec<DynamicImage> = file_paths.iter().map(|&path| load_image(path)).collect();
    let max_texture_size: u32 = 32; // Adjust as needed
    let texture_width = max_texture_size;
    let texture_height = max_texture_size;
    let num_textures = images.len() as u32;
    
    // Create a texture array
    let texture = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("Texture Array"),
        view_formats: &[config.format], // Use sRGB format for storage
        size: wgpu::Extent3d {
            width: texture_width,
            height: texture_height,
            depth_or_array_layers: num_textures,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: TextureDimension::D2,
        format: TextureFormat::Rgba8Unorm, // Adjust format as needed
        usage: wgpu::TextureUsages::COPY_DST | wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::RENDER_ATTACHMENT,
    });

    // Copy images into the texture array
    for (index, image) in images.iter().enumerate() {
        let offset = wgpu::Origin3d {
            x: 0,
            y: 0,
            z: index as u32,
        };
        
        let (width, height) = image.dimensions();
        let bytes_per_pixel = 4; // Assuming RGBA8Unorm format
        let bytes_per_row = width * bytes_per_pixel;
        let image_data = image.to_rgba8().into_raw();

        queue.write_texture(
            wgpu::ImageCopyTexture {
                texture: &texture,
                mip_level: 0,
                origin: offset,
                aspect: wgpu::TextureAspect::All,
            },
            &image_data,
            wgpu::ImageDataLayout {
                offset: 0,
                bytes_per_row: bytes_per_row.into(),
                rows_per_image: height.into(),
            },
            wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
        );
    }

    texture
}