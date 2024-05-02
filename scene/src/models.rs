use std::fs::File;
use std::io::{BufRead, BufReader};
use image::{DynamicImage, ImageBuffer, Rgba};
use crate::structs::{Triangle, Material};

use core::ops::Deref;
use image::Pixel;
use exr;

pub fn load_obj(file_path: String, obj_material_id: i32) -> Result<(Vec<Triangle>, Vec<Material>), Box<dyn std::error::Error>> {
    let file = File::open(file_path)?;
    let reader = BufReader::new(file);

    let mut vertices = Vec::new();
    let mut texture_coords = Vec::new();
    let mut normals = Vec::new();
    let mut faces: Vec<Triangle> = Vec::new();

    for line in reader.lines() {
        let line = line?;
        let line = line.trim();

        if line.starts_with("v ") {
            // Parse vertex coordinates
            let values: Vec<f32> = line[2..]
                .split_whitespace()
                .map(|x| x.parse::<f32>())
                .collect::<Result<_, _>>()?;

            if values.len() >= 3 {
                let vertex = [values[0], values[1], values[2]];
                vertices.push(vertex);
            }
        } else if line.starts_with("vt ") {
            // Parse texture coordinates
            let values: Vec<f32> = line[3..]
                .split_whitespace()
                .map(|x| x.parse::<f32>())
                .collect::<Result<_, _>>()?;

            if values.len() >= 2 {
                let tex_coord = [values[0], values[1]];
                texture_coords.push(tex_coord);
            }
        } else if line.starts_with("vn ") {
            // Parse normals
            let val: Vec<f32> = line[3..]
                .split_whitespace()
                .map(|x| x.parse::<f32>())
                .collect::<Result<_, _>>()?;

            if val.len() >= 3 {
                let normal = [val[0], val[1], val[2]];
                normals.push(normal);
            }
        } else if line.starts_with("f ") {
            // Parse face indices
            let indices: Vec<(usize, usize, usize)> = line[2..]
                .split_whitespace()
                .map(|x| {
                    let indices: Vec<usize> = x
                        .split('/')
                        .map(|y| y.parse::<usize>())
                        .collect::<Result<_, _>>()
                        .unwrap();
                    (indices[0], indices[1], indices[2])
                })
                .collect();
        
            if indices.len() >= 3 {
                let v1_index = indices[0].0 - 1;
                let v2_index = indices[1].0 - 1;
                let v3_index = indices[2].0 - 1;
                let normal_index = indices[0].2 - 1;

                // let mut rng = rand::thread_rng();
                // let r: f32 = rng.gen_range(0.0..1.0);
                // let g: f32 = rng.gen_range(0.0..1.0);
                // let b: f32 = rng.gen_range(0.0..1.0);
        
                let triangle = Triangle::new(
                    [
                        vertices[v1_index],
                        vertices[v2_index],
                        vertices[v3_index],
                    ],
                    normals[normal_index],
                    obj_material_id,
                    [-1.0, -1.0, -1.0],
                    [
                        texture_coords[indices[0].1 - 1],
                        texture_coords[indices[1].1 - 1],
                        texture_coords[indices[2].1 - 1],
                    ],
                );
                faces.push(triangle);
            }
        }
    }

    Ok((faces,Vec::new()))
}

