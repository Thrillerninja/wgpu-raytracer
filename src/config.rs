use crate::structs::{Material, Triangle, Sphere};   
pub struct Config {
    pub camera_position: (f32, f32, f32),
    pub camera_rotation: [f32; 2],
    pub camera_near_far: [f32; 2],
    pub camera_fov: f32,
    pub triangle_obj_path: &'static str,
    pub triangle_svg_uv_mapping_path: &'static str,
    pub spheres: Vec<Sphere>,
    pub materials: Vec<Material>,
    pub textures: Vec<[&'static str; 3]>,

}

impl Config {
    pub fn new() -> Self {
        let mut spheres: Vec<Sphere> = Vec::new();
        //                                            x    y     z   radius    mat_id texture_id
        spheres.push(Sphere::new(cgmath::Point3::new(0.5, 0.0, -1.0), 0.5    , 0,        0));
        spheres.push(Sphere::new(cgmath::Point3::new(-0.5, 0.0, -1.0), 0.5   , 1,       -1));
        spheres.push(Sphere::new(cgmath::Point3::new(0.5, 1.0, -1.0), 0.3    , 2,       -1));
        spheres.push(Sphere::new(cgmath::Point3::new(0.5, -50.5, -1.0), 50.0 , 3,       -1));
        spheres.push(Sphere::new(cgmath::Point3::new(-1.5, 0.0, -1.0), 0.4   , 4,       -1));
        // for i in 0..100 {
        //     spheres.push(Sphere::new(cgmath::Point3::new(rand::random::<f32>() * 10.0 - 5.0, rand::random::<f32>() * 10.0 - 5.0, rand::random::<f32>() * 10.0 - 5.0), rand::random::<f32>() * 0.5, 1, -1));
        // }
        
        let mut materials: Vec<Material> = Vec::new();
        //                            r     g    b      attenuation      rough emis  ior 
        materials.push(Material::new([0.0, 1.0, 0.0], [0.5, 1.0, 1.0], 0.8, 0.0, 0.0));
        materials.push(Material::new([0.5, 0.2, 0.5], [1.0, 1.0, 1.0], 1.0, 0.0, 0.0));
        materials.push(Material::new([0.0, 0.0, 1.0], [1.0, 1.0, 1.0], 0.0, 0.0, 0.0));
        materials.push(Material::new([1.0, 0.3, 0.2], [0.2, 1.0, 1.0], 0.2, 0.0, 0.0));
        materials.push(Material::new([1.0, 1.0, 1.0], [0.5, 1.0, 1.0], 0.0, 0.0, 1.0));

        // Load textures from files into a textures
        let mut textures = Vec::new();
        textures.push(["res/cobble-diffuse.png", "res/cobble-normal.png", "res/cobble-diffuse.png"]);
        textures.push(["res/COlor.png", "res/Unbenannt2.png", "res/roughness.png"]);
        //textures.push([ "res/pavement_26_basecolor-1K.png", "res/pavement_26_normal-1K.png", "res/pavement_26_roughness-1K.png"]);
        
        Self {
            // Camera
            camera_position: (-0.8, 1.29, -2.14),
            camera_rotation: [195.0, -20.0],
            camera_near_far: [0.1, 100.0],
            camera_fov: 45.0,

            // Objects
            //triangulated objects
            triangle_obj_path: r"res\untitled.obj",
            triangle_svg_uv_mapping_path: r"res\Cube.svg",

            //spheres
            spheres: spheres,

            // Materials & Textures
            materials: materials,
            textures: textures,

        }
    }
}
