@group(0) @binding(0) var color_buffer: texture_storage_2d<rgba8unorm, write>;

// Camera
struct Camera {
    view_pos: vec3<f32>,
    view_proj: mat4x4<f32>,
    inv_view_proj: mat4x4<f32>,
}
@group(1) @binding(0)
var<uniform> camera: Camera;

struct Material {
    albedo: vec4<f32>,
    attenuation: vec4<f32>,
    roughness: vec4<f32>,   //only 1. float used
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
    center: vec3<f32>,
    radius: f32,
    material: Material,
}

struct Ray {
    origin: vec3<f32>,
    direction: vec3<f32>,
}

// Function to test for ray-sphere intersection
fn hit_sphere(ray: Ray, sphere: Sphere) -> f32 {
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

// Init Spheres
// const spheres: array<Sphere> = array<Sphere>(
//     Sphere(vec3<f32>(0.0, 0.0, 1.0), 0.5, Material(vec4<f32>(1.0, 0.5, 0.0, 1.0), vec4<f32>(1.0, 1.0, 1.0, 1.0), vec4<f32>(0.0, 0.0, 0.0, 0.0))),
//     Sphere(vec3<f32>(0.0, 1.0, 0.0), 0.5, Material(vec4<f32>(1.0, 0.5, 0.0, 1.0), vec4<f32>(1.0, 1.0, 1.0, 1.0), vec4<f32>(0.0, 0.0, 0.0, 0.0))),
//     Sphere(vec3<f32>(1.0, 0.0, 0.0), 0.5, Material(vec4<f32>(1.0, 0.5, 0.0, 1.0), vec4<f32>(1.0, 1.0, 1.0, 1.0), vec4<f32>(0.0, 0.0, 0.0, 0.0))),
//     Sphere(vec3<f32>(0.0, 0.0, 0.0), 0.7, Material(vec4<f32>(1.0, 0.5, 0.0, 1.0), vec4<f32>(1.0, 1.0, 1.0, 1.0), vec4<f32>(0.0, 0.0, 0.0, 0.0))),
// );
// fn color_sphere(ray: Ray, sphere: Sphere) -> vec3<f32> {
//     // Check for ray-sphere intersection
//     var t = hit(ray, sphere);

//     if (t>0.0) {
//         let hit_point: vec3<f32> = ray.origin + ray.direction * t;
//         let normal: vec3<f32> = normalize(hit_point - sphere.center);
//         return 0.5 * (normal + vec3<f32>(1.0, 1.0, 1.0));
//     }

//     // Background color (e.g., sky color)
//     let unit_direction: vec3<f32> = normalize(ray.direction);
//     t = 0.5 * (ray.direction.y + 1.0);
//     return mix(vec3<f32>(1.0, 1.0, 1.0), vec3<f32>(0.5, 0.7, 1.0), t);
// }


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

    // Calculate Ray
    var ray = calc_ray(screen_pos, screen_size);

    // Get Color of Objects if hit
    let MAX_BOUNCES: i32 = 5;
    let pixel_color = color(ray, MAX_BOUNCES, 10000.0);

    // //----------Obejcts----------------
    // var pixel_color = color_tris(ray);

    // // Define 4 spheres in an array
    // let sphere: Sphere = Sphere(vec3<f32>(0.0, 0.0, -1.0), 0.5);

    // // Initialize pixel color as background color (modify as needed)
    // var pixel_color: vec3<f32> = vec3<f32>(0.0, 0.0, 0.0);

    // // Check for ray-sphere intersection
    // pixel_color = color(ray, sphere);

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

fn reflect(v: vec3<f32>, n: vec3<f32>) -> vec3<f32> {
    return v - 2.0 * dot(v, n) * n;
}

// fn color_tris(ray: Ray) -> vec3<f32> {
//     // Check for smallest t in Tris array
//     var t = 100000000.0;
//     var color = vec3<f32>(0.1, 0.2, 0.3);
//     var closest_tris: Triangle;

//     for(var i = 0; i < i32(arrayLength(&triangles)); i = i + 1){
//         var hit: f32 = hit_tri(triangles[i], ray);
//         if(hit > 0.0 && hit < t){
//             t = hit;
//             closest_tris = triangles[i];
//         }
//     }

//     if (t < 100000000.0) {
//         let hit_point: vec3<f32> = ray.origin + ray.direction * t;
//         color = closest_tris.material.albedo.xyz*t/500.0;
//     }
//     return color;
// }

// fn basic_coloring(screen_pos: vec2<u32>, screen_size: vec2<u32>) -> vec3<f32> {
//     return vec3<f32>(f32(screen_pos.x) / f32(screen_size.x), f32(screen_pos.y) / f32(screen_size.y), 0.2);
// }

fn hit_tri(ray: Ray, triangle: Triangle) -> f32 {
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

fn sky_color(ray: Ray) -> vec3<f32> {
    let unit_direction: vec3<f32> = normalize(ray.direction);
    let t: f32 = 0.5 * (unit_direction.y + 1.0);
    return mix(vec3<f32>(1.0, 1.0, 1.0), vec3<f32>(0.5, 0.7, 1.0), t);
}

fn color(imported_ray: Ray, MAX_BOUNCES: i32, t_max: f32) -> vec4<f32> {
    let WHITE: vec3<f32> = vec3<f32>(0.5, 0.7, 1.0);
    let BLUE: vec3<f32> = vec3<f32>(1.0, 1.0, 1.0);
    let MAX_COLOR: f32 = 1.0;

    var depth = 0;
    var ray: Ray = imported_ray;
    var attenuation = vec3<f32>(1.0,1.0,1.0);

    var color = vec3<f32>(1.0,1.0,1.0);
    var weight = 1.0;

    while (depth <= 1) {
        var t = t_max;
        // Closest object
        var closest_tris: Triangle;
        var closest_sphere: Sphere;
        var is_sphere: bool = false;

        // Check if a Sphere is hit
        // for(var i = 0; i < 5; i = i + 1){
        //     var hit: f32 = hit_sphere(ray, spheres[i]);
        //     if(hit > 0.0 && hit < t){
        //         t = hit;
        //         closest_sphere = spheres[i];
        //         is_sphere = true;
        //     }
        // }

        //Check if a Triangle is hit
        for(var i = 0; i < i32(arrayLength(&triangles)); i = i + 1){
            var hit: f32 = hit_tri(ray, triangles[i]);
            if(hit > 0.0 && hit < t){
                t = hit;
                closest_tris = triangles[i];
                is_sphere = false;
            }
        }

        // Return background color if no object is hit
        if (t == t_max) {
            color = mix(color, sky_color(ray), weight);
            return vec4<f32>(color, 1.0);
            //return vec4<f32>(abs(closest_tris.normals.xyz), 1.0);   // For normals debugging
        }

        //get color of closest hit object and reflect ray if needed
        // if(is_sphere){
        //     color += closest_sphere.material.albedo.xyz * weight;
        //     attenuation = closest_sphere.material.attenuation.xyz;
        //     ray = Ray(ray.origin + ray.direction * t, reflect(ray.direction, normalize(ray.origin - closest_sphere.center)));
        // } else if (is_sphere == false && t < t_max) {
            color *= closest_tris.material.albedo.xyz;
            ray = Ray(ray.origin + ray.direction * t, rand_vec3_in_unit_sphere(0.5)*rand(0.5)+closest_tris.normals.xyz);
        // }

        weight = 0.5;    
        depth += 1;    
    }
    return vec4<f32>(color, 1.0);
}

fn rand_vec3_in_unit_sphere(r: f32) -> vec3<f32> {
    var squared_magnitude = 2.0;
    var direction: vec3<f32> = vec3<f32>(0.0, 0.0, 0.0);
    var counter = 0.0;
    while (squared_magnitude <= 1.0) {
        //random point as direction of scatter ray
        direction = vec3<f32>(
            rand(r*counter),
            rand(r*6546.0/counter),
            rand(r+5665.0-counter),
        );

        squared_magnitude = direction[0] * direction[0] + direction[1] * direction[1] + direction[2] * direction[2];
        counter += 1.0;
    }
    return normalize(direction)*r;
}

fn rand(v: f32) -> f32{
    return fract(sin(v) * 43758.5453);
}