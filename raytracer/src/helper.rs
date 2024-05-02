use image::{DynamicImage, GenericImageView};
use rtbvh::{Aabb, Builder, Primitive};
use wgpu::SurfaceConfiguration;
use scene::{
    camera::{Camera, CameraController, Projection}, config::{Config, Textureset}, models::{load_gltf, load_obj}, structs::{self, BvhUniform, Material, Triangle, TriangleUniform, CameraUniform}};

use scene::texture::{create_texture, load_textures_from_image, scale_texture};
use scene::models::load_hdr;

/// Sets up the camera for the rendering scene.
///
/// This function initializes a camera, a projection, a camera controller, and a camera uniform
/// based on the provided surface configuration and user configuration.
///
/// # Arguments
///
/// * `config` - A reference to the surface configuration which includes the width and height of the surface.
/// * `userconfig` - A reference to the user configuration which includes the camera position, rotation, field of view (fov), and near and far clipping planes.
///
/// # Returns
///
/// * `Camera` - The initialized camera with the position and rotation specified in the user configuration.
/// * `Projection` - The initialized projection with the width, height, fov, and near and far clipping planes specified in the configurations.
/// * `CameraController` - The initialized camera controller with a speed of 4.0 and a sensitivity of 1.6.
/// * `CameraUniform` - The initialized camera uniform which is updated with the view projection of the camera and projection.
///
/// # Example
///
/// ```
/// let surface_result = unsafe {
///     instance.create_surface_unsafe(wgpu::SurfaceTargetUnsafe::from_window(&window).unwrap())
/// };
///
/// let surface = match surface_result {
///     Ok(surface) => surface,
///     Err(error) => {
///         // Handle the error here
///         panic!("Failed to create surface: {:?}", error);
///     }
/// };
/// let surface_caps = surface.get_capabilities(&adapter);
/// let userconfig: Config = Config::defualt();
/// let config: SurfaceConfiguration = wgpu::SurfaceConfiguration {
///         usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
///         format: wgpu::TextureFormat::Rgba8Unorm,
///         width: 800,
///         height: 600,
///         present_mode: surface_caps.present_modes[0],
///         alpha_mode: surface_caps.alpha_modes[0],
///         view_formats: vec![],
///         desired_maximum_frame_latency: 10,
///     };
/// 
/// let (camera, projection, camera_controller, camera_uniform) = setup_camera(&config, &userconfig);
/// ```
pub fn setup_camera(config: &SurfaceConfiguration, userconfig: &Config) -> (Camera, Projection, CameraController, CameraUniform) {
    let camera = Camera::new(userconfig.camera_position, 
                                        cgmath::Deg(userconfig.camera_rotation[0]), 
                                            cgmath::Deg(userconfig.camera_rotation[1]));
    let projection = Projection::new(config.width, 
                                                        config.height, 
                                                        cgmath::Deg(userconfig.camera_fov),
                                                         userconfig.camera_near_far[0], 
                                                         userconfig.camera_near_far[1]);
    let camera_controller = CameraController::new(4.0, 1.6);

    let mut camera_uniform = structs::CameraUniform::new();
    camera_uniform.update_view_proj(&camera, &projection);

    return (camera, projection, camera_controller, camera_uniform)
}

/// Sets up the triangle objects for the rendering scene.
///
/// This function initializes a list of triangles
/// It loads materials, textures and the triangle data from the .obj and .gltf files if specified in the configuration.
/// These get stored in the respective vectors passed as arguments.
///
/// # Arguments
///
/// * `userconfig` - A user configuration which includes the paths to the .obj and .gltf files, the materials and textures to be used.
/// * `materials` - A mutable reference to the vector of materials to which the user-defined materials will be added.
/// * `textures` - A mutable reference to the vector of textures to which the user-defined textures will be added.
///
/// # Returns
///
/// * `Vec<Triangle>` - The list of triangles loaded from the .obj and .gltf files.
/// * `Vec<TriangleUniform>` - The list of triangle uniforms created from the triangles in a GPU friendly format.
/// * `Config` - The original user configuration.
///
/// # Example
///
/// ```
/// let userconfig: Config = Config::default()
/// let (triangles, triangle_uniforms, materials, textures, config) = setup_tris_objects(userconfig);
/// ```
pub fn setup_tris_objects(userconfig: Config, materials: &mut Vec<Material>, textures: &mut Vec<DynamicImage>) -> (Vec<Triangle>, Vec<TriangleUniform>, Config) {
    let gltf_path = userconfig.model_paths.gltf_path.clone();
    let obj_path = userconfig.model_paths.obj_path.clone();
    let obj_material_id = match userconfig.model_paths.obj_material_id {
        Some(obj_material_id) => obj_material_id,
        None => 0,
    };

    let mut triangles: Vec<Triangle> = Vec::new();
    let mut triangles_uniform: Vec<TriangleUniform> = Vec::new();

    let are_paths_empty: bool = obj_path.is_none() && gltf_path.is_none();

    if are_paths_empty {
        // Push Triangle with empty flag to avoid driver crash since the buffer can't be empty
        triangles_uniform.push(TriangleUniform::empty());
        triangles.push(Triangle::empty());
    } else {
        load_obj_file(&mut triangles, materials, obj_path, obj_material_id);
        load_gltf_file(&mut triangles, materials, textures, gltf_path);
        // Convert Triangles in a GPU friendly format (no complex data types because of the C interface limits)
        triangles_uniform = triangles.iter().map(|triangle| TriangleUniform::new(*triangle)).collect();
    }


    (triangles, triangles_uniform, userconfig)
}

