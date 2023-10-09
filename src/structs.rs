use cgmath::prelude::*;
use rand::Rng;
use cgmath::*;
use crate::camera::{Camera, Projection};

//-----------Camera-----------------

#[repr(C)]
#[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct CameraUniform {
    frame:  [f32; 4],
    view_position: [f32; 4],
    view_proj: [[f32; 4]; 4],
    inv_view_proj: [[f32; 4]; 4],
}

impl CameraUniform {
    pub fn new() -> Self {
        Self {
            view_position: [0.0; 4],
            view_proj: cgmath::Matrix4::identity().into(),
            inv_view_proj: cgmath::Matrix4::identity().into(),
            frame: [0.0, 0.0, 0.0, 0.0],
        }
    }

    // UPDATED!
    pub fn update_view_proj(&mut self, camera: &Camera, projection: &Projection) {
        self.view_position = camera.position.to_homogeneous().into();
        self.view_proj = (projection.calc_matrix() * camera.calc_matrix()).into();
        self.inv_view_proj = (projection.calc_matrix() * camera.calc_matrix()).invert().unwrap().into();
    }

    pub fn update_frame(&mut self) {
        self.frame = [self.frame[0] + 1.0, 0.0, 0.0, 0.0];
    }
}


//-----------Material-----------------
#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
pub struct Material {
    albedo: [f32; 4],
    attenuation: [f32; 4],
    roughness: f32,     //0.0 - 1.0 0.0 = mirror, 1.0 = diffuse
    emission: f32,      //0.0 - 1.0 0.0 = no emission, >0.0 = emission
    ior: f32,           //index of refraction
    __padding: f32,
}

impl Material {
    pub fn new(albedo: [f32; 3], attenuation: [f32; 3], roughness: f32, emission: f32, ior: f32) -> Self {
        Self {
            albedo: [albedo[0], albedo[1], albedo[2], 0.0],
            attenuation: [attenuation[0], attenuation[1], attenuation[2], 0.0],
            roughness: roughness,
            emission: emission,
            ior: ior,
            __padding: 0.0,
        }
    }
}

//-----------Textureset-----------------
pub struct TextureSet{
    pub diffuse: wgpu::Texture,
    pub normal: wgpu::Texture,
    pub roughness: wgpu::Texture,
}

//-----------Sphere-----------------
#[derive(Clone, Copy)]
pub struct Sphere {
    pub center: Point3<f32>,
    pub radius: f32,
    pub material_id: i32,
    pub texture_id: i32,
}  

impl Sphere {
    pub fn new(center: Point3<f32>, radius: f32, material_id: i32, texture_id: i32) -> Self{
        Self {center, radius, material_id, texture_id}
    }
}

#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
pub struct SphereUniform {
    center: [f32; 4],
    radius: [f32; 4],
    material_texture_id: [f32; 4], //[material_id, texture_id, 0.0, 0.0]
}

impl SphereUniform {
    pub fn new(sphere: Sphere) -> Self {
        let mut rng = rand::thread_rng();
        Self {
            center: [sphere.center[0], sphere.center[1], sphere.center[2], rng.gen_range(0.0..1.0)],//rand number in last slot
            radius: [sphere.radius, 0.0, 0.0, 0.0],
            material_texture_id: [sphere.material_id as f32, sphere.texture_id as f32, 0.0, 0.0], //[material_id, texture_id, 0.0, 0.0]
        }
    }
}

//-----------Triangle-----------------
#[derive(Clone, Copy)]
pub struct Triangle{
    pub points: [[f32; 3]; 3],
    pub normal: [f32; 3],
    pub material_id: i32,
    pub texture_id: i32,
}

impl Triangle{
    pub fn new(points: [[f32; 3]; 3], normal: [f32; 3], material_id: i32, texture_id: i32) -> Triangle{
        Self{points, normal, material_id, texture_id}
    }
}

#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
pub struct TriangleUniform {
    vertex1: [f32; 4],
    vertex2: [f32; 4],
    vertex3: [f32; 4],
    normal: [f32; 4],
    uv1: [f32; 4],
    uv2: [f32; 4],
    material_texture_id: [f32; 4], //[material_id, texture_id, 0.0, 0.0]
}

impl TriangleUniform {
    pub fn new(triangle: Triangle, uv: Vec<[f32; 2]>, count: i32) -> Self {
        Self {
            vertex1: [triangle.points[0][0], triangle.points[0][1], triangle.points[0][2], 0.0],
            vertex2: [triangle.points[1][0], triangle.points[1][1], triangle.points[1][2], 0.0],
            vertex3: [triangle.points[2][0], triangle.points[2][1], triangle.points[2][2], 0.0],
            normal: [triangle.normal[0],triangle.normal[1],triangle.normal[2], 0.0],
            uv1: [uv[0][0], uv[0][1], uv[1][0], uv[1][1]],
            uv2: [uv[2][0], uv[2][1], count as f32, 0.0],
            material_texture_id: [triangle.material_id as f32, triangle.texture_id as f32, 0.0, 0.0],
        }
    }
}
