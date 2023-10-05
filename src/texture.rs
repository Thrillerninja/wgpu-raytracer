use std::path::Path;
use image::{DynamicImage, GenericImageView};
use wgpu::{Device, Queue, Texture, TextureDescriptor, TextureDimension, TextureFormat, TextureUsages, SurfaceConfiguration};


pub struct TextureSet{
    pub diffuse: wgpu::Texture,
    pub normal: wgpu::Texture,
    pub roughness: wgpu::Texture,
}

// Load an image from a file and return it as DynamicImage
fn load_image(file_path: &str) -> DynamicImage {
    match image::open(file_path) {
        Ok(image) => image,
        Err(err) => panic!("Failed to load image from {}: {}", file_path, err),
    }
}
fn create_texture(device: &Device, config: &SurfaceConfiguration, texture_width: u32, texture_height: u32, num_textures: u32) -> Texture {
    return device.create_texture(&wgpu::TextureDescriptor {
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
}

pub fn create_textureset(device: &Device, config: &SurfaceConfiguration, texture_width: u32, texture_height: u32, num_textures: u32) -> TextureSet {
    // Create a textureset
    let diffuse = create_texture(device, config, texture_width, texture_height, num_textures);
    let normal = create_texture(device, config, texture_width, texture_height, num_textures);
    let roughness = create_texture(device, config, texture_width, texture_height, num_textures);

    return TextureSet {
        diffuse,
        normal,
        roughness,
    }
}

pub fn load_texture_set(device: &Device, queue: &Queue, config: &wgpu::SurfaceConfiguration, textureset: TextureSet, diffuse: &str, normal: &str, roughness: &str, index: i32) -> TextureSet {
    //add textures to textureset
    let diffuse_image   = load_image(diffuse);
    let normal_image    = load_image(normal);
    let roughness_image = load_image(roughness);

    let offset = wgpu::Origin3d {
        x: 0,
        y: 0,
        z: index as u32,
    };
    let (width, height) = diffuse_image.dimensions();
    let bytes_per_pixel = 4; // Assuming RGBA8Unorm format
    let bytes_per_row = width * bytes_per_pixel;

    // Diffuse
    let basecolor_data = diffuse_image.to_rgba8().into_raw();
    queue.write_texture(
        wgpu::ImageCopyTexture {
            texture: &textureset.diffuse,
            mip_level: 0,
            origin: offset,
            aspect: wgpu::TextureAspect::All,
        },
        &basecolor_data,
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

    // Normal
    let normal_data = normal_image.to_rgba8().into_raw();
    queue.write_texture(
        wgpu::ImageCopyTexture {
            texture: &textureset.normal,
            mip_level: 0,
            origin: offset,
            aspect: wgpu::TextureAspect::All,
        },
        &normal_data,
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

    return textureset;
}

// Load textures from image files and store them in a texture array
pub fn load_textures_to_array(device: &Device, queue: &Queue, file_paths: Vec<&str>, config: wgpu::SurfaceConfiguration) -> wgpu::Texture {
    // Load images and calculate the size of the texture array
    let images: Vec<DynamicImage> = file_paths.iter().map(|&path| load_image(path)).collect();
    let max_texture_size: u32 = 1024; // Adjust as needed
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