/// Adds materials from the user configuration to the materials vector.
///
/// This function checks if there are any user-defined materials in the configuration. If there are, it appends them to the existing materials vector.
/// If there are no user-defined materials, it prints a message indicating that no materials were found in the configuration.
///
/// # Arguments
///
/// * `materials` - A mutable reference to the vector of materials to which the user-defined materials will be added.
/// * `user_materials` - An optional reference to the vector of user-defined materials from the configuration.
///
/// # Example
///
/// ```
/// let materials = Vec::new();
/// let new_materials = Some(vec![Material::default(), Material::default()]);
/// add_materials_from_config(&mut materials, &new_materials);
/// ```
///
/// # Output
///
/// Prints the number of materials in the configuration after the user-defined materials have been added.
/// If there are no materials in the configuration, it prints a message indicating that no materials were found.
pub fn add_materials_from_config(materials: &mut Vec<Material>, user_materials: &Option<Vec<Material>>) {
    if let Some(user_materials) = user_materials {
        materials.append(&mut user_materials.clone());
    } else {
        println!("No materials in config");
    }
    println!("Config Material count: {}", materials.len());
}

/// Adds textures from the user configuration to the textures vector.
///
/// This function checks if there are any user-defined textures in the configuration. If there are, it loads them and appends them to the existing textures vector.
/// If there are no user-defined textures, it prints a message indicating that no textures were found in the configuration.
///
/// # Arguments
///
/// * `textures` - A mutable reference to the vector of textures to which the user-defined textures will be added.
/// * `user_texturesets` - An optional reference to the vector of user-defined textures from the configuration.
///
/// # Example
///
/// ```
/// let textures = Vec::new();
/// let new_textures = Some(vec![Textureset::default()])
/// add_textures_from_config(&mut textures, &new_textures);
/// ```
///
/// # Output
///
/// Prints the number of textures in the configuration after the user-defined textures have been added.
/// If there are no textures in the configuration, it prints a message indicating that no textures were found.
/// If there is an error loading a texture file, it prints an error message and exits the program.
pub fn add_textures_from_config(textures: &mut Vec<DynamicImage>, user_texturesets: &Option<Vec<Textureset>>) {
    if let Some(user_texturesets) = user_texturesets { 
        for user_textureset in user_texturesets {
            //load diffuse, normal and roughness textures
            if let Some(diffuse_path) = &user_textureset.diffuse_path {
                let diffuse_texture = match image::open(diffuse_path) {
                    Err(error) => {
                        eprintln!("Error loading texture file: {:?}", error);
                        std::process::exit(1);
                    }
                    Ok(data) => data,
                };
                textures.push(diffuse_texture);
            }
            if let Some(normal_path) = &user_textureset.normal_path {
                let normal_texture = match image::open(normal_path) {
                    Err(error) => {
                        eprintln!("Error loading texture file: {:?}", error);
                        std::process::exit(1);
                    }
                    Ok(data) => data,
                };
                textures.push(normal_texture);
            }
            if let Some(roughness_path) = &user_textureset.roughness_path {
                let roughness_texture = match image::open(roughness_path) {
                    Err(error) => {
                        eprintln!("Error loading texture file: {:?}", error);
                        std::process::exit(1);
                    }
                    Ok(data) => data,
                };
                textures.push(roughness_texture);
            }
        }
    } else {
        println!("No textures in config");
    }
    println!("Config Texture count: {}", textures.len());
}

