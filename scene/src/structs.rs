
use rand::Rng;
use cgmath::{Matrix4, Point3, SquareMatrix};
use rtbvh::{Aabb, Primitive, SpatialTriangle, BvhNode};
use serde::Deserialize;
use glam::Vec3;

use crate::camera::{Camera, Projection};

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
            view_proj: Matrix4::identity().into(),
        }
    }

    pub fn update_view_proj(&mut self, camera: &Camera, projection: &Projection) {
        self.view_position = camera.position.to_homogeneous().into();
        self.view_proj = Matrix4::from(camera.rotation).into();
        self.frame[1] = projection.fovy.0.to_degrees() as f32;
    }

    pub fn update_frame(&mut self) {
        self.frame[0] += 1.0;
    }
}


//-----------Material-----------------
#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable, Debug, Deserialize)]
pub struct Material {
    #[serde(rename = "color")]
    pub albedo: [f32; 4],
    pub attenuation: [f32; 4],
    pub roughness: f32,     //0.0 - 1.0 0.0 = mirror, 1.0 = diffuse
    pub emission: f32,      //0.0 - 1.0 0.0 = no emission, >0.0 = emission
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

    pub fn default() -> Self {
        Self { albedo: [1.0, 1.0, 1.0, 1.0], attenuation: [1.0, 1.0, 1.0, 1.0], roughness: 0.5, emission: 0.0, ior: 0.0, __padding: 0.0 }
    }
}

#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable, Debug, Deserialize)]
pub struct Background {
    pub material_texture_id: [f32; 4], //[material_id, texture_id_diffuse, ,]
    pub intensity: f32,
    pub _padding: [f32; 3],
}

impl Background {
    pub fn new(material_id: i32, texture_id: i32, intensity: f32) -> Self {
        Self {
            material_texture_id: [material_id as f32, texture_id as f32, 0.0, 0.0],
            intensity: intensity,
            _padding: [0.0; 3],
        }
    }
    
    pub fn default() -> Self {
        Self {
            material_texture_id: [-1.0, -1.0, 0.0, 0.0],
            intensity: 1.0,
            _padding: [0.0; 3],
        }
    }
}

//-----------Sphere-----------------

#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable, Deserialize, Debug)]
pub struct Sphere {
    pub center: [f32; 4],
    pub radius: [f32; 4],
    pub material_texture_id: [f32; 4], //[material_id, texture_id_diffuse, texture_id_roughness, texture_id_normal]
}

impl Sphere {
    pub fn new(center: Point3<f32>, radius: f32, material_id: i32, texture_ids: [i32; 3]) -> Self {
        let mut rng = rand::thread_rng();
        Self {
            center: [center[0], center[1], center[2], rng.gen_range(0.0..1.0)],//rand number in last slot
            radius: [radius, 0.0, 0.0, 0.0],
            material_texture_id: [material_id as f32, texture_ids[0] as f32, texture_ids[1] as f32, texture_ids[2] as f32], //material_id, texture_id_diffuse, texture_id_roughness, texture_id_normal
        }
    }

    pub fn empty() -> Self {
        Self {
            center: [0.0; 4],
            radius: [0.0; 4],
            material_texture_id: [0.0; 4],
        }
    }
}

impl Primitive for Sphere {
    fn center(&self) -> glam::Vec3 {
        glam::Vec3::new(self.center[0], self.center[1], self.center[2])
    }

    fn aabb(&self) -> Aabb {
        let mut aabb = Aabb::new();
        aabb.grow(Vec3::new(self.center[0] - self.radius[0], self.center[1] - self.radius[0], self.center[2] - self.radius[0]));
        aabb.grow((self.center[0] + self.radius[0], self.center[1] + self.radius[0], self.center[2] + self.radius[0]).into());
        aabb
    }
}

//-----------Triangle-----------------
#[derive(Clone, Copy, Debug)]
pub struct Triangle{
    pub points: [[f32; 3]; 3],
    pub normal: [f32; 3],
    pub material_id: i32,
    pub texture_ids: [f32; 3],
    pub tex_coords: [[f32; 2]; 3],
}

