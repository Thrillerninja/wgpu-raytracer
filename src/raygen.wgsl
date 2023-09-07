@group(0) @binding(0) var color_buffer: texture_storage_2d<rgba8unorm, write>;

//Camera
struct Camera {
    cam_position: vec3<f32>,
    cam_direction: vec3<f32>,
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

// Function to test for ray-sphere intersection
fn hit(ray: Ray, sphere: Sphere) -> bool {
    let oc: vec3<f32> = ray.origin - sphere.center;
    let a: f32 = dot(ray.direction, ray.direction);
    let b: f32 = dot(oc, ray.direction);
    let c: f32 = dot(oc, oc) - sphere.radius * sphere.radius;
    let discriminant: f32 = b * b - a * c;

    return discriminant > 0.0;
}

// Main ray tracing function
@compute @workgroup_size(1, 1, 1)
fn main(@builtin(global_invocation_id) GlobalInvocationID: vec3<u32>) {
    // Get the screen size
    let screen_size: vec2<i32> = vec2<i32>(textureDimensions(color_buffer));
    // Calculate screen position
    let screen_pos: vec2<u32> = vec2<u32>(GlobalInvocationID.xy);

    // Define the camera ray
    let ray_origin: vec3<f32> = camera.cam_position;
    let ray_direction: vec3<f32> = normalize(vec3<f32>(
        f32(screen_pos.x) - 0.5 * f32(screen_size.x),
        f32(screen_pos.y) - 0.5 * f32(screen_size.y),
        -1.0,
    ));

    var ray: Ray;
    ray.origin = ray_origin;
    ray.direction = ray_direction;


    // Define the sphere
    var sphere: Sphere;
    sphere.center = vec3<f32>(0.0, 0.0, 0.0);
    sphere.radius = 1.0;

    // Initialize pixel color
    var pixel_color: vec3<f32> = vec3<f32>(0.5, 0.0, 0.25);

    // Test for ray-sphere intersection
    if hit(ray, sphere) {
        pixel_color = vec3<f32>(0.5, 1.0, 0.75); // Hit the sphere, change color
    }

    // Store the pixel color in the color buffer
    textureStore(color_buffer, vec2<i32>(screen_pos), vec4<f32>(linear_to_srgb(pixel_color), 1.0));
}