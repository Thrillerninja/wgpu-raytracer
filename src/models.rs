use cgmath::*;
use std::f32::consts::FRAC_PI_2;
use std::time::Duration;
use winit::dpi::PhysicalPosition;
use winit::event::*;
use std::{time::{Instant}, fs::File, io::BufReader};
use std::io::BufRead;
use rand::Rng;

#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
pub struct Material {
    albedo: [f32; 4],
    attenuation: [f32; 4],
    roughness: [f32; 4],
}

impl Material {
    pub fn new(albedo: [f32; 3], attenuation: [f32; 3], roughness: f32) -> Self {
        Self {
            albedo: [albedo[0], albedo[1], albedo[2], 0.0],
            attenuation: [attenuation[0], attenuation[1], attenuation[2], 0.0],
            roughness: [roughness, 0.0, 0.0, 0.0],
        }
    }

    pub fn clone(&self) -> Material{
        Material { albedo: self.albedo, attenuation: self.attenuation, roughness: self.roughness}
    }
}

//all objects in scene
#[derive(Clone)]
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

#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
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

// Uniform for transferin the tris to the gpu
#[repr(C)]
#[derive(Copy, Clone)]
pub struct TriangleUniform {
    pub points: [Vector3<f32>; 3],
    pub normal: Vector3<f32>,
    pub material: Material
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

            println!("{} {} {}", val[0], val[1], val[2]);

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
                let v1_index = indices[0].0 - 1;    //inices of vertecies to be added
                let v2_index = indices[1].0 - 1;
                let v3_index = indices[2].0 - 1;
                let n_index = indices[0].2 - 1;     //index of normal to be added calced with rand. vertex value as all have equal normals

                // Generate random RGB values for the material color
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
                        [r, g, b], // Use the random values for the color
                        [1.0, 1.0, 1.0],
                        1.0,
                    ),
                );
                faces.push(triangle);
            }
        }
    }

    Ok(faces)
}