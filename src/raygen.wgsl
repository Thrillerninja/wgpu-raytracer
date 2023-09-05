@group(0) @binding(0) var color_buffer: texture_storage_2d<rgba8unorm, write>;

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

    let horizontal_coefficient: f32 = (f32(screen_pos.x) - f32(screen_size.x) / 2.0) / f32(screen_size.x);
    let vertical_coefficient: f32 = (f32(screen_pos.y) - f32(screen_size.y) / 2.0) / f32(screen_size.x);

    var mySphere: Sphere;
    mySphere.center = vec3<f32>(0.0, 0.0, 0.0); // Center of the circle
    mySphere.radius = 0.2; // Radius of the circle

    var myRay: Ray;
    myRay.direction = normalize(vec3<f32>(horizontal_coefficient, vertical_coefficient, 1.0)); // Direction from the pixel to the sphere
    myRay.origin = vec3<f32>(0.0, 0.0, 1.0); // Origin of the ray (adjusted for visibility)

    var pixel_color: vec3<f32> = vec3<f32>(0.5, 0.0, 0.25);

    if (hit(myRay, mySphere)) {
        pixel_color = vec3<f32>(0.5, 1.0, 0.75);
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