/// Loads an OBJ file and appends the triangles and materials to the provided vectors.
///
/// This function takes an optional path to an OBJ file. If the path is `None` or an empty string, it returns early or prints a message indicating that no path was provided.
/// If the path is valid, it attempts to load the OBJ file. If the loading fails, it prints an error message and exits the program.
/// If the loading succeeds, it appends the triangles and materials from the OBJ file to the provided vectors and prints the number of triangles loaded.
///
/// # Arguments
///
/// * `triangles` - A mutable reference to the vector of triangles to which the triangles from the OBJ file will be added.
/// * `materials` - A mutable reference to the vector of materials to which the materials from the OBJ file will be added.
/// * `obj_path` - An optional string representing the path to the OBJ file.
///
/// # Example
///
/// ```
/// let mut materials = Vec<Material>::new();
/// let mut triangeles = Vec<Triangles>::new();
/// load_obj_file(&mut triangles, &mut materials, Some("path/to/obj/file.obj"));
/// ```
///
/// # Output
///
/// Prints the number of triangles loaded from the OBJ file, or a message indicating that no OBJ path was provided.
/// If there is an error loading the OBJ file, it prints an error message and exits the program.
/// If the OBJ path is empty or `None`, it returns early without loading the OBJ file.
fn load_obj_file(triangles: &mut Vec<Triangle>, materials: &mut Vec<Material>, obj_path: Option<String>, obj_material_id: i32) {
    let obj_path: String = match obj_path {
        Some(obj_path) => obj_path,
        None => return,
    };
    if obj_path != "" {
        let (mut obj_triangles, mut obj_materials) = match load_obj(obj_path, obj_material_id) {
            Err(error) => {
                eprintln!("Error loading OBJ file: {:?}", error);
                std::process::exit(1);
            }
            Ok(data) => data,
        };
        println!("OBJ Triangle count: {}", obj_triangles.len());
        triangles.append(&mut obj_triangles);
        materials.append(&mut obj_materials);
    } else {
        println!("No OBJ path in config");
    }
}