impl Triangle{
    pub fn new(points: [[f32; 3]; 3], normal: [f32; 3], material_id: i32, texture_ids: [f32; 3], tex_coords: [[f32;2];3]) -> Triangle{
        Self{points, normal, material_id, texture_ids, tex_coords}
    }
    pub fn empty() -> Triangle{
        Self{points: [[0.0; 3]; 3], normal: [0.0; 3], material_id: 0, texture_ids: [0.0; 3], tex_coords: [[0.0; 2]; 3]}
    }
}

#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable, Debug)]
pub struct TriangleUniform {
    vertex1: [f32; 4],
    vertex2: [f32; 4],
    vertex3: [f32; 4],
    normal: [f32; 4],
    texcords1: [f32; 4],
    texcords2: [f32; 4],    // tex3x, tex3y, 0.0, 0.0
    material_texture_id: [f32; 4], //[material_id, texture_id_diffuse, texture_id_roughness, texture_id_normal]
}

impl TriangleUniform {
    pub fn new(triangle: Triangle) -> Self {
        Self {
            vertex1: [triangle.points[0][0], triangle.points[0][1], triangle.points[0][2], 0.0],
            vertex2: [triangle.points[1][0], triangle.points[1][1], triangle.points[1][2], 0.0],
            vertex3: [triangle.points[2][0], triangle.points[2][1], triangle.points[2][2], 0.0],
            normal: [triangle.normal[0],triangle.normal[1],triangle.normal[2], 0.0],
            material_texture_id: [triangle.material_id as f32, triangle.texture_ids[0] as f32, triangle.texture_ids[1] as f32, triangle.texture_ids[2] as f32],
            texcords1: [triangle.tex_coords[0][0], triangle.tex_coords[0][1], triangle.tex_coords[1][0], triangle.tex_coords[1][1]],
            texcords2: [triangle.tex_coords[2][0], triangle.tex_coords[2][1], 0.0, 0.0],
        }
    }
    pub fn empty() -> Self {
        Self {
            vertex1: [1.0; 4],
            vertex2: [2.0; 4],
            vertex3: [3.0; 4],
            normal: [0.0; 4],
            material_texture_id: [0.0; 4],
            texcords1: [0.0; 4],
            texcords2: [0.0; 4],
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

//-----------Shader Config-----------------
#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
pub struct ShaderConfig {
    //raytracing shader
    pub ray_max_bounces: i32,
    pub ray_samples_per_pixel: i32,
    pub ray_max_ray_distance: f32,

    //camera
    pub ray_focus_distance: f32,
    pub ray_aperture: f32,
    pub ray_lens_radius: f32,

    pub ray_debug_rand_color: i32, //used as bool
    pub ray_focus_viewer_visible: i32, //used as bool
    pub ray_debug_bvh_bounding_box: i32, //used as bool
    pub ray_debug_bvh_bounding_color: i32, //used as bool



    //denoising shader
    pub first_pass: i32,
    pub second_pass: i32,

    //temporal basic
    pub temporal_basic_low_threshold: f32,
    pub temporal_basic_high_threshold: f32,
    pub temporal_basic_low_blend_factor: f32,
    pub temporal_basic_high_blend_factor: f32,

    //temporal adaptive
    pub temporal_adaptive_motion_threshold: f32,
    pub temporal_adaptive_direction_threshold: f32,
    pub temporal_adaptive_low_threshold: f32,
    pub temporal_adaptive_high_threshold: f32,
    pub temporal_adaptive_low_blend_factor: f32,
    pub temporal_adaptive_high_blend_factor: f32,

    //spatial basic
    pub spatial_kernel_size: i32,
    //spatial bilateral
    pub spatial_bilat_space_sigma: f32,
    pub spatial_bilat_color_sigma: f32,
    pub spatial_bilat_radius: i32,
    //spatial non local means
    pub spatial_den_cormpare_radius: i32,
    pub spatial_den_patch_radius: i32,
    pub spatial_den_significant_weight: f32,  
}

impl Default for ShaderConfig {
    fn default() -> Self {
        Self {
            ray_max_bounces: 10,
            ray_samples_per_pixel: 1,
            ray_max_ray_distance: 10_000.0,
            ray_focus_distance: 2.5,
            ray_aperture: 0.005,
            ray_lens_radius: 0.0,
            ray_debug_rand_color: 0,
            ray_focus_viewer_visible: 0,
            ray_debug_bvh_bounding_box: 0,
            ray_debug_bvh_bounding_color: 0,

            first_pass: 4,
            second_pass: 2,

            temporal_basic_low_threshold: 0.05,
            temporal_basic_high_threshold: 0.2,
            temporal_basic_low_blend_factor: 0.03,
            temporal_basic_high_blend_factor: 0.2,

            temporal_adaptive_motion_threshold: 0.005,
            temporal_adaptive_direction_threshold: 0.01,
            temporal_adaptive_low_threshold: 0.05,
            temporal_adaptive_high_threshold: 0.2,
            temporal_adaptive_low_blend_factor: 0.03,
            temporal_adaptive_high_blend_factor: 0.2,

            spatial_kernel_size: 3,

            spatial_bilat_space_sigma: 100.0,
            spatial_bilat_color_sigma: 20.0,
            spatial_bilat_radius: 3,


            spatial_den_cormpare_radius: 13,
            spatial_den_patch_radius: 5,
            spatial_den_significant_weight: 0.001
        }
    }
}

impl ShaderConfig {
    pub fn default_denoise(shaderconfig: ShaderConfig) -> Self {
        Self {
            first_pass: 4,
            second_pass: 2,

            temporal_basic_low_threshold: 0.05,
            temporal_basic_high_threshold: 0.2,
            temporal_basic_low_blend_factor: 0.03,
            temporal_basic_high_blend_factor: 0.2,

            temporal_adaptive_motion_threshold: 0.005,
            temporal_adaptive_direction_threshold: 0.01,
            temporal_adaptive_low_threshold: 0.05,
            temporal_adaptive_high_threshold: 0.2,
            temporal_adaptive_low_blend_factor: 0.03,
            temporal_adaptive_high_blend_factor: 0.2,

            spatial_kernel_size: 3,

            spatial_bilat_space_sigma: 100.0,
            spatial_bilat_color_sigma: 20.0,
            spatial_bilat_radius: 3,

            spatial_den_cormpare_radius: 13,
            spatial_den_patch_radius: 5,
            spatial_den_significant_weight: 0.001,
            ..shaderconfig
        }
    }

    pub fn default_raytrace(shaderconfig: ShaderConfig) -> Self {
        Self {
            ray_max_bounces: 10,
            ray_samples_per_pixel: 1,
            ray_max_ray_distance: 10_000.0,
            ray_focus_distance: 2.5,
            ray_aperture: 0.005,
            ray_lens_radius: 0.0,
            ray_debug_rand_color: 0,
            ray_focus_viewer_visible: 0,
            ray_debug_bvh_bounding_box: 0,
            ray_debug_bvh_bounding_color: 0,
            ..shaderconfig
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_camera_uniform() {
        let camera = CameraUniform::new();
        assert_eq!(camera.frame, [0.0; 4]);
        assert_eq!(camera.view_position, [0.0; 4]);        
    }

    // #[test]
    // fn update_view_proj() {
    //     let mut camera = CameraUniform::new();
    //     let camera = Camera::new(Point3::new(0.0, 0.0, 0.0), Rad(0.0), Rad(0.0));
    //     let projection = Projection::new(100, 100, Rad(1.0), 0.1, 100.0);
    //     camera.update_view_proj(&camera, &projection);
    //     assert_eq!(camera.position, [0.0, 0.0, 0.0, 1.0]);
    //     assert_eq!(camera.view_proj, Matrix4::from(camera.rotation) * Matrix4::from(camera.position));
    // }

    #[test]
    fn update_frame() {
        let mut camera = CameraUniform::new();
        camera.update_frame();
        assert_eq!(camera.frame, [1.0, 0.0, 0.0, 0.0]);
    }

    #[test]
    fn test_material() {
        let material = Material::new([1.0, 1.0, 1.0], [1.0, 1.0, 1.0], 0.5, 0.0, 0.0);
        assert_eq!(material.albedo, [1.0, 1.0, 1.0, 0.0]);
        assert_eq!(material.attenuation, [1.0, 1.0, 1.0, 0.0]);
        assert_eq!(material.roughness, 0.5);
        assert_eq!(material.emission, 0.0);
        assert_eq!(material.ior, 0.0);
    }

    #[test]
    fn test_background() {
        let background = Background::new(1, 1, 1.0);
        assert_eq!(background.material_texture_id, [1.0, 1.0, 0.0, 0.0]);
        assert_eq!(background.intensity, 1.0);
    }

    #[test]
    fn test_sphere() {
        let sphere = Sphere::new(Point3::new(0.0, 0.0, 0.0), 1.0, 1, [1, 1, 1]);
        assert_eq!(sphere.center[0..3], [0.0, 0.0, 0.0]);
        assert_eq!(sphere.radius, [1.0, 0.0, 0.0, 0.0]);
        assert_eq!(sphere.material_texture_id, [1.0, 1.0, 1.0, 1.0]);
    }

    #[test]
    fn test_sphere_center() {
        let sphere = Sphere::new(Point3::new(0.0, 0.0, 0.0), 1.0, 1, [1, 1, 1]);
        assert_eq!(sphere.center(), glam::Vec3::new(0.0, 0.0, 0.0));
    }

    #[test]
    fn test_sphere_aabb() {
        let sphere = Sphere::new(Point3::new(0.0, 0.0, 0.0), 1.0, 1, [1, 1, 1]);
        let aabb = sphere.aabb();
        assert_eq!(aabb.min, Vec3::new(-1.0, -1.0, -1.0));
        assert_eq!(aabb.max, Vec3::new(1.0, 1.0, 1.0));
    }

    #[test]
    fn test_triangle() {
        let triangle = Triangle::new([[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0]], [0.0, 0.0, 1.0], 1, [1.0, 1.0, 1.0], [[0.0, 0.0], [1.0, 0.0], [0.0, 1.0]]);
        assert_eq!(triangle.points, [[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0]]);
        assert_eq!(triangle.normal, [0.0, 0.0, 1.0]);
        assert_eq!(triangle.material_id, 1);
        assert_eq!(triangle.texture_ids, [1.0, 1.0, 1.0]);
        assert_eq!(triangle.tex_coords, [[0.0, 0.0], [1.0, 0.0], [0.0, 1.0]]);
    }

    #[test]
    fn test_triangle_center() {
        let triangle = Triangle::new([[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0]], [0.0, 0.0, 1.0], 1, [1.0, 1.0, 1.0], [[0.0, 0.0], [1.0, 0.0], [0.0, 1.0]]);
        assert_eq!(triangle.center(), glam::Vec3::new(0.33333334, 0.33333334, 0.0));
    }

    #[test]
    fn test_triangle_aabb() {
        let triangle = Triangle::new([[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0]], [0.0, 0.0, 1.0], 1, [1.0, 1.0, 1.0], [[0.0, 0.0], [1.0, 0.0], [0.0, 1.0]]);
        let aabb = triangle.aabb();
        assert_eq!(aabb.min, Vec3::new(0.0, 0.0, 0.0));
        assert_eq!(aabb.max, Vec3::new(1.0, 1.0, 0.0));
    }

    #[test]
    fn test_triangle_uniform() {
        let triangle = Triangle::new([[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0]], [0.0, 0.0, 1.0], 1, [1.0, 1.0, 1.0], [[0.0, 0.0], [1.0, 0.0], [0.0, 1.0]]);
        let triangle_uniform = TriangleUniform::new(triangle);
        assert_eq!(triangle_uniform.vertex1, [0.0, 0.0, 0.0, 0.0]);
        assert_eq!(triangle_uniform.vertex2, [1.0, 0.0, 0.0, 0.0]);
        assert_eq!(triangle_uniform.vertex3, [0.0, 1.0, 0.0, 0.0]);
        assert_eq!(triangle_uniform.normal, [0.0, 0.0, 1.0, 0.0]);
        assert_eq!(triangle_uniform.material_texture_id, [1.0, 1.0, 1.0, 1.0]);
        assert_eq!(triangle_uniform.texcords1, [0.0, 0.0, 1.0, 0.0]);
        assert_eq!(triangle_uniform.texcords2, [0.0, 1.0, 0.0, 0.0]);
    }

    #[test]
    fn test_bvh_uniform() {
        let bvh = BvhNode::new();
        let bvh_uniform = BvhUniform::new(&bvh);
        assert_eq!(bvh_uniform.bounds_min, [1e34, 1e34, 1e34, 0.0]);
        assert_eq!(bvh_uniform.bounds_max, [-1e34, -1e34, -1e34, 0.0]);
        assert_eq!(bvh_uniform.bounds_extra1, [0.0, 0.0, 0.0, 0.0]);
        assert_eq!(bvh_uniform.bounds_extra2, [0.0, 0.0, 0.0, 0.0]);
    }
}