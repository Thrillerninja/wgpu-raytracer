use image::{DynamicImage, GenericImageView};
use rtbvh::{Aabb, Builder, Primitive};
use wgpu::SurfaceConfiguration;
use crate::models::{load_svg, load_gltf};
use crate::structs::{self, BvhUniform, Sphere};
use crate::camera;
use crate::structs::{Triangle, Material, TriangleUniform};
use crate::models::load_obj;
use crate::texture::{create_texture, load_textures, load_textures_from_image, scale_texture};
use crate::{load_hdri, load_hdr, load_exr};
use crate::config;


pub fn setup_camera(config: &SurfaceConfiguration, userconfig: &crate::config::Config) -> (camera::Camera, camera::Projection, camera::CameraController, structs::CameraUniform) {
    let camera = camera::Camera::new(userconfig.camera_position, cgmath::Deg(userconfig.camera_rotation[0]), cgmath::Deg(userconfig.camera_rotation[1]));
    let projection =
        camera::Projection::new(config.width, config.height, cgmath::Deg(userconfig.camera_fov), userconfig.camera_near_far[0], userconfig.camera_near_far[1]);
    let camera_controller = camera::CameraController::new(4.0, 1.6);

    let mut camera_uniform = structs::CameraUniform::new();
    camera_uniform.update_view_proj(&camera, &projection);

    return (camera, projection, camera_controller, camera_uniform)
}

pub fn setup_tris_objects(userconfig: &config::Config) -> (Vec<Triangle>, Vec<TriangleUniform>, Vec<Material>, Vec<DynamicImage>){
    // Load SVG UV mapping file
    let tris_uv_mapping = match load_svg(userconfig.model_paths.triangle_svg_uv_mapping_path.clone()) {
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
    // Add materials from config to materials if defined
    match userconfig.materials {
        Some(user_materials) => {
            materials.append(&mut user_materials.clone());
        }
        None => {println!("No materials in config");}
    }
    println!("Config Material count: {}", materials.len());
    

    // --------Triangles-------------
    // Load OBJ file
    match userconfig.model_paths.obj_path {
        Some(obj_path) => {
            if obj_path != "" {
                let (mut obj_triangles,mut obj_materials) = match load_obj(obj_path) {
                    Err(error) => {
                        // Handle the error
                        eprintln!("Error loading OBJ file: {:?}", error);
                        std::process::exit(1);
                    }
                    Ok(data) => data,
                };   
                println!("OBJ Triangle count: {}", obj_triangles.len());
                triangles.append(&mut obj_triangles);
                materials.append(&mut obj_materials);
            }
        }
        None => {println!("No OBJ path in config");}
    }

    // Load GLTF file and add to triangles and materials
    match userconfig.model_paths.gltf_path {
        Some(gltf_path) => {
            if gltf_path != "" {
                let (mut gltf_triangles, mut gltf_materials, mut gltf_textures) = match load_gltf(gltf_path, materials.len() as i32, textures.len() as i32) {
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
        }
        None => {println!("No GLTF path in config");}
    }

    // Triangles and UV to Uniform buffer
    let mut triangles_uniform: Vec<TriangleUniform> = Vec::new();
    let triangles_count = triangles.len() as i32;
    let times = triangles_count / tris_uv_mapping.len() as i32;

    println!("fill Triangle  buffer");
    for i in 0..triangles_count as usize {
        triangles_uniform.push(TriangleUniform::new(triangles[i], tris_uv_mapping[i % tris_uv_mapping.len()].clone(), times));
    }

    return (triangles, triangles_uniform, materials, textures)
}

pub fn setup_textures(userconfig: &config::Config, textures: Vec<DynamicImage>, device: &wgpu::Device, queue: &wgpu::Queue, config: &SurfaceConfiguration) -> wgpu::Texture {
    // Load textures from files into a textureset
    let num_textureslots = if textures.len() as u32 == 0{
        2
    } else {
        textures.len() as u32
    };

    let mut textures_buffer = create_texture(&device, &config, 1024, 1024, num_textureslots);
    let mut texture_count = 0;

    // Add textures from config to textureset
    for i in 0..textures.len(){
        match load_textures(&queue, textures_buffer, &textures[i], i as i32) {
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

    // Add textures from GLTF to textureset
    for i in 0..textures.len(){
        let resized_texture = scale_texture(&textures[i], 1024, 1024);

        match load_textures_from_image(&queue, textures_buffer, &resized_texture, texture_count as i32) {
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

    return textures_buffer;
}

pub fn setup_bvh(triangles: &Vec<Triangle>) ->(Vec<BvhUniform>, Vec<f32>){
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

    //Get the indices of the primitives
    let bvh_prim_indices: Vec<f32> = raw.1.iter().map(|x| *x as f32).collect();

    return (bvh_uniform, bvh_prim_indices);
}


pub fn setup_hdri(userconfig: &config::Config, device: &wgpu::Device, queue: &wgpu::Queue, config: &SurfaceConfiguration) -> wgpu::Texture {
    // Check if a background is configured
    let background_path = match userconfig.background_path {
        Some(background_path) => {
            if background_path == "" {
                return create_texture(&device, &config, 1024, 1024, 1);
            } else {
                background_path
            }
        }
        None => {
            return create_texture(&device, &config, 1024, 1024, 1);
        }
    };

    // Load background image
    let background_img = match load_hdr(background_path){
        Err(error) => {
            // Handle the error
            eprintln!("Error loading HDRI file: {:?}", error);
            std::process::exit(1);
        }
        Ok(data) => data,
    };

    // Create texture from background image
    let mut background_texture = create_texture(&device, &config, background_img.dimensions().0, background_img.dimensions().1, 1);
    background_texture = match load_textures_from_image(&queue, background_texture, &background_img, 0) {
        Err(error) => {
            // Handle the error
            eprintln!("Error loading texture file: {:?}", error);
            std::process::exit(1);
        }
        Ok(data) => data,
    };

    return background_texture;
}