@group(0) @binding(0) var color_buffer: texture_storage_2d<rgba8unorm, write>;

// Camera
struct Camera {
    view_pos: vec3<f32>,
    view_proj: mat4x4<f32>,
    inv_view_proj: mat4x4<f32>,
}
@group(1) @binding(0) var<uniform> camera: Camera;

struct Material {
    albedo: vec4<f32>,
    attenuation: vec4<f32>,
    roughness: vec4<f32>,   //only 1. float used, 2. as rand value
}

// Triangles
struct Triangle {
    vertex1: vec4<f32>,
    vertex2: vec4<f32>,
    vertex3: vec4<f32>,
    normals: vec4<f32>,
    material: Material,
}
@group(2) @binding(0) var<storage> triangles : array<Triangle>;

struct Sphere {
    center: vec4<f32>,
    radius: vec4<f32>,
    material: Material,
}
@group(2) @binding(1) var<storage> spheres : array<Sphere>;

struct Ray {
    origin: vec3<f32>,
    direction: vec3<f32>,
}

// Function to test for ray-sphere intersection
fn hit_sphere(ray: Ray, sphere: Sphere) -> f32 {
    let oc: vec3<f32> = ray.origin - sphere.center.xyz;
    let a: f32 = dot(ray.direction, ray.direction);
    let b: f32 = 2.0 * dot(oc, ray.direction);
    let c: f32 = dot(oc, oc) - sphere.radius.x * sphere.radius.x;
    let discriminant: f32 = b * b - 4.0 * a * c;

    if (discriminant < -0.000001) {         // If Noise in Sphere rendering is visible, increase this value.
        return -1.0;
    } else {
        return (-b - sqrt(discriminant)) / (2.0 * a);
    }
}

// Main ray tracing function
@compute @workgroup_size(1, 1, 1)
fn main(@builtin(global_invocation_id) GlobalInvocationID: vec3<u32>) {
    // Get the screen size
    let screen_size: vec2<u32> = vec2<u32>(textureDimensions(color_buffer));
    // Calculate screen position
    let screen_pos: vec2<u32> = vec2<u32>(GlobalInvocationID.xy);

    // Calculate Ray
    var ray = calc_ray(screen_pos, screen_size);

    // Get Color of Objects if hit
    let MAX_BOUNCES: i32 = 2;
    let pixel_color = color(ray, MAX_BOUNCES, 10000.0);

    // Store the pixel color in the color buffer
    textureStore(color_buffer, vec2<i32>(screen_pos), pixel_color);
}

fn calc_ray(screen_pos: vec2<u32>, screen_size: vec2<u32>) -> Ray {

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
    return Ray(ray_origin, ray_direction);
}

fn hit_tri(ray: Ray, triangle: Triangle) -> f32 {
    let v0 = triangle.vertex1.xyz;
    let v1 = triangle.vertex2.xyz;
    let v2 = triangle.vertex3.xyz;

    let edge1 = v1 - v0;
    let edge2 = v2 - v0;

    let h = cross(ray.direction, edge2);
    let a = dot(edge1, h);

    if abs(a) < 0.000001 {
        return -1.0; // Ray is parallel to the triangle
    }

    let f = 1.0 / a;
    let s = ray.origin - v0;
    let u = f * dot(s, h);

    if u < 0.0 || u > 1.0 {
        return -1.0; // Intersection is outside the triangle's edges
    }

    let q = cross(s, edge1);
    let v = f * dot(ray.direction, q);

    if v < 0.0 || (u + v) > 1.0 {
        return -1.0; // Intersection is outside the triangle's edges
    }

    let t = f * dot(edge2, q);

    if t > 0.00001 { // Adjust this epsilon value based on your scene scale. If Noise in Tris rendering is visible, increase this value.
        return t; // Intersection found
    }

    return -1.0; // Intersection is behind the ray's origin
}

fn sky_color(ray: Ray) -> vec3<f32> {
    let unit_direction: vec3<f32> = normalize(ray.direction);
    let t: f32 = 0.5 * (unit_direction.y + 1.0);
    return mix(vec3<f32>(1.0, 1.0, 1.0), vec3<f32>(0.5, 0.7, 1.0), t);
}

fn color(imported_ray: Ray, MAX_BOUNCES: i32, t_max: f32) -> vec4<f32> {
    let MAX_COLOR: f32 = 1.0;

    var depth = 0;
    var ray: Ray = imported_ray;
    var attenuation = vec3<f32>(1.0, 1.0, 1.0);

    var color = vec3<f32>(1.0, 1.0, 1.0);
    var weight = 1.0;

    while (depth <= MAX_BOUNCES) {
        var t = t_max;
        // Closest object
        var closest_tris: Triangle;
        var closest_sphere: Sphere;
        var is_sphere: bool = false;

        // Check if a Sphere is hit
        for (var i = 0; i < i32(arrayLength(&spheres)); i = i + 1) {
            var hit: f32 = hit_sphere(ray, spheres[i]);
            if (hit > 0.0 && hit < t) {
                t = hit;
                closest_sphere = spheres[i];
                is_sphere = true;
            }
        }

        // Check if a Triangle is hit
        for (var j = 0; j < i32(arrayLength(&triangles)); j = j + 1) {
            var hit: f32 = hit_tri(ray, triangles[j]);
            if (hit > 0.0 && hit < t) {
                t = hit;
                closest_tris = triangles[j];
                is_sphere = false;
            }
        }

        // Return background color if no object is hit
        if (t == t_max) {
            color = mix(color, sky_color(ray), weight);
            return vec4<f32>(color, 1.0);
        }

        // Get color of the closest hit object and reflect ray if needed
        if (is_sphere) {
            color *= closest_sphere.material.albedo.xyz * weight;
            attenuation = closest_sphere.material.attenuation.xyz;
            ray = Ray(ray.origin + ray.direction * t, normalize(ray.origin + ray.direction * t - closest_sphere.center.xyz));
        } else if (!is_sphere && t < t_max) {
            color *= closest_tris.material.albedo.xyz;
            ray = Ray(ray.origin + ray.direction * t, normalize(ray.origin + ray.direction * t + closest_tris.normals.xyz));
        }

        weight *= 0.5; // Update weight based on material attenuation
        depth += 1;
    }
    return vec4<f32>(color, 1.0);
}


fn rand_vec3_in_unit_sphere(roughness: vec2<f32>) -> vec3<f32> {
    var squared_magnitude = 2.0;
    var direction: vec3<f32> = vec3<f32>(0.0, 0.0, 0.0);
    while (squared_magnitude <= 1.0) {
        //random point as direction of scatter ray
        direction = vec3<f32>(
            rand(roughness.y*11.0),
            rand(roughness.y*3.0),
            rand(roughness.y*7.0),
        );

        squared_magnitude = direction[0] * direction[0] + direction[1] * direction[1] + direction[2] * direction[2];
    }
    return normalize(direction)*0.5;
}

fn rand(v: f32) -> f32{
    return fract(sin(v) * 43758.5453);
}