@group(0) @binding(0)
var color_buffer: texture_storage_2d<rgba8unorm, write>;

// Camera
struct Camera {
    view_pos: vec4<f32>,
    view_proj: mat4x4<f32>,
}
@group(1) @binding(0)
var<uniform> camera: Camera;

struct Sphere {
    center: vec3<f32>,
    radius: f32,
}

struct Ray {
    direction: vec3<f32>,
    origin: vec3<f32>,
}

// Helper function to convert from linear RGB to sRGB
fn linear_to_srgb(color: vec3<f32>) -> vec3<f32> {
    return mix(pow(color, vec3<f32>(1.0 / 2.2)), color * 12.92, step(vec3<f32>(0.0), color));
}

@compute @workgroup_size(1, 1, 1)
fn main(@builtin(global_invocation_id) GlobalInvocationID: vec3<u32>) {
    let screen_size: vec2<i32> = vec2<i32>(textureDimensions(color_buffer));
    let screen_pos: vec2<i32> = vec2<i32>(i32(GlobalInvocationID.x), i32(GlobalInvocationID.y));

    // Convert screen coordinates to normalized device coordinates (NDC)
    let ndc_coords: vec2<f32> = (vec2<f32>(screen_pos) / vec2<f32>(screen_size) * 2.0) - 1.0;

    // Apply camera perspective transformation
    let clip_coords: vec4<f32> = vec4<f32>(ndc_coords.x, ndc_coords.y, 0.0, 1.0);
    let view_proj_matrix: mat4x4<f32> = camera.view_proj;
    let world_coords: vec4<f32> = view_proj_matrix * clip_coords;
    let ray_dir_camera: vec3<f32> = normalize(vec3<f32>(world_coords.x, world_coords.y, world_coords.z));

    var mySphere: Sphere;
    mySphere.center = vec3<f32>(0.0, 0.0, 0.0); // Center of the sphere
    mySphere.radius = 0.5; // Radius of the sphere

    var mySphere2: Sphere;
    mySphere2.center = vec3<f32>(1.0, 0.0, 0.0); // Center of the sphere
    mySphere2.radius = 0.2; // Radius of the sphere

    var mySphere3: Sphere;
    mySphere3.center = vec3<f32>(0.0, 1.0, 0.0); // Center of the sphere
    mySphere3.radius = 0.2; // Radius of the sphere

    var mySphere4: Sphere;
    mySphere4.center = vec3<f32>(0.0, 0.0, 1.0); // Center of the sphere
    mySphere4.radius = 0.2; // Radius of the sphere

    var myRay: Ray;
    myRay.direction = ray_dir_camera; // Ray direction in camera space
    myRay.origin = camera.view_pos.xyz; // Origin of the ray (camera position)

    var mySphereArray: array<Sphere, 4> = array<Sphere, 4>(mySphere, mySphere2, mySphere3, mySphere4);

    var pixel_color: vec3<f32> = vec3<f32>(0.5, 0.0, 0.25);

    for (var i = 0; i < 4; i++) {
        if (hit(myRay, mySphereArray[i])) {
            pixel_color = vec3<f32>(0.5, 1.0, 0.75);
        }
    }

    textureStore(color_buffer, screen_pos, vec4<f32>(pixel_color, 1.0));
}

fn hit(ray: Ray, sphere: Sphere) -> bool {
    let oc: vec3<f32> = ray.origin - sphere.center;
    let a: f32 = dot(ray.direction, ray.direction);
    let b: f32 = dot(oc, ray.direction);
    let c: f32 = dot(oc, oc) - sphere.radius * sphere.radius;
    let discriminant: f32 = b * b - a * c;

    return discriminant > 0.0;
}
