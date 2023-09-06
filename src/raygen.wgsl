// Constants
const BACKGROUND_COLOR: vec3<f32> = vec3<f32>(0.5, 0.0, 0.25);
const HIT_COLOR: vec3<f32> = vec3<f32>(0.5, 1.0, 0.75);

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

@group(2) @binding(0)
var<uniform> spheres: array<Sphere,4>;


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

    // Initialize the model matrix as an identity matrix
    var model_matrix = mat4x4<f32>(    
    vec4<f32>(1.0, 0.0, 0.0, 0.0),
    vec4<f32>(0.0, 1.0, 0.0, 0.0),
    vec4<f32>(0.0, 0.0, 1.0, 0.0),
    vec4<f32>(0.0, 0.0, 0.0, 1.0),
    );

    // Apply camera view-projection transformation
    for (var i ; i < 4; i++) {
        let sphere_center = spheres[i].center;
        let translation_matrix = mat4x4<f32>(    // Create a translation matrix for the current sphere
            vec4<f32>(1.0, 0.0, 0.0, 0.0),
            vec4<f32>(0.0, 1.0, 0.0, 0.0),
            vec4<f32>(0.0, 0.0, 1.0, 0.0),
            vec4<f32>(sphere_center.x, sphere_center.y, sphere_center.z, 1.0),
        );

        // Combine the translation matrix with the model matrix
        model_matrix = translation_matrix * model_matrix;
    }
    //Calc clip position
    let clip_position: vec4<f32> = camera.view_proj * model_matrix * vec4<f32>(sphere.center, 1.0);



    let view_proj_matrix: mat4x4<f32> = camera.view_proj;
    let world_coords: vec4<f32> = view_proj_matrix * clip_coords;
    let ray_dir_camera: vec3<f32> = normalize(vec3<f32>(world_coords.x, world_coords.y, world_coords.z));

    var myRay: Ray;
    myRay.direction = ray_dir_camera; // Ray direction in camera space
    myRay.origin = camera.view_pos.xyz; // Origin of the ray (camera position)

    var pixel_color: vec3<f32> = BACKGROUND_COLOR;

    for (var i = 0; i < 4; i++) {
        // Check if the sphere is inside the view frustum
        if (isSphereInFrustum(spheres[i], camera.view_proj) &&
            hit(myRay, spheres[i])) {
            pixel_color = HIT_COLOR;
            break; // Early exit if a hit is found
        }
    }

    textureStore(color_buffer, screen_pos, vec4<f32>(pixel_color, 1.0));
}

fn isSphereInFrustum(sphere: Sphere, view_proj_matrix: mat4x4<f32>) -> bool {
    // The view_proj_matrix should be the combined view and projection matrix of the camera
    // Calculate the center of the sphere in view space
    let sphere_center_view: vec4<f32> = view_proj_matrix * vec4<f32>(sphere.center, 1.0);
    
    // Calculate the distance from the sphere center to the camera in view space
    let distance_to_camera_view: f32 = length(sphere_center_view.xyz);
    
    // If the distance is less than the sphere's radius, it is inside the frustum
    return distance_to_camera_view + sphere.radius > 0.0;
}

fn hit(ray: Ray, sphere: Sphere) -> bool {
    let oc: vec3<f32> = ray.origin - sphere.center;
    let a: f32 = dot(ray.direction, ray.direction);
    let b: f32 = dot(oc, ray.direction);
    let c: f32 = dot(oc, oc) - sphere.radius * sphere.radius;
    let discriminant: f32 = b * b - a * c;

    return discriminant > 0.0;
}
