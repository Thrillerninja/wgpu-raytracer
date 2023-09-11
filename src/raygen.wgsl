@group(0) @binding(0) var color_buffer: texture_storage_2d<rgba8unorm, write>;

// Camera
struct Camera {
    view_pos: vec3<f32>,
    view_proj: mat4x4<f32>,
    inv_view_proj: mat4x4<f32>,
}
@group(1) @binding(0)
var<uniform> camera: Camera;

// Triangles
struct Triangle {
    vertex1: vec4<f32>,
    vertex2: vec4<f32>,
    vertex3: vec4<f32>,
    normals: vec4<f32>,
    material: Material,
}

struct Material {
    albedo: vec4<f32>,
    attenuation: vec4<f32>,
    roughness: vec4<f32>,
}

@group(2) @binding(0) var<storage> triangles : array<Triangle>;

struct Sphere {
    center: vec3<f32>,
    radius: f32,
}

struct Ray {
    origin: vec3<f32>,
    direction: vec3<f32>,
}

// Helper function to convert from linear RGB to sRGB
fn linear_to_srgb(color: vec3<f32>) -> vec3<f32> {
    return mix(pow(color, vec3<f32>(1.0 / 2.2)), color * 12.92, step(vec3<f32>(0.0), color));
}

// Function to test for ray-sphere intersection
fn hit(ray: Ray, sphere: Sphere) -> f32 {
    let oc: vec3<f32> = ray.origin - sphere.center;
    let a: f32 = dot(ray.direction, ray.direction);
    let b: f32 = 2.0 * dot(oc, ray.direction);
    let c: f32 = dot(oc, oc) - sphere.radius * sphere.radius;
    let discriminant: f32 = b * b - 4.0 * a * c;

    if (discriminant < 0.0) {
        return -1.0;
    } else {
        return (-b - sqrt(discriminant)) / (2.0 * a);
    }
}

fn color(ray: Ray, sphere: Sphere) -> vec3<f32> {
    // Check for ray-sphere intersection
    var t = hit(ray, sphere);

    if (t>0.0) {
        let hit_point: vec3<f32> = ray.origin + ray.direction * t;
        let normal: vec3<f32> = normalize(hit_point - sphere.center);
        return 0.5 * (normal + vec3<f32>(1.0, 1.0, 1.0));
    }

    // Background color (e.g., sky color)
    let unit_direction: vec3<f32> = normalize(ray.direction);
    t = 0.5 * (ray.direction.y + 1.0);
    return mix(vec3<f32>(1.0, 1.0, 1.0), vec3<f32>(0.5, 0.7, 1.0), t);
}


fn cam_to_world(camera: Camera, vector: vec3<f32>) -> vec4<f32> {
    return camera.view_proj * vec4<f32>(vector, 1.0);
}

// Main ray tracing function
@compute @workgroup_size(1, 1, 1)
fn main(@builtin(global_invocation_id) GlobalInvocationID: vec3<u32>) {
    // Get the screen size
    let screen_size: vec2<u32> = vec2<u32>(textureDimensions(color_buffer));
    // Calculate screen position
    let screen_pos: vec2<u32> = vec2<u32>(GlobalInvocationID.xy);

    //----------Camera----------------
    // Replace these with your camera properties
    let vfov: f32 = 90.0; // Vertical field of view in degrees
    let aspect_ratio: f32 = f32(screen_size.x) / f32(screen_size.y);
    let look_from: vec3<f32> = camera.view_pos; // Camera position

    //REdefine Lookat from cameralet 
    let look_at: vec3<f32> = camera.view_pos + normalize(camera.view_proj * vec4<f32>(0.0, 0.0, -1.0, 0.0)).xyz;


    let focus_dist: f32 = 1.0; // Focus distance
    let aperture: f32 = 0.01; // Aperture size

    let theta: f32 = radians(vfov);
    let h: f32 = tan(theta / 2.0);
    let viewport_height: f32 = 2.0 * h;
    let viewport_width: f32 = aspect_ratio * viewport_height;

    let u: f32 = f32(screen_pos.x) / f32(screen_size.x);
    let v: f32 = f32(screen_pos.y) / f32(screen_size.y);

    let w: vec3<f32> = normalize(look_from - look_at);
    let u_axis: vec3<f32> = cross(vec3<f32>(0.0, 1.0, 0.0), w);
    let v_axis: vec3<f32> = -cross(w, u_axis);

    let horizontal: vec3<f32> = viewport_width * u_axis;
    let vertical: vec3<f32> = viewport_height * v_axis;
    let lower_left_corner: vec3<f32> = look_from - 0.5 * horizontal - 0.5 * vertical - w;

    //----------Ray----------------
    let ray_origin: vec3<f32> = look_from;
    let ray_direction: vec3<f32> = normalize(lower_left_corner + u * horizontal + v * vertical - look_from);

    // Create the ray
    let ray: Ray = Ray(ray_origin, ray_direction);

    //----------Obejcts----------------
    var pixel_color = color_tris(ray);

    // // Define 4 spheres in an array
    // let sphere: Sphere = Sphere(vec3<f32>(0.0, 0.0, -1.0), 0.5);

    // // Initialize pixel color as background color (modify as needed)
    // var pixel_color: vec3<f32> = vec3<f32>(0.0, 0.0, 0.0);

    // // Check for ray-sphere intersection
    // pixel_color = color(ray, sphere);

    // Store the pixel color in the color buffer
    textureStore(color_buffer, vec2<i32>(screen_pos), vec4<f32>(pixel_color, 1.0));
}

fn color_tris(ray: Ray) -> vec3<f32> {
    // Check for smallest t in Tris array
    var t = 100000000.0;
    var color = vec3<f32>(0.0, 0.0, 0.0);
    var closest_tris: Triangle;

    for(var i = 0; i < i32(arrayLength(&triangles)); i = i + 1){
        var hit: f32 = hit_tri(triangles[i], ray);
        if(hit > 0.0 && hit < t){
            t = hit;
            closest_tris = triangles[i];
        }
    }

    if (t < 100000000.0) {
        let hit_point: vec3<f32> = ray.origin + ray.direction * t;
        color = normalize(closest_tris.material.albedo.xyz);
    }
    return color;
}

fn basic_coloring(screen_pos: vec2<u32>, screen_size: vec2<u32>) -> vec3<f32> {
    return vec3<f32>(f32(screen_pos.x) / f32(screen_size.x), f32(screen_pos.y) / f32(screen_size.y), 0.2);
}

fn hit_tri(triangle: Triangle, ray: Ray) -> f32 {
    let v0 = triangle.vertex1.xyz;
    let v1 = triangle.vertex2.xyz;
    let v2 = triangle.vertex3.xyz;

    let edge0 = v1 - v0;
    let edge1 = v2 - v0;

    let h = cross(ray.direction, edge1);
    let a = dot(edge0, h);

    if abs(a) < 0.000001 {
        // The ray is parallel or nearly parallel to the triangle's plane
        return -1.0;
    }

    let f = 1.0 / a;
    let s = ray.origin - v0;
    let u = f * dot(s, h);

    if u < 0.0 || u > 1.0 {
        // The intersection point is outside the triangle
        return -1.0;
    }

    let q = cross(s, edge0);
    let v = f * dot(ray.direction, q);

    if v < 0.0 || (u + v) > 1.0 {
        // The intersection point is outside the triangle
        return -1.0;
    }

    let t = f * dot(edge1, q);

    if t > 0.0 {
        // The ray intersects the triangle
        return t;
    }

    // The intersection point is behind the ray's origin
    return -1.0;
}
