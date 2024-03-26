use cgmath::prelude::*;
use rand::Rng;
use cgmath::*;
use crate::camera::{Camera, Projection};
use rtbvh::*;
use glam::*;

//-----------Camera-----------------
#[repr(C)]
#[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct CameraUniform {
    frame: [f32; 4],
    view_position: [f32; 4],
    view_proj: [[f32; 4]; 4],
}

impl CameraUniform {
    pub fn new() -> Self {
        Self {
            frame: [0.0; 4],
            view_position: [0.0; 4],
            view_proj: cgmath::Matrix4::identity().into(),
        }
    }

    // UPDATED!
    pub fn update_view_proj(&mut self, camera: &Camera, projection: &Projection) {
        self.view_position = camera.position.to_homogeneous().into();
        self.view_proj = cgmath::Matrix4::from(camera.rotation).into();
        self.frame[1] = projection.fovy.0.to_degrees() as f32;
    }

    pub fn update_frame(&mut self) {
        self.frame[0] += 1.0;
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
#[derive(Clone, Copy, Debug)]
pub struct Sphere {
    pub center: Point3<f32>,
    pub radius: f32,
    pub material_id: i32,
    pub texture_ids: [i32; 3]
}  

impl Sphere {
    pub fn new(center: Point3<f32>, radius: f32, material_id: i32, texture_ids: [i32; 3]) -> Self{
        Self {center, radius, material_id, texture_ids}
    }
}

#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
pub struct SphereUniform {
    center: [f32; 4],
    radius: [f32; 4],
    material_texture_id: [f32; 4], //[material_id, texture_id_diffuse, texture_id_roughness, texture_id_normal]
}

impl SphereUniform {
    pub fn new(sphere: Sphere) -> Self {
        let mut rng = rand::thread_rng();
        Self {
            center: [sphere.center[0], sphere.center[1], sphere.center[2], rng.gen_range(0.0..1.0)],//rand number in last slot
            radius: [sphere.radius, 0.0, 0.0, 0.0],
            material_texture_id: [sphere.material_id as f32, sphere.texture_ids[0] as f32, sphere.texture_ids[1] as f32, sphere.texture_ids[2] as f32], //material_id, texture_id_diffuse, texture_id_roughness, texture_id_normal
        }
    }
}

impl Primitive for Sphere {
    fn center(&self) -> glam::Vec3 {
        glam::Vec3::new(self.center[0], self.center[1], self.center[2])
    }

    fn aabb(&self) -> Aabb {
        let mut aabb = Aabb::new();
        aabb.grow(Vec3::new(self.center[0] - self.radius, self.center[1] - self.radius, self.center[2] - self.radius));
        aabb.grow((self.center[0] + self.radius, self.center[1] + self.radius, self.center[2] + self.radius).into());
        aabb
    }
}

impl SpatialTriangle for Sphere {
    fn vertex0(&self) -> Vec3 {
        (self.center[0] - self.radius, self.center[1], self.center[2]).into()
    }

    fn vertex1(&self) -> Vec3 {
        (self.center[0], self.center[1] + self.radius, self.center[2]).into()
    }

    fn vertex2(&self) -> Vec3 {
        (self.center[0], self.center[1], self.center[2] + self.radius).into()
    }
}
//-----------Triangle-----------------
#[derive(Clone, Copy, Debug)]
pub struct Triangle{
    pub points: [[f32; 3]; 3],
    pub normal: [f32; 3],
    pub material_id: i32,
    pub texture_ids: [f32; 3],
}

impl Triangle{
    pub fn new(points: [[f32; 3]; 3], normal: [f32; 3], material_id: i32, texture_ids: [f32; 3]) -> Triangle{
        Self{points, normal, material_id, texture_ids}
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
    material_texture_id: [f32; 4], //[material_id, texture_id_diffuse, texture_id_roughness, texture_id_normal]
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
            material_texture_id: [triangle.material_id as f32, triangle.texture_ids[0] as f32, triangle.texture_ids[1] as f32, triangle.texture_ids[2] as f32],
        }
    }
}

impl Primitive for Triangle {
    fn center(&self) -> glam::Vec3 {
        glam::Vec3::new(self.points[0][0] + self.points[1][0] + self.points[2][0],
                        self.points[0][1] + self.points[1][1] + self.points[2][1],
                        self.points[0][2] + self.points[1][2] + self.points[2][2]) / 3.0
    }

    fn aabb(&self) -> Aabb {
        let mut aabb = Aabb::new();
        aabb.grow(self.points[0].into());
        aabb.grow(self.points[1].into());
        aabb.grow(self.points[2].into());
        aabb
    }
}

impl SpatialTriangle for Triangle {
    fn vertex0(&self) -> Vec3 {
        self.points[0].into()
    }

    fn vertex1(&self) -> Vec3 {
        self.points[1].into()
    }

    fn vertex2(&self) -> Vec3 {
        self.points[2].into()
    }
}


#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
pub struct BvhUniform {
    bounds_min: [f32; 4],
    bounds_max: [f32; 4],
    bounds_extra1: [f32; 4],
    bounds_extra2: [f32; 4],
}

impl BvhUniform {
    pub fn new(bvh: &BvhNode) -> Self {
        Self {
            bounds_min: [bvh.bounds.min.x, bvh.bounds.min.y, bvh.bounds.min.z, 0.0],
            bounds_max: [bvh.bounds.max.x, bvh.bounds.max.y, bvh.bounds.max.z, 0.0],
            bounds_extra1: [bvh.bounds.extra1 as f32, 0.0, 0.0, 0.0],
            bounds_extra2: [bvh.bounds.extra2 as f32, 0.0, 0.0, 0.0],
        }
    }
}