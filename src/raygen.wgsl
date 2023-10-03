@group(0) @binding(0) var color_buffer: texture_storage_2d<rgba8unorm, write>;
@group(3) @binding(0) var temporal_color_buffer: sampler;
// Camera
struct Camera {
    current_frame_counter: f32,
    view_pos: vec4<f32>,
    view_proj: mat4x4<f32>,
    inv_view_proj: mat4x4<f32>,
}
@group(1) @binding(0) var<uniform> camera: Camera;

struct Material {
    albedo: vec4<f32>,
    attenuation: vec4<f32>,
    roughness: f32,
    emission: f32,
    ior: f32,
    _padding: f32,
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

var<private> seed: f32;
var<private> screen_size: vec2<u32>;
var<private> screen_pos: vec2<u32>;
var<private> rand_val: vec2<f32>;
var<private> pi: f32 = 3.1415926535897932384626433832795;

// Constants
var<private> _SAMPLES: i32 = 10; // Adjust the number of samples as needed

// Flag to indicate if it's the first frame (for buffer initialization)
var<private> first_frame: bool = true;
var<private> sample_count: i32 = 0;

// Main ray tracing function
@compute @workgroup_size(1, 1, 1)
fn main(@builtin(global_invocation_id) GlobalInvocationID: vec3<u32>) {
    // Get the screen size
    let screen_size: vec2<u32> = vec2<u32>(textureDimensions(color_buffer));
    // Calculate screen position
    let screen_pos: vec2<u32> = vec2<u32>(GlobalInvocationID.xy);

    // Initialize pixel_color to zero
    var pixel_color = vec3<f32>(0.0, 0.0, 0.0);

    // Start rand seed
    seed = f32(initRng(screen_pos, screen_size, u32(camera.current_frame_counter)));

    // Multiple Samples as Antialiasing
    for (var color_samples = 0; color_samples < _SAMPLES; color_samples += 1) {
        // Calculate Ray
        var ray = calc_ray(screen_pos, screen_size);

        // Get Color of Objects if hit
        pixel_color += color(ray, 20, 10000.0).xyz;
    }

    // Weighted average of pixel colors
    pixel_color /= f32(_SAMPLES);

    //pixel_color = rand_color();

    // Store the pixel color in the color buffer
    textureStore(color_buffer, vec2<i32>(screen_pos), vec4<f32>(pixel_color, 1.0));
}

fn rand_color() -> vec3<f32> {
    let rand = rngNextFloat();
    if (rand<0.0){
        return vec3<f32>(1.0, 0.0, 0.0);
    } else if (rand<1.0 && rand>0.0){
        return vec3<f32>(0.0, 1.0, 0.0);
    } else {
        return vec3<f32>(0.0, 0.0, 1.0);
    }
}

fn calc_ray(screen_pos: vec2<u32>, screen_size: vec2<u32>) -> Ray {

    //----------Camera----------------
    // Replace these with your camera properties
    let vfov: f32 = 90.0; // Vertical field of view in degrees
    let aspect_ratio: f32 = f32(screen_size.x) / f32(screen_size.y);
    let look_from: vec3<f32> = camera.view_pos.xyz; // Camera position

    //REdefine Lookat from cameralet 
    let look_at: vec3<f32> = camera.view_pos.xyz + normalize(camera.view_proj * vec4<f32>(0.0, 0.0, -1.0, 0.0)).xyz;


    let focus_dist: f32 = 2.5; // Focus distance
    let aperture: f32 = 0.05; // Aperture size

    let theta: f32 = radians(vfov);
    let h: f32 = tan(theta / 2.0);
    let viewport_height: f32 = 2.0 * h * focus_dist;
    let viewport_width: f32 = aspect_ratio * viewport_height;

    let u: f32 = (f32(screen_pos.x) + -0.5+rngNextFloat()) / f32(screen_size.x);   // + Random offset
    let v: f32 = (f32(screen_pos.y) + -0.5+rngNextFloat()) / f32(screen_size.y);

    let w: vec3<f32> = normalize(look_from - look_at);
    let u_axis: vec3<f32> = cross(vec3<f32>(0.0, 1.0, 0.0), w);
    let v_axis: vec3<f32> = -cross(w, u_axis);

    let horizontal: vec3<f32> = viewport_width * u_axis;
    let vertical: vec3<f32> = viewport_height * v_axis;
    let lower_left_corner: vec3<f32> = look_from - 0.5 * horizontal - 0.5 * vertical - w*focus_dist;

    // Depth of field settings
    let lens_radius: f32 = 0.0; // Radius of the lens aperture

    // Randomly sample a point within the lens aperture
    let random_in_unit_disk: vec2<f32> = rngNextVec2InUnitDisk() * lens_radius;
    let lens_offset: vec3<f32> = u_axis * random_in_unit_disk.x + v_axis * random_in_unit_disk.y;

    // Compute the new ray direction with depth of field
    let ray_origin: vec3<f32> = look_from + lens_offset;
    let ray_direction: vec3<f32> = lower_left_corner + u * horizontal + v * vertical - ray_origin;

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

// Function to test for ray-sphere intersection
fn hit_sphere(ray: Ray, sphere: Sphere) -> f32 {
    let oc: vec3<f32> = ray.origin - sphere.center.xyz;
    let a: f32 = dot(ray.direction, ray.direction);
    let b: f32 = 2.0 * dot(oc, ray.direction);
    let c: f32 = dot(oc, oc) - sphere.radius.x * sphere.radius.x;
    let discriminant: f32 = b * b - 4.0 * a * c;

    if (discriminant < -0.00001) {         // If Noise in Sphere rendering is visible, increase this value.
        return -1.0;
    } else {
        return (-b - sqrt(discriminant)) / (2.0 * a);
    }
}

fn sky_color(ray: Ray) -> vec3<f32> {
    let unit_direction: vec3<f32> = normalize(ray.direction);
    let t: f32 = 0.5 * (unit_direction.y + 1.0);
    return mix(vec3<f32>(1.0, 1.0, 1.0), vec3<f32>(0.5, 0.7, 1.0), t);
}

fn color(primary_ray: Ray, MAX_BOUNCES: i32, t_max: f32) -> vec4<f32> {
    let MAX_COLOR: f32 = 1.0;

    var depth = 0;
    var ray: Ray = primary_ray;

    // Initialize pixel_color to background color
    var pixel_color = vec3<f32>(sky_color(ray));
    var weight = 1.0;

    while (depth <= MAX_BOUNCES) {
        var t = t_max;
        var closest_sphere: Sphere;
        var closest_tris: Triangle;
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
        for (var j = 0; j < 3; j = j + 1) {
            var hit: f32 = hit_tri(ray, triangles[j]);
            if (hit > 0.0 && hit < t) {
                t = hit;
                closest_tris = triangles[j];
                is_sphere = false;
            }
        }

        // Return background color if no object is hit
        if (t == t_max) {
            if (depth == 0){
                return vec4<f32>(pixel_color, 1.0);
            } else {
                pixel_color = mix(pixel_color, sky_color(ray), weight);
                return vec4<f32>(pixel_color, 1.0);
            }
        }

        // Get color of the closest hit object and reflect ray if needed
        if (is_sphere) {
            let hit_point: vec3<f32> = ray.origin + ray.direction * t;
            let normal: vec3<f32> = normalize(hit_point - closest_sphere.center.xyz);

            // Emission
            if (closest_sphere.material.emission > 0.0) {
                // Handle emissive material directly
                pixel_color += closest_sphere.material.albedo.xyz * closest_sphere.material.emission * weight;
                return vec4<f32>(pixel_color, 1.0); // Terminate the loop when an emissive object is hit

            } else if (closest_sphere.material.ior > 0.0) {
            // Dielectric
                ray = dielectric_scatter(ray, hit_point, normal, closest_sphere.material);
            } else {
                // Accumulate color for non-emissive objects
                pixel_color *= closest_sphere.material.albedo.xyz;

                var reflected_direction: vec3<f32> = reflect(ray.direction, normal + rngNextVec3InUnitSphere() * closest_sphere.material.roughness);

                ray = Ray(hit_point + normal*0.0001, reflected_direction); //normal*0.01 is a offset to fix z-fighting
            }
          
            is_sphere = false;
        } else {
            let hit_point: vec3<f32> = ray.origin + ray.direction * t;
            let normal: vec3<f32> = normalize(closest_tris.normals.xyz);
            let reflected_direction: vec3<f32> = reflect(ray.direction, normal + rngNextVec3InUnitSphere() * closest_tris.material.roughness);

            ray = Ray(hit_point + normal*0.0001, reflected_direction); //normal*0.01 is a offset to fix z-fighting

            // Emission
            if (closest_tris.material.emission > 0.0) {
                // Handle emissive material directly
                pixel_color += closest_tris.material.albedo.xyz * closest_tris.material.emission * weight;
                return vec4<f32>(pixel_color, 1.0); // Terminate the loop when an emissive object is hit
            } else {
                // Accumulate color for non-emissive objects
                pixel_color *= closest_tris.material.albedo.xyz;
            }
            
            // Accumulate color for non-emissive objects
            pixel_color *= closest_tris.material.albedo.xyz;
        }

        weight *= closest_sphere.material.attenuation.x; // Update weight based on material attenuation
        depth += 1;
    }

    // Return the accumulated color as the pixel color
    return vec4<f32>(pixel_color, 1.0);
}

// Dielectric material function
fn dielectric_scatter(ray: Ray, hit_point: vec3<f32>, normal: vec3<f32>, material: Material) -> Ray {
    var etai_over_etat: f32;
    if (dot(ray.direction, normal) > 0.0) {
        etai_over_etat = material.ior;
    } else {
        etai_over_etat = 1.0 / material.ior;
    };

    let unit_direction: vec3<f32> = normalize(ray.direction);
    let cos_theta: f32 = min(dot(-unit_direction, normal), 1.0);
    let sin_theta: f32 = sqrt(1.0 - cos_theta * cos_theta);

    let reflect_prob: f32 = schlick(cos_theta, etai_over_etat);

    let cannot_refract = etai_over_etat * sin_theta > 1.0;

    if ((rngNextFloat() - 0.5) < reflect_prob || cannot_refract) {
        // Reflect
        let reflected_direction: vec3<f32> = reflect(unit_direction, normal);
        return Ray(hit_point, reflected_direction);   //normal*0.01 is a offset to fix z-fighting
    } else {
        // Refract
        let refracted_direction: vec3<f32> = refract(unit_direction, normal, etai_over_etat);
        return Ray(hit_point, refracted_direction);   //normal*0.01 is a offset to fix z-fighting
    }
}

fn reflect(v: vec3<f32>, n: vec3<f32>) -> vec3<f32> {
    return v - 2.0 * dot(v, n) * n;
}

fn refract(direction: vec3<f32>, normal: vec3<f32>, etai_over_etat: f32) -> vec3<f32> {
    let cos_theta: f32 = dot(-direction, normal);
    let r_out_parallel: vec3<f32> = etai_over_etat * (direction + cos_theta * normal);
    let r_out_perp: vec3<f32> = -sqrt(1.0 - length_squared(r_out_parallel)) * normal;
    return r_out_parallel + r_out_perp;
}

fn schlick(cosine: f32, ref_idx: f32) -> f32 {
    var r0: f32 = (1.0 - ref_idx) / (1.0 + ref_idx);
    r0 = r0 * r0;
    return r0 + (1.0 - r0) * pow((1.0 - cosine), 5.0);
}

fn length_squared(v: vec3<f32>) -> f32 {
    return dot(v, v);
}





// RAND FUNCTIONS
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

fn rand_on_hemisphere(normal: vec3<f32>, roughness: vec2<f32>) -> vec3<f32> {
    var dir = rand_vec3_in_unit_sphere(roughness);
    if (dot(dir, normal) < 0.0) {
        dir = -dir;
    }
    return dir;

}



fn rngNextFloat() -> f32 {
    let state = seed;
    rngNextInt();
    return state / f32(0xffffffffu);
}

fn rngNextInt() -> f32 {
    // PCG random number generator
    // Based on https://www.shadertoy.com/view/XlGcRh

    let oldState = u32(seed) + 747796405u + 2891336453u;
    let word = ((oldState >> ((oldState >> 28u) + 4u)) ^ oldState) * 277803737u;
    seed = f32((word >> 22u) ^ word);
    return seed;
}

fn initRng(pixel: vec2<u32>, resolution: vec2<u32>, frame: u32) -> u32 {
    // Adapted from https://github.com/boksajak/referencePT
    let seed = u32(dot(vec2<f32>(pixel), vec2<f32>(1.0, f32(resolution.x)))) ^ jenkinsHash(frame);
    return jenkinsHash(seed);
}

fn jenkinsHash(input: u32) -> u32 {
    var x = input;
    x += x << 10u;
    x ^= x >> 6u;
    x += x << 3u;
    x ^= x >> 11u;
    x += x << 15u;
    return x;
}

fn rngNextVec3InUnitSphere() -> vec3<f32> {
    // r^3 ~ U(0, 1)
    let r = pow(rngNextFloat(), 0.33333f);
    let theta = pi * rngNextFloat();
    let phi = 2f * pi * rngNextFloat();

    let x = r * sin(theta) * cos(phi);
    let y = r * sin(theta) * sin(phi);
    let z = r * cos(theta);

    return vec3(x, y, z);
}

fn rngNextVec2InUnitDisk() -> vec2<f32> {
    let r = sqrt(rngNextFloat()); // Square root for disk
    let theta = 2.0 * pi * rngNextFloat();

    let x = r * cos(theta);
    let y = r * sin(theta);

    return vec2(x, y);
}