pub fn load_gltf(path: String, material_count: i32, texture_count: i32) -> Result<(Vec<Triangle>, Vec<Material>, Vec<DynamicImage>), Box<dyn std::error::Error>> {
    let scenes = easy_gltf::load(path).expect("Failed to load glTF");
    let mut converted_triangles = Vec::new();
    let mut converted_materials = Vec::new();
    let mut material_index = material_count;
    let mut texture_index = texture_count;  // jet unused
    let mut textures: Vec<DynamicImage> = Vec::new();

    for scene in scenes {
        println!(
            "Cameras: #{}  Lights: #{}  Models: #{}  Textures: #{} in GLFT scene",
            scene.cameras.len(),
            scene.lights.len(),
            scene.models.len(),
            texture_index
        );

        for model in scene.models {
            let material = model.material();

            match &material.pbr.base_color_texture {
                Some(texture) => {
                    println!("Texture dimensions: {:?}", texture.dimensions());
                }
                None => {
                    println!("No texture found");
                }
            }

            // Convert material to own format
            let base_color_factor = material.pbr.base_color_factor;
            let roughness_factor = material.pbr.roughness_factor;

            converted_materials.push(Material::new(
                [base_color_factor[0], base_color_factor[1], base_color_factor[2]],
                [0.6;3], // if dielectric it should be [1.0]
                roughness_factor,
                material.emissive.factor[0],    // emissive_factor is returned as rgb but we only use the first value
                0.0
            ));


            // Convert textures to own format
            let mut has_base_color_texture = false;
            let mut has_roughness_texture = false;
            let mut has_normal_texture = false;
            let mut has_emissive_texture = false;

            if let Some(base_color_texture) = &material.pbr.base_color_texture {
                let base_color_image = convert_to_dynamic_image(base_color_texture);
                textures.push(base_color_image);
                texture_index += 1;
                has_base_color_texture = true;
            }
            if let Some(roughness_texture) = &material.pbr.roughness_texture {
                let roughness_image = convert_to_dynamic_image(roughness_texture);
                textures.push(roughness_image);
                texture_index += 1;
                has_roughness_texture = true;
            }
            if let Some(normal) = &material.normal {
                let normal_image = convert_to_dynamic_image(&normal.texture);
                textures.push(normal_image);
                texture_index += 1;
                has_normal_texture = true;
            }
            if let Some(emissive) = &material.emissive.texture {
                let emissive_image = convert_to_dynamic_image(emissive);
                textures.push(emissive_image);
                texture_index += 1;
                has_emissive_texture = true;
            }

            let mut texture_ids = [-1,-1,-1];

            if has_base_color_texture && has_roughness_texture && has_normal_texture && has_emissive_texture {
                texture_ids[0] = texture_index - 4;
                texture_ids[1] = texture_index - 3;
                texture_ids[2] = texture_index - 2;
                // texture_ids[3] = texture_index - 1;
            } else if has_base_color_texture && has_roughness_texture && has_normal_texture {
                texture_ids[0] = texture_index - 3;
                texture_ids[1] = texture_index - 2;
                texture_ids[2] = texture_index - 1;
            } else if has_base_color_texture && has_roughness_texture && has_emissive_texture {
                texture_ids[0] = texture_index - 3;
                texture_ids[1] = texture_index - 2;
                // texture_ids[3] = texture_index - 1;
            } else if has_base_color_texture && has_normal_texture && has_emissive_texture {
                texture_ids[0] = texture_index - 3;
                texture_ids[2] = texture_index - 2;
                // texture_ids[3] = texture_index - 1;
            } else if has_roughness_texture && has_normal_texture && has_emissive_texture {
                texture_ids[1] = texture_index - 3;
                texture_ids[2] = texture_index - 2;
                // texture_ids[3] = texture_index - 1;
            } else if has_base_color_texture && has_roughness_texture {
                texture_ids[0] = texture_index - 2;
                texture_ids[1] = texture_index - 1;
            } else if has_base_color_texture && has_normal_texture {
                texture_ids[0] = texture_index - 2;
                texture_ids[2] = texture_index - 1;
            } else if has_base_color_texture && has_emissive_texture {
                texture_ids[0] = texture_index - 2;
                // texture_ids[3] = texture_index - 1;
            } else if has_roughness_texture && has_normal_texture {
                texture_ids[1] = texture_index - 2;
                texture_ids[2] = texture_index - 1;
            } else if has_roughness_texture && has_emissive_texture {
                texture_ids[1] = texture_index - 2;
                // texture_ids[3] = texture_index - 1;
            } else if has_normal_texture && has_emissive_texture {
                texture_ids[2] = texture_index - 2;
                // texture_ids[3] = texture_index - 1;
            } else if has_base_color_texture {
                texture_ids[0] = texture_index - 1;
            } else if has_roughness_texture {
                texture_ids[1] = texture_index - 1;
            } else if has_normal_texture {
                texture_ids[2] = texture_index - 1;
            } else if has_emissive_texture {
                // texture_ids[3] = texture_index - 1;
            }
            // Convert the mesh to a triangle list
            match model.triangles() {
                Ok(triangles) => {
                    for triangle in triangles {
                        // Process each triangle
                        let converted_triangle = Triangle::new(
                            [
                                [triangle[0].position.x, triangle[0].position.y, triangle[0].position.z],
                                [triangle[1].position.x, triangle[1].position.y, triangle[1].position.z],
                                [triangle[2].position.x, triangle[2].position.y, triangle[2].position.z],	
                            ],
                            [triangle[0].normal.x, triangle[0].normal.y, triangle[0].normal.z],
                            material_index,
                            texture_ids.map(|x| x as f32),
                            [
                                [triangle[0].tex_coords.x, triangle[0].tex_coords.y],
                                [triangle[1].tex_coords.x, triangle[1].tex_coords.y],
                                [triangle[2].tex_coords.x, triangle[2].tex_coords.y],
                            ],
                        );
                        converted_triangles.push(converted_triangle);
                        // println!(" TEx_coords: {:?}", converted_triangle.tex_coords);
                    };
                }
                Err(err) => {
                    // Handle the error case
                    println!("Failed to retrieve triangles: {}", err);
                }
            }
            material_index += 1;
        }
        println!(
            "Cameras: #{}  Lights: #{}   Textures: #{} in GLFT scene",
            scene.cameras.len(),
            scene.lights.len(),
            texture_index
        );
    }

    Ok((converted_triangles, converted_materials, textures))
}

