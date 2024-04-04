use std::fs::File;
use std::io::{BufRead, BufReader, Read};
use image::{DynamicImage, ImageBuffer, Rgba, GenericImageView};
use std::sync::Arc;
// use rand::Rng;
use crate::structs::{Triangle, Material};
use crate::texture;

use cgmath::*;
use core::ops::Deref;
use image::Pixel;

pub fn load_obj(file_path: &str) -> Result<(Vec<Triangle>, Vec<Material>), Box<dyn std::error::Error>> {
    let file = File::open(file_path)?;
    let reader = BufReader::new(file);

    let mut vertices = Vec::new();
    let mut texture_coords = Vec::new();
    let mut normals = Vec::new();
    let mut faces: Vec<Triangle> = Vec::new();

    let mut mat: Vec<Material> = Vec::new();

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
                    0,
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

    Ok((faces,mat))
}

pub fn load_svg(file_path: &str) -> Result<Vec<Vec<[f32; 2]>>, Box<dyn std::error::Error>> {
    let mut file = match File::open(file_path){
        Ok(file) => file,
        Err(e) => panic!("Failed to open SVG: {} | Error: {}", file_path, e),
    };
    let mut svg_content = String::new();
    match file.read_to_string(&mut svg_content){
        Ok(_) => (),
        Err(e) => panic!("Failed to read SVG: {} | Error: {}", file_path, e),
    }

    // Parse the SVG content
    let mut tris = Vec::new();
    let mut height: f32 = 1.0;
    let mut width: f32 = 1.0;

    for line in svg_content.lines() {
        // FIlter for svg size info
        if line.trim().starts_with("<svg ") {
            let width_string = line.split("width=\"").collect::<Vec<&str>>()[1].to_string();
            width = width_string.split("\" ").collect::<Vec<&str>>()[0].to_string().parse::<f32>().unwrap();

            let height_string = line.split("height=\"").collect::<Vec<&str>>()[1].to_string();
            height = height_string.split("\" ").collect::<Vec<&str>>()[0].to_string().parse::<f32>().unwrap();
        // Filter for polygons
        }else if line.trim().starts_with("<polygon") {
            //filter for points
            let mut point_string = line.split("points=\"").collect::<Vec<&str>>()[1].to_string();  //xxxxx points="xxxxx" yyyyy => "xxxxx" yyyyy
            point_string = point_string.split(" \" />").collect::<Vec<&str>>()[0].to_string();      //"xxxxx" yyyyy => "xxxxx"

            //split into single points
            let point_string = point_string.split(" ").collect::<Vec<&str>>();
            let mut points = Vec::new();
            for point in point_string {
                let point = point.split(",").collect::<Vec<&str>>();
                let x = point[0].parse::<f32>().unwrap();
                let y = point[1].parse::<f32>().unwrap();
                points.push([x / width, y / height]);   //scale points to 0.0 - 1.0
            }
            tris.push(points);
        }
    }

    return Ok(tris);
}

pub fn load_gltf(path: &str, material_count: i32, texture_count: i32) -> Result<(Vec<Triangle>, Vec<Material>, Vec<DynamicImage>), Box<dyn std::error::Error>> {
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

            let mut texture_ids = [-1,-1,-1];

            if has_base_color_texture && has_roughness_texture && has_normal_texture {
                texture_ids[0] = texture_index - 3;
                texture_ids[1] = texture_index - 2;
                texture_ids[2] = texture_index - 1;
            } else if has_base_color_texture && has_roughness_texture {
                texture_ids[0] = texture_index - 2;
                texture_ids[1] = texture_index - 1;
            } else if has_base_color_texture && has_normal_texture {
                texture_ids[0] = texture_index - 2;
                texture_ids[2] = texture_index - 1;
            } else if has_roughness_texture && has_normal_texture {
                texture_ids[1] = texture_index - 2;
                texture_ids[2] = texture_index - 1;
            } else if has_base_color_texture {
                texture_ids[0] = texture_index - 1;
            } else if has_roughness_texture {
                texture_ids[1] = texture_index - 1;
            } else if has_normal_texture {
                texture_ids[2] = texture_index - 1;
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

pub fn load_hdri(path: &str) -> Result<DynamicImage, Box<dyn std::error::Error>> {
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
    //stome img
    texture.save("res/cobblestone_street_night_4k.png")?;
    Ok(texture)
}

fn get_pixel<P, Container>(tex_coords: Vector2<f32>, texture: &ImageBuffer<P, Container>) -> P
where
    P: Pixel + 'static,
    P::Subpixel: 'static,
    Container: Deref<Target = [P::Subpixel]>,
{
    let coords = tex_coords.mul_element_wise(Vector2::new(
        texture.width() as f32,
        texture.height() as f32,
    ));

    texture[(
        (coords.x as i64).rem_euclid(texture.width() as i64) as u32,
        (coords.y as i64).rem_euclid(texture.height() as i64) as u32,
    )]
}

fn convert_to_dynamic_image<P, Container>(texture: &ImageBuffer<P, Container>) -> DynamicImage
where
    P: Pixel<Subpixel = u8> + 'static,
    Container: Deref<Target = [P::Subpixel]>,
{
    DynamicImage::ImageRgba8(
        ImageBuffer::<Rgba<u8>, Vec<u8>>::from_fn(texture.width(), texture.height(), |x, y| {
            let pixel = texture.get_pixel(x, y);
            let (r, g, b, a) = pixel.channels4();
            Rgba([r, g, b, a])
        }),
    )
}