/// Loads an GLTF file and appends the triangles, materials, and textures to the provided vectors.
/// 
/// This function takes an optional path to a GLTF file. If the path is `None` or an empty string, it returns early or prints a message indicating that no path was provided.
/// If the path is valid, it attempts to load the GLTF file. If the loading fails, it prints an error message and exits the program.
/// If the loading succeeds, it appends the triangles, materials, and textures from the GLTF file to the provided vectors and prints the number of triangles loaded.
/// 
/// # Arguments
/// 
/// * `triangles` - A mutable reference to the vector of triangles to which the triangles from the GLTF file will be added.
/// * `materials` - A mutable reference to the vector of materials to which the materials from the GLTF file will be added.
/// * `textures` - A mutable reference to the vector of textures to which the textures from the GLTF file will be added.
/// * `gltf_path` - An optional string representing the path to the GLTF file.
/// 
/// # Example
/// 
/// ```
/// let mut materials = Vec<Material>::new();
/// let mut textures = Vec<DynamicImage>::new();
/// let mut triangeles = Vec<Triangles>::new();
/// load_gltf_file(&mut triangles, &mut materials, &mut textures, Some("path/to/gltf/file.gltf"));
/// ```
/// 
/// # Output
/// 
/// Prints the number of triangles loaded from the GLTF file, or a message indicating that no GLTF path was provided.
/// If there is an error loading the GLTF file, it prints an error message and exits the program.
/// If the GLTF path is empty or `None`, it returns early without loading the GLTF file.
fn load_gltf_file(triangles: &mut Vec<Triangle>, materials: &mut Vec<Material>, textures: &mut Vec<DynamicImage>, gltf_path: Option<String>) {
    let gltf_path: String = match gltf_path {
        Some(gltf_path) => gltf_path,
        None => return,
    };
    if gltf_path != "" {
        let (mut gltf_triangles, mut gltf_materials, mut gltf_textures) = match load_gltf(gltf_path, materials.len() as i32, textures.len() as i32) {
            Err(error) => {
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
    } else {
        println!("No GLTF path in config");
    }
}

/// Sets up the textures for the application.
///
/// This function takes a vector of `DynamicImage` objects, a reference to a `wgpu::Device`, a reference to a `wgpu::Queue`, and a reference to a `SurfaceConfiguration`.
/// It creates a texture buffer, then iterates over the vector of `DynamicImage` objects, loading each image into the texture buffer.
/// If an error occurs while loading an image, it prints an error message and exits the program.
/// After all images have been loaded, it prints a message indicating the number of textures that have been loaded and returns the texture buffer.
///
/// # Arguments
///
/// * `textures` - A vector of `DynamicImage` objects representing the textures to be loaded.
/// * `device` - A reference to a `wgpu::Device`.
/// * `queue` - A reference to a `wgpu::Queue`.
/// * `config` - A reference to a `SurfaceConfiguration`.
///
/// # Example
///
/// ```
/// let textures = vec![DynamicImage::new_rgb8(1024, 1024)];
/// let device = wgpu::Device::new();
/// let queue = wgpu::Queue::new();
/// let config = SurfaceConfiguration::new();
/// setup_textures(textures, &device, &queue, &config);
/// ```
///
/// # Output
///
/// Prints the number of textures loaded.
pub fn setup_textures(mut textures: Vec<DynamicImage>, device: &wgpu::Device, queue: &wgpu::Queue, config: &SurfaceConfiguration) -> wgpu::Texture {
    let mut num_textureslots = textures.len() as u32;

    // If there are no Textures added via the config or the 3d model imports,
    // a new empty Texture is created to avoid driver crash caused by empty buffer
    if num_textureslots == 0 {
        textures.push(DynamicImage::new_rgb8(1024, 1024));
        textures.push(DynamicImage::new_rgb8(1024, 1024));
        num_textureslots = 2;
    }


    let mut textures_buffer = create_texture(&device, &config, 1024, 1024, num_textureslots);
    let mut texture_count = 0;
    println!("Textures ready ({})", texture_count);

    // Add textures from config to textureset
    for i in 0..textures.len(){        
        let resized_img = scale_texture(&textures[i], 1024, 1024, i as i32);
        match load_textures_from_image(&queue, textures_buffer, &resized_img, i as i32) {   //TODO: originally load_textures and broke
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
    println!("Textures ready ({})", num_textureslots);

    return textures_buffer;
}

/// Sets up the Bounding Volume Hierarchy (BVH) for the given triangles.
///
/// This function takes a vector of `Triangle` objects and constructs a BVH for them.
/// It first generates Axis-Aligned Bounding Boxes (AABBs) for each triangle and then uses the `Builder` struct to construct the BVH.
/// The BVH construction algorithm used is the Surface Area Heuristic (SAH) with binning.
/// After the BVH is constructed, it is validated and transformed into raw data.
/// The raw data is then converted into a format compatible with a uniform buffer and the indices of the primitives are collected.
///
/// # Arguments
///
/// * `triangles` - A reference to a vector of `Triangle` objects for which the BVH is to be constructed.
///
/// # Returns
///
/// A tuple containing a vector of `BvhUniform` objects representing the BVH in a format compatible with a uniform buffer, and a vector of `f32` representing the indices of the primitives.
///
/// # Example
///
/// ```
/// let triangles = vec![Triangle::new(...)];
/// let (bvh_uniform, bvh_prim_indices) = setup_bvh(&triangles);
/// ```
///
/// # Output
///
/// Prints the progress of the AABB generation, BVH construction, and BVH validation.
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
    let bvh = match builder.construct_binned_sah() {
        Err(error) => {
            // Handle the error
            eprintln!("Error constructing BVH: {:?}", error);
            std::process::exit(1);
        }
        Ok(data) => data
    };

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

/// Sets up the High Dynamic Range Imaging (HDRI) texture for the application.
///
/// This function takes the user configuration, device, queue, and surface configuration as input.
/// It checks if a background is configured in the user configuration.
/// If a background is configured and the path is not empty, it loads the background image from the specified path.
/// If a background is not configured or the path is empty, it creates a default texture.
/// After loading the background image, it creates a texture from the image and loads the texture.
///
/// # Arguments
///
/// * `userconfig` - A reference to the `Config` object containing the user configuration.
/// * `device` - A reference to the `wgpu::Device` object representing the GPU device.
/// * `queue` - A reference to the `wgpu::Queue` object representing the command queue.
/// * `config` - A reference to the `SurfaceConfiguration` object representing the surface configuration.
///
/// # Returns
///
/// A `wgpu::Texture` object representing the HDRI texture.
///
/// # Example
///
/// ```
/// let userconfig = Config::new(...);
/// let device = wgpu::Device::new(...);
/// let queue = wgpu::Queue::new(...);
/// let config = SurfaceConfiguration::new(...);
/// let hdri_texture = setup_hdri(&userconfig, &device, &queue, &config);
/// ```
///
/// # Errors
///
/// This function will terminate the process if there is an error loading the HDRI file or the texture file.
pub fn setup_hdri(userconfig: &Config, device: &wgpu::Device, queue: &wgpu::Queue, config: &SurfaceConfiguration) -> wgpu::Texture {
    // Check if a background is configured
    let background_path = userconfig.background_path.clone();
    
    let background_path = match background_path {
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