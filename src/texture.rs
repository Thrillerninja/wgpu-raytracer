use image::{DynamicImage, GenericImageView};
use wgpu::{Device, Queue, Texture, TextureDimension, TextureFormat, SurfaceConfiguration};
use crate::structs::TextureSet;

// Load an image from a file and return it as DynamicImage
fn load_image(file_path: &str) -> Result<DynamicImage, Box<dyn std::error::Error>> {
    image::open(file_path).map_err(|err| format!("Failed to load texture from {}: {}", file_path, err).into())
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

fn write_texture(queue: &Queue, texture: &Texture, image: DynamicImage, offset: wgpu::Origin3d) {
    let (width, height) = image.dimensions();
    let bytes_per_pixel = 4; // Assuming RGBA8Unorm format
    let bytes_per_row = width * bytes_per_pixel;
    let data = image.to_rgba8().into_raw();

    queue.write_texture(
        wgpu::ImageCopyTexture {
            texture,
            mip_level: 0,
            origin: offset,
            aspect: wgpu::TextureAspect::All,
        },
        &data,
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

pub fn load_texture_set(queue: &Queue, textureset: TextureSet, diffuse: &str, normal: &str, roughness: &str, index: i32) -> Result<TextureSet, Box<dyn std::error::Error>> {
    let offset = wgpu::Origin3d {
        x: 0,
        y: 0,
        z: index as u32,
    };

    // Diffuse
    let diffuse_image = load_image(diffuse)?;
    write_texture(queue, &textureset.diffuse, diffuse_image, offset);

    // Normal
    let normal_image = load_image(normal)?;
    write_texture(queue, &textureset.normal, normal_image, offset);

    // Roughness
    let roughness_image = load_image(roughness)?;
    write_texture(queue, &textureset.roughness, roughness_image, offset);

    Ok(textureset)
}

pub fn load_texture_set_from_images(queue: &Queue, textureset: TextureSet, diffuse: &DynamicImage, normal: &DynamicImage, roughness: &DynamicImage, index: i32) -> Result<TextureSet, Box<dyn std::error::Error>> {
    let offset = wgpu::Origin3d {
        x: 0,
        y: 0,
        z: index as u32,
    };

    // Diffuse
    write_texture(queue, &textureset.diffuse, diffuse.clone(), offset);

    // Normal
    write_texture(queue, &textureset.normal, normal.clone(), offset);

    // Roughness
    write_texture(queue, &textureset.roughness, roughness.clone(), offset);

    Ok(textureset)
}

pub fn load_texture_set_from_image(queue: &Queue, textureset: TextureSet, texture: &DynamicImage, index: i32) -> Result<TextureSet, Box<dyn std::error::Error>> {
    let offset = wgpu::Origin3d {
        x: 0,
        y: 0,
        z: index as u32,
    };

    write_texture(queue, &textureset.diffuse, texture.clone(), offset);

    Ok(textureset)
}