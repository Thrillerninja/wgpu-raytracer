use cgmath::*;
use std::fs::File;
use std::io::{BufRead, BufReader, Read};
use rand::Rng;

#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
pub struct Material {
    albedo: [f32; 4],
    attenuation: [f32; 4],
    roughness: f32,     //0.0 - 1.0 0.0 = mirror, 1.0 = diffuse
    emission: f32,      //0.0 - 1.0 0.0 = no emission, >0.0 = emission
    ior: f32,           //index of refraction
    texture_id:f32,
}

impl Material {
    pub fn new(albedo: [f32; 3], attenuation: [f32; 3], roughness: f32, emission: f32, ior: f32, texture_id: i32) -> Self {
        let mut rng = rand::thread_rng();
        Self {
            albedo: [albedo[0], albedo[1], albedo[2], 0.0],
            attenuation: [attenuation[0], attenuation[1], attenuation[2], 0.0],
            roughness: roughness,
            emission: emission,
            ior: ior,
            texture_id: texture_id as f32,
        }
    }

    pub fn clone(&self) -> Material{
        Material { albedo: self.albedo, attenuation: self.attenuation, roughness: self.roughness, emission: self.emission, ior: self.ior, texture_id: self.texture_id }
    }
}

//all objects in scene
#[derive(Clone, Copy)]
pub struct Sphere {
    pub center: Point3<f32>,
    pub radius: f32,
    pub material: Material
}  

impl Sphere {
    pub fn new(center: Point3<f32>, radius: f32, material: Material) -> Self{
        Self {center, radius, material}
    }
}

#[derive(Clone, Copy)]
pub struct Triangle{
    pub points: [[f32; 3]; 3],
    pub normal: [f32; 3],
    pub material: Material
}

impl Triangle{
    pub fn new(points: [[f32; 3]; 3], normal: [f32; 3], material: Material) -> Triangle{
        Self{points, normal, material}
    }
}

#[derive(Clone)]
pub enum Object{
    Sphere(Sphere),
    Triangle(Triangle)
}

pub fn load_obj(file_path: &str) -> Result<Vec<Triangle>, Box<dyn std::error::Error>> {
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
                let uv1_index = indices[0].1 - 1; // UV index
                let uv2_index = indices[1].1 - 1; // UV index
                let uv3_index = indices[2].1 - 1; // UV index
                let n_index = indices[0].2 - 1;

                let mut rng = rand::thread_rng();
                let r: f32 = rng.gen_range(0.0..1.0);
                let g: f32 = rng.gen_range(0.0..1.0);
                let b: f32 = rng.gen_range(0.0..1.0);
        
                let triangle = Triangle::new(
                    [
                        vertices[v1_index],
                        vertices[v2_index],
                        vertices[v3_index],
                    ],
                    normals[n_index],
                    Material::new(
                        [r, g, b],
                        [0.5, 0.5, 0.5],
                        0.5,
                        0.0,
                        0.0,
                        1,
                    ),
                );
                faces.push(triangle);
            }
        }
    }

    Ok(faces)
}

pub fn load_svg(file_path: &str) -> Result<Vec<Vec<[f32; 2]>>, Box<dyn std::error::Error>> {
    let mut file = File::open(file_path).expect("Failed to open SVG file");
    let mut svg_content = String::new();
    file.read_to_string(&mut svg_content).expect("Failed to read SVG content");

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