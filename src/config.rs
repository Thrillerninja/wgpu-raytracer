use crate::structs::{Background, Material, Sphere};

pub struct Config {
    pub camera_position: (f32, f32, f32),
    pub camera_rotation: [f32; 2],
    pub camera_near_far: [f32; 2],
    pub camera_fov: f32,
    
    pub materials: Vec<Material>,
    pub textures: Vec<[&'static str; 3]>,
    pub triangle_svg_uv_mapping_path: &'static str,
    pub background: Background,
    pub background_path: &'static str,

    pub spheres: Vec<Sphere>,
    pub gltf_path: &'static str,
    pub obj_path: &'static str,

}

impl Config {
    pub fn new() -> Self {
        let mut spheres: Vec<Sphere> = Vec::new();
        //                                            x    y     z   radius    mat_id texture_id
        spheres.push(Sphere::new(cgmath::Point3::new(0.5, 0.0, -1.0), 0.2    , 0,       [-1, -1, -1]));
        spheres.push(Sphere::new(cgmath::Point3::new(-0.5, 3.0, -1.0), 0.5   , 1,       [-1, -1, -1]));
        spheres.push(Sphere::new(cgmath::Point3::new(0.5, 1.0, -1.0), 0.3    , 2,       [-1, -1, -1]));
        spheres.push(Sphere::new(cgmath::Point3::new(0.5, -50.5, -1.0), 50.0 , 3,       [-1, -1, -1]));
        spheres.push(Sphere::new(cgmath::Point3::new(-1.5, 0.0, -1.0), 0.4   , 4,       [-1, -1, -1]));
        spheres.push(Sphere::new(cgmath::Point3::new(-1.5, 0.0, -1.0), 0.2   , 6,       [-1, -1, -1]));
        // for i in 0..100 {
        //     spheres.push(Sphere::new(cgmath::Point3::new(rand::random::<f32>() * 10.0 - 5.0, rand::random::<f32>() * 10.0 - 5.0, rand::random::<f32>() * 10.0 - 5.0), rand::random::<f32>() * 0.5, 1, -1));
        // }
        
        let mut materials: Vec<Material> = Vec::new();
        //                            r     g    b      attenuation      rough emis  ior 
        materials.push(Material::new([1.0, 1.0, 1.0], [0.5, 1.0, 1.0], 0.8, 10.0, 0.0));
        materials.push(Material::new([0.2, 1.5, 0.2], [1.0, 1.0, 1.0], 1.0, 20.0, 0.0));
        materials.push(Material::new([0.0, 0.0, 1.0], [1.0, 1.0, 1.0], 0.0, 0.0, 0.0));
        materials.push(Material::new([1.0, 0.3, 0.2], [0.2, 1.0, 1.0], 0.2, 0.0, 0.0));
        materials.push(Material::new([1.0, 1.0, 1.0], [1.0, 1.0, 1.0], 0.0, 0.0, 1.0));
        materials.push(Material::new([1.0, 1.0, 1.0], [1.0, 1.0, 1.0], 0.0, 30.0, 0.0));
        materials.push(Material::new([1.0, 1.0, 1.0], [1.0, 1.0, 1.0], 0.0, 0.0, -1.0));

        // Load textures from files into a textures
        let mut textures = Vec::new();
        // textures.push(["res/cobble-diffuse.png", "res/cobble-normal.png", "res/cobble-diffuse.png"]);
        //textures.push(["res/COlor.png", "res/Unbenannt2.png", "res/roughness.png"]);
        // textures.push([ "res/PavingStones134_1K-PNG_Color.png", "res/PavingStones134_1K-PNG_Color.png", "res/PavingStones134_1K-PNG_Color.png"]);
        
        let background = Background::new(-1, 0, 1.0);

        Self {
            // Camera
            camera_position: (0.0,2.0,0.0),//(-0.8, 1.59, -2.14),
            camera_rotation: [0.0,-90.0],//[195.0, -20.0],
            camera_near_far: [0.1, 100.0],
            camera_fov: 90.0,

            // Objects
            //obj
            obj_path: r"",
            //gltf
            gltf_path: r"res\untitled.gltf",

            //spheres
            spheres: spheres,

            // Materials & Textures
            materials: materials,
            textures: textures,
            triangle_svg_uv_mapping_path: r"res\Cube.svg",
            background: background,
            background_path: r"res\belfast_farmhouse_4k.exr", //r"res\cobblestone_street_night_4k.hdr",

        }
    }
}
