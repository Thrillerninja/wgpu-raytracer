use image::{DynamicImage, GenericImageView};
use wgpu::{Device, Queue, Texture, TextureDimension, TextureFormat, SurfaceConfiguration};

pub fn create_texture(device: &Device, config: &SurfaceConfiguration, texture_width: u32, texture_height: u32, num_textures: u32) -> Texture {
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
        usage: wgpu::TextureUsages::COPY_DST | 
               wgpu::TextureUsages::TEXTURE_BINDING | 
               wgpu::TextureUsages::RENDER_ATTACHMENT,
    });   
}

fn write_texture(queue: &Queue, texture: &Texture, image: &DynamicImage, offset: wgpu::Origin3d) {
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

pub fn load_textures_from_image(queue: &Queue, textureset: Texture, image: &DynamicImage, index: i32) -> Result<Texture, Box<dyn std::error::Error>> {
    let offset = wgpu::Origin3d {
        x: 0,
        y: 0,
        z: index as u32,
    };

    write_texture(queue, &textureset, image, offset);

    Ok(textureset)
}

//index only added for easier debugging
pub fn scale_texture(texture: &DynamicImage, width: u32, height: u32, _index: i32) -> DynamicImage {
    // Inspect images: if uncommented
    // Save the original texture
    // let original_path = format!("textures_{}_original.png", index);
    // let res = texture.save(original_path);
    // match res {
    //     Ok(_) => {
    //         println!("IMG successfully saved")
    //     }
    //     Err(res) => {
    //         println!("Failed to save IMG {:?}", res)
    //     }
    // }

    // Resize the texture
    let resized_texture = texture.resize(width, height, image::imageops::FilterType::Nearest);

    // Save the resized texture
    // let resized_path = format!("textures_{}_resized.png", index);
    // let res = resized_texture.save(resized_path);
    // match res {
    //     Ok(_) => {
    //         println!("IMG successfully saved (resized)")
    //     }
    //     Err(res) => {
    //         println!("Failed to save IMG (resized) {:?}", res)
    //     }
    // }
    return resized_texture;
}