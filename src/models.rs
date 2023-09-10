use cgmath::*;
use std::f32::consts::FRAC_PI_2;
use std::time::Duration;
use winit::dpi::PhysicalPosition;
use winit::event::*;
use std::{time::{Instant}, fs::File, io::BufReader};
use std::io::BufRead;

#[derive(Clone)]
pub struct Material{
    pub albedo: [f32;3],
    pub attenuation: [f32;3],
    pub roughness: f32
}

impl Material{
    pub fn new(albedo: [f32;3], attenuation: [f32;3], roughness: f32)-> Material{
        Self {albedo, attenuation, roughness}
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

#[derive(Clone)]
pub struct Triangle{
    pub points: Vector3<Point3<f32>>,
    pub normal: Vector3<f32>,
    pub material: Material
}

impl Triangle{

    pub fn new(points: Vector3<Point3<f32>>, normal: Vector3<f32>, material: Material) -> Triangle{
        Self{points, normal, material}
    }
}

#[derive(Clone)]
pub enum Object{
    Sphere(Sphere),
    Triangle(Triangle)
}

pub fn load_obj(file_path: &str) -> Result<(Vec<[f32; 3]>, Vec<[f32; 3]>, Vec<[usize; 3]>), Box<dyn std::error::Error>> {
    let file = File::open(file_path)?;
    let reader = BufReader::new(file);

    let mut vertices = Vec::new();
    let mut texture_coords = Vec::new();
    let mut normals = Vec::new();
    let mut faces = Vec::new();

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
            let values: Vec<f32> = line[3..]
                .split_whitespace()
                .map(|x| x.parse::<f32>())
                .collect::<Result<_, _>>()?;

            if values.len() >= 3 {
                let normal = [values[0], values[1], values[2]];
                normals.push(normal);
            }
        } else if line.starts_with("f ") {
            // Parse face indices
            let indices: Vec<usize> = line[2..]
                .split_whitespace()
                .map(|x| x.split('/').next().unwrap_or("0").parse::<usize>().unwrap_or(0))
                .collect();
        
            if indices.len() >= 3 {
                // Ensure we have at least 3 indices and create an array of [usize; 3]
                let face: [usize; 3] = [
                    indices[0],
                    indices[1],
                    indices[2],
                ];
                faces.push(face);
            }
        }
    }
    Ok((vertices, normals, faces))
}