pub fn load_hdr(path: String) -> Result<DynamicImage, Box<dyn std::error::Error>> {
    // check fiel extension if hdr or exr
    let binding = path.split('.').collect::<Vec<&str>>();
    let extension = binding.last().unwrap();
    match extension {
        &"hdr" => load_hdri(path),
        &"exr" => load_exr(path),
        _ => panic!("Unsupported file format for background image. Supported formats are: .hdr, .exr"),
    }
}

pub fn load_hdri(path: String) -> Result<DynamicImage, Box<dyn std::error::Error>> {
    let contents = std::fs::read(path)?;
    let mut data = zune_hdr::HdrDecoder::new(contents);
    let pix: Vec<f32> = data.decode()?;
    let dimensions = data.get_dimensions().unwrap();
    println!("first pix:{:?}", (pix[0], pix[1], pix[2]));

    let image = ImageBuffer::<Rgba<u8>, Vec<u8>>::from_fn(dimensions.0 as u32, dimensions.1 as u32, |x, y| {
        let index = (y * dimensions.0 as u32 + x) as usize * 3;
        let r = (pix[index] * 255.0) as u8;
        let g = (pix[index + 1] * 255.0) as u8;
        let b = (pix[index + 2] * 255.0) as u8;
        Rgba([r, g, b, 255])
    });
    let texture: DynamicImage = DynamicImage::ImageRgba8(image);

    Ok(texture)
}

pub fn load_exr(path: String) -> Result<DynamicImage, Box<dyn std::error::Error>> {
    use exr::prelude::*;
    use exr::prelude as exrs;

    // read from the exr file directly into a new `png::RgbaImage` image without intermediate buffers
    let reader = exrs::read()
        .no_deep_data()
        .largest_resolution_level()
        .rgba_channels(
        |resolution, _channels: &RgbaChannels| -> image::RgbaImage {
                image::ImageBuffer::new(
                    resolution.width() as u32,
                    resolution.height() as u32
                )
            },

            // set each pixel in the png buffer from the exr file
            |png_pixels, position, (r,g,b,a): (f32,f32,f32,f32)| { // TODO implicit argument types!
                png_pixels.put_pixel(
                    position.x() as u32, position.y() as u32,
                    image::Rgba([tone_map(r), tone_map(g), tone_map(b), (a * 255.0) as u8])
                );
            }
        )
        .first_valid_layer()
        .all_attributes();

    // an image that contains a single layer containing an png rgba buffer
    let image: Image<Layer<SpecificChannels<image::RgbaImage, RgbaChannels>>> = reader
        .from_file(path)
        .expect("failed to read exr file");


    /// compress any possible f32 into the range of [0,1].
    /// and then convert it to an unsigned byte.
    fn tone_map(linear: f32) -> u8 {
        // TODO does the `image` crate expect gamma corrected data?
        let clamped = (linear - 0.5).tanh() * 0.5 + 0.5;
        (clamped * 255.0) as u8
    }

    let pixel_buffer = image.layer_data.channel_data.pixels;
    // convert the image to a dynamic image
    let image = DynamicImage::ImageRgba8(pixel_buffer);
    Ok(image)
}

fn convert_to_dynamic_image<P, Container>(texture: &image::ImageBuffer<P, Container>) -> DynamicImage
where
    P: Pixel<Subpixel = u8> + 'static,
    Container: Deref<Target = [P::Subpixel]>,
{
    image::DynamicImage::ImageRgba8(
        ImageBuffer::<Rgba<u8>, Vec<u8>>::from_fn(texture.width(), texture.height(), |x, y| {
            let pixel = texture.get_pixel(x, y);
            let (r, g, b, a) = pixel.channels4();
            Rgba([r, g, b, a])
        }),
    )
}