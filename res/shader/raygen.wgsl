struct Shaderconfig  {
    max_bounces: i32,
    samples: i32,
    max_ray_distance: f32,
    
    focus_distance: f32,
    aperture: f32,
    lens_radius: f32,

    debug_random_color_visible: i32,
    focus_viewer_visible: i32,
    debug_bvh_bounding_visible: i32,
    debug_bvh_bounding_color_visible: i32,

    //denoising
    first_pass: i32,
    second_pass: i32,

    //temporal basic                            //Not used in this shader |
    temporal_basic_low_threshold: f32,          //                        v
    temporal_basic_high_threshold: f32,
    temporal_basic_low_blend_factor: f32,
    temporal_basic_high_blend_factor: f32,

    //temporal adaptive
    temporal_adaptive_motion_threshold: f32,
    temporal_adaptive_direction_threshold: f32,
    temporal_adaptive_low_threshold: f32,
    temporal_adaptive_high_threshold: f32,
    temporal_adaptive_low_blend_factor: f32,
    temporal_adaptive_high_blend_factor: f32,

    //spatial basic
    spatial_kernel_size: i32,
    //spatial bilateral
    spatial_bilat_space_sigma: f32,
    spatial_bilat_color_sigma: f32,
    spatial_bilat_radius: i32,
    //spatial non local means
    spatial_den_cormpare_radius: i32,
    spatial_den_patch_radius: i32,              //                        ^
    spatial_den_significant_weight: f32,        //Not used in this shader |
}
@group(0) @binding(0) var<uniform> config: Shaderconfig;

@group(1) @binding(0) var color_buffer: texture_storage_2d<rgba8unorm, read_write>;// Only needs to be write, but helps with bindgroup generation

// Camera
struct Camera {
    frame: vec4<f32>,
    view_pos: vec4<f32>, // 4. is a frame counter
    view_proj: mat4x4<f32>,
}
@group(2) @binding(0) var<uniform> camera: Camera;

struct Material {
    albedo: vec4<f32>,
    attenuation: vec4<f32>,
    roughness: f32,
    emission: f32,
    ior: f32,
    _padding: f32,
}

struct Background {
    material_ids: vec4<f32>, //material_id, texture_id_diffuse
    intensity: vec4<f32>,
}

@group(4) @binding(0) var texture_sampler: sampler;
@group(4) @binding(1) var textures: texture_2d_array<f32>;
@group(4) @binding(2) var<storage> materials: array<Material>;
@group(4) @binding(3) var<storage> background: Background;
@group(4) @binding(4) var background_texture: texture_2d<f32>;


// Triangles
struct Triangle {
    vertex1: vec4<f32>,
    vertex2: vec4<f32>,
    vertex3: vec4<f32>,
    normals: vec4<f32>,
    tex_coords1: vec4<f32>,
    tex_coords2: vec4<f32>,
    material_texture_ids: vec4<f32>, //material_id, texture_id_diffuse, texture_id_roughness, texture_id_normal
}
@group(3) @binding(0) var<storage> triangles : array<Triangle>;

struct Sphere {
    center: vec4<f32>,
    radius: vec4<f32>,
    material_texture_ids: vec4<f32>, //material_id, texture_id_diffuse, texture_id_roughness, texture_id_normal
}
@group(3) @binding(1) var<storage> spheres : array<Sphere>;

struct Ray {
    origin: vec3<f32>,
    direction: vec3<f32>,
}

struct BVHTraversal {
    nodeIdx: i32,
}

struct BVHNodes {
    min: vec4<f32>,
    max: vec4<f32>,
    extra1: vec4<f32>,
    extra2: vec4<f32>, 
}

@group(5) @binding(0) var<storage> bvh: array<BVHNodes>;
@group(5) @binding(1) var<storage> bvh_prim_indices: array<f32>;


var<private> seed: f32;
var<private> screen_size: vec2<u32>;
var<private> screen_pos: vec2<u32>;
var<private> rand_val: vec2<f32>;
var<private> pi: f32 = 3.1415926535897932384626433832795;

var<private> focus_distance: f32 = 0.0;

// Flag to indicate if it's the first frame (for buffer initialization)
var<private> first_frame: bool = true;
var<private> sample_count: i32 = 0;

// Initialize pixel_color to zero
var<private> pixel_color: vec3<f32> = vec3<f32>(0.0, 0.0, 0.0);

// Main ray tracing function
@compute @workgroup_size(8, 8, 1)
fn main(@builtin(global_invocation_id) GlobalInvocationID: vec3<u32>) {
    // Get the screen size
    let screen_size: vec2<u32> = vec2<u32>(textureDimensions(color_buffer));
    // Calculate screen position
    let screen_pos: vec2<u32> = vec2<u32>(GlobalInvocationID.xy);

    // Start rand seed
    seed = f32(initRng(screen_pos, screen_size, u32(camera.frame[0])));

    // Multiple Samples as Antialiasing (MSAA)
    for (var color_samples = 0; color_samples < config.samples; color_samples += 1) {
        // Calculate Ray
        var ray = calc_ray(screen_pos, screen_size);

        // Debugging options, Focus viewer is toggled in color() function
        if (config.debug_random_color_visible == 1) {
            pixel_color += debug_rand_color();
        } else if (config.debug_bvh_bounding_visible == 1) {
            pixel_color += debug_bvh_bounding(ray);
            pixel_color += color(ray).xyz * 0.2;
        } else if (config.debug_bvh_bounding_color_visible == 1) {
            debug_bvh_bounding_color(ray);
            pixel_color += color(ray).xyz * 0.4;
        } else {
            // Normal color calculation
            pixel_color += color(ray).xyz;
        }

    }
    // Weighted average of pixel colors
    pixel_color /= f32(config.samples);

    // Store the pixel color in the color buffer
    textureStore(color_buffer, vec2<i32>(screen_pos), vec4<f32>(pixel_color, 1.0));
}

fn intersectPrimitive(ray: Ray, prim_index: i32) -> f32 {
    // Check if a Triangle is hit
    var hit: f32 = hit_tri(ray, triangles[prim_index]);
    if (hit > 0.0) {
        return hit;
    }
    return -1.0;
}

fn debug_rand_color() -> vec3<f32> {
    return vec3<f32>(rngNextFloat(), rngNextFloat(), rngNextFloat());
}

fn debug_bvh_bounding(ray: Ray) -> vec3<f32> {
    // draws all the bounding boxes
    // Optimized O(logn) complexity
    var hit_bvh: vec3<f32> = intersectBVH(ray);
    return vec3<f32>(0.1*hit_bvh.z, 0.0, 0.0); // Adjust Scaling factor to make the bounding boxes more visible
}

fn debug_bvh_bounding_color(ray: Ray) {
    // draws all the bounding boxes and colors them
    // Optimized O(logn) complexity
    var hit_bvh: f32 = intersectBVH(ray).z;
    if (hit_bvh > 0.0) {
        // BVH node hit, color the pixel red
        let color_index = i32(hit_bvh) % 4; // Adjust 4 based on the number of layers
        if color_index == 0 {
            pixel_color += vec3<f32>(0.5, 0.0, 0.0);
        } else if color_index == 1 {
            pixel_color += vec3<f32>(0.0, 0.5, 0.0);
        } else if color_index == 2 {
            pixel_color += vec3<f32>(0.0, 0.0, 0.5);
        } else {
            pixel_color += vec3<f32>(0.5, 0.5, 0.0);
        }
        // pixel_color += mix(color(ray, 10, 10000.0).xyz, vec3<f32>(0.3, 0.0, 0.0) * hit_bvh, 0.9);
    } else {
        // No BVH node hit, color the pixel based on ray tracing logic
        pixel_color += mix(color(ray).xyz, vec3<f32>(0.0, 0.0, 0.0), 0.4);
    }
}

fn background_color(ray: Ray) -> vec3<f32> {
    let null_sphere = Sphere(vec4<f32>(vec3<f32>(0.0, 0.0, 0.0), 1.0), vec4<f32>(0.0, 0.0, 0.0, 0.0), vec4<f32>(0.0, 0.0, 0.0, 0.0));
    let uv = sphereUVMapping(-1.0*ray.direction, null_sphere); // *-1 fixes upside down environment
    
    if (background.material_ids.x != -1.0) && (background.material_ids.y != -1.0) {
        return textureSampleLevel(background_texture, texture_sampler, uv, 0.0).xyz * background.intensity.x * materials[i32(background.material_ids.x)].albedo.xyz;
    //} else if (background.material_ids.x != -1.0) {
    //    return background.intensity.x * materials[i32(background.material_ids.x)].albedo.xyz;
    //} else if (background.material_ids.y != -1.0) {
    //    return textureSampleLevel(background_texture, texture_sampler, uv, 0.0).xyz * background.intensity.x;
    } else {
        return sky_color(ray);
    }
}

fn calc_ray(screen_pos: vec2<u32>, screen_size: vec2<u32>) -> Ray {

    //----------Camera----------------
    // Replace these with your camera properties
    let vfov: f32 = camera.frame[1]; // Vertical field of view in degrees
    let aspect_ratio: f32 = f32(screen_size.x) / f32(screen_size.y);
    let look_from: vec3<f32> = camera.view_pos.xyz; // Camera position

    // Redefine Lookat from cameralet 
    let look_at: vec3<f32> = camera.view_pos.xyz + normalize(camera.view_proj * vec4<f32>(0.0, 0.0, -1.0, 0.0)).xyz;


    let focus_dist: f32 = config.focus_distance; // Focus distance
    let aperture: f32 = config.aperture; // Aperture size

    let theta: f32 = radians(vfov);
    let h: f32 = tan(theta / 2.0);
    let viewport_height: f32 = 2.0 * h * focus_dist;
    let viewport_width: f32 = aspect_ratio * viewport_height;

    let u: f32 = (f32(screen_pos.x) + -0.5+rngNextFloat()) / f32(screen_size.x);   // + Random offset
    let v: f32 = (f32(screen_pos.y) + -0.5+rngNextFloat()) / f32(screen_size.y);

    let w: vec3<f32> = normalize(look_from - look_at);
    let u_axis: vec3<f32> = normalize(cross(vec3<f32>(0.0, 1.0, 0.0), w));
    let v_axis: vec3<f32> = -normalize(cross(w, u_axis));

    let horizontal: vec3<f32> = viewport_width * u_axis;
    let vertical: vec3<f32> = viewport_height * v_axis;
    let lower_left_corner: vec3<f32> = look_from - 0.5 * horizontal - 0.5 * vertical - w*focus_dist;

    // Depth of field settings
    let lens_radius: f32 = config.lens_radius; // Radius of the lens aperture (small numbers)

    // Randomly sample a point within the lens aperture
    let random_in_unit_disk: vec2<f32> = rngNextVec2InUnitDisk() * lens_radius;
    let lens_offset: vec3<f32> = u_axis * random_in_unit_disk.x + v_axis * random_in_unit_disk.y;

    // Compute the new ray direction with depth of field
    let ray_origin: vec3<f32> = look_from + lens_offset;
    let ray_direction: vec3<f32> = lower_left_corner + u * horizontal + v * vertical - ray_origin;

    // Create the ray
    return Ray(ray_origin, ray_direction);
}

fn intersectBVH(ray: Ray, ) -> vec3<f32> {
    var hit_bvh: i32 = -1;  //has any hit happened?
    var t: f32 = config.max_ray_distance;     //at what t did it happen?
    var hit_count: f32 = 0.0; //how many hits happened? (Only for debug shader)

    // Traverse the BVH
    var todo: array<BVHTraversal, 32>;  // Stores the nodes to visit to find the closest tris intersection
    var stacknr: i32 = 1;
    todo[stacknr].nodeIdx = 0;

    while (stacknr != 0) {
        let nodeIdx = todo[stacknr].nodeIdx;
        stacknr = stacknr - 1;

        let node = bvh[nodeIdx];
            // If the ray intersects the BVH node's bounding box
            if (node.extra1.x > -1.0) {
                // If it's a leaf node
                // Check if the triangel is hit
                for (var i = 0; i < i32(node.extra1.x); i = i + 1) {
                    let primID = i32(bvh_prim_indices[i32(node.extra2.x)+i]);
                    var hit: f32 = hit_tri(ray, triangles[primID]);

                    if (hit > 0.0) {
                        if (hit < t+0.001){
                            t = hit;
                            hit_bvh = primID;
                        }
                    }
                }

            } else {
                // If it's an internal node move to the next nodes
                let leftChildIdx = i32(node.extra2.x);
                let rightChildIdx = leftChildIdx + 1;

                let left_hit = intersectBox(ray, bvh[leftChildIdx].min.xyz, bvh[leftChildIdx].max.xyz, 0.0);
                let right_hit = intersectBox(ray, bvh[rightChildIdx].min.xyz, bvh[rightChildIdx].max.xyz, 0.0);

                if (left_hit != -1.0 && right_hit != -1.0) { // >= disables bvh improvementes but no missing tris
                    if (left_hit < right_hit) {
                        stacknr = stacknr + 1;
                        todo[stacknr].nodeIdx = leftChildIdx;
                        stacknr = stacknr + 1;
                        todo[stacknr].nodeIdx = rightChildIdx;
                    } else {
                        stacknr = stacknr + 1;
                        todo[stacknr].nodeIdx = rightChildIdx;
                        stacknr = stacknr + 1;
                        todo[stacknr].nodeIdx = leftChildIdx;
                    }
                } else if (left_hit != -1.0) {
                    stacknr = stacknr + 1;
                    todo[stacknr].nodeIdx = i32(node.extra2.x);

                } else if (right_hit != -1.0) {
                    stacknr = stacknr + 1;
                    todo[stacknr].nodeIdx = i32(node.extra2.x) + 1;
                }
                hit_count += 1.0;
            }
    }
    let out = vec3<f32>(f32(hit_bvh), t, hit_count);
    return out;
}

fn hit_tri(ray: Ray, triangle: Triangle) -> f32 {   // https://en.wikipedia.org/wiki/M%C3%B6ller%E2%80%93Trumbore_intersection_algorithm
    let epsilon = 0.0001;
    
    let v0 = triangle.vertex1.xyz;
    let v1 = triangle.vertex2.xyz;
    let v2 = triangle.vertex3.xyz;

    let edge1 = v1 - v0;
    let edge2 = v2 - v0;

    let ray_cross_e2 = cross(ray.direction, edge2);
    let det = dot(edge1, ray_cross_e2);

    if (det > -epsilon) && (det < epsilon){
        return -1.0; // Ray is parallel to the triangle
    }

    let inv_det  = 1.0 / det;
    // Computes Barycentric coordinates.
    let centered = ray.origin - v0;
    
    let u = inv_det  * dot(centered, ray_cross_e2);

    if u < -epsilon || u > 1.0+epsilon {
        return -1.0; // Intersection is outside the triangle's edges
    }

    let centered_cross_e1 = cross(centered, edge1);
    let v = inv_det * dot(ray.direction, centered_cross_e1);

    if v < -epsilon || (u + v) > 1.0+epsilon {
        return -1.0; // Intersection is outside the triangle's edges
    }

    let t = inv_det * dot(edge2, centered_cross_e1);

    if t > 0.0001 { // Adjust this epsilon value based on your scene scale. If Noise in Tris rendering is visible, increase this value.
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

fn color(primary_ray: Ray) -> vec4<f32> {
    let MAX_COLOR: f32 = 1.0;

    var depth = 0;
    var ray: Ray = primary_ray;

    // Initialize pixel_color to background color
    var pixel_color: vec3<f32> = vec3<f32>(1.0, 1.0, 1.0);
    var stacknr: i32 = 0;

    var weight = vec3<f32>(1.0,1.0,1.0);

    while (depth <= config.max_bounces) {
        var t = config.max_ray_distance;
        var closest_sphere: Sphere;
        var closest_tris: Triangle;
        var is_sphere: bool = false;

        // Check if a Sphere is hit
        for (var i = 0; i < i32(arrayLength(&spheres)); i = i + 1) {
            // if no sphere is denfined, a "empty" sphere wth radius is 
            //   added so that the buffer exists and the shader can be compiled
            if spheres[i].radius.x == 0.0 { 
                continue;
            }
            // Check if a Sphere is hit
            var hit: f32 = hit_sphere(ray, spheres[i]);
            if (hit > 0.0 && hit < t) {
                t = hit;
                closest_sphere = spheres[i];
                is_sphere = true;
            }
        }

        // Check if a BVH node is hit
        var hit_bvh: vec3<f32> = intersectBVH(ray);
        if (hit_bvh.x > -1.0 && hit_bvh.y < t) {
            // Set 'bvh_hit' to the index of the hit BVH node
            t = hit_bvh.y;
            closest_tris = triangles[i32(hit_bvh.x)];
            is_sphere = false;
        }
        
        // Return background color if no object is hit
        if (t == config.max_ray_distance) {
            if (depth == 0){
                return vec4<f32>(background_color(ray), 1.0);
            } else {
                pixel_color = mix(pixel_color, background_color(ray), weight); //like this or with weight.x better?
                return vec4<f32>(pixel_color, 1.0);
            }
        }

        // Check for focus distance if focus viewer is enabled
        if (config.focus_viewer_visible == 1) {
            if (depth == 0) {
                if (t > 1.0 - 0.005 && t < 1.0 + 0.005) {
                    return vec4<f32>(1.0, 0.0, 0.0, 1.0);
                }
            }
        }
        
        let hit_point: vec3<f32> = ray.origin + ray.direction * t;
        var normal: vec3<f32>;
        var material: Material;
        // Texture ids
        var texture_id_diffuse: i32;
        var texture_id_roughness: i32;
        var texture_id_normal: i32;
    
        var uv: vec2<f32>;
        if (is_sphere){
            normal = normalize(hit_point - closest_sphere.center.xyz);
            material = materials[i32(closest_sphere.material_texture_ids[0])];
            uv = sphereUVMapping(hit_point, closest_sphere);
            // Texture ids
            texture_id_diffuse = i32(closest_sphere.material_texture_ids[1]);
            texture_id_roughness = i32(closest_sphere.material_texture_ids[2]);
            texture_id_normal = i32(closest_sphere.material_texture_ids[3]);
        } else {
            normal = normalize(closest_tris.normals.xyz);
            material = materials[i32(closest_tris.material_texture_ids[0])];

            //new uv coords
            let tex1 = closest_tris.tex_coords1.xy;
            let tex2 = closest_tris.tex_coords1.zw;
            let tex3 = closest_tris.tex_coords2.xy;
            uv = tex_coord(closest_tris.vertex1.xyz, closest_tris.vertex2.xyz, closest_tris.vertex3.xyz, tex1, tex2, tex3, hit_point);
            // Texture ids
            texture_id_diffuse = i32(closest_tris.material_texture_ids[1]);
            texture_id_roughness = i32(closest_tris.material_texture_ids[2]);
            texture_id_normal = i32(closest_tris.material_texture_ids[3]);
        }

        // Update color
        if texture_id_diffuse > -1 {
            pixel_color *= get_texture_color(texture_id_diffuse, uv);
            weight *= get_texture_color(texture_id_roughness, uv); // Update weight based on material attenuation
        } else if (material.emission > 0.0) {
            // Handle emissive material directly
            if (depth == 0) {
                pixel_color = material.albedo.xyz * material.emission;
            } else{
                pixel_color += material.albedo.xyz * material.emission * weight;
            }
            return vec4<f32>(pixel_color, 1.0); // Terminate the loop when an emissive object is hit
        } else {
            pixel_color *= material.albedo.xyz;
            weight *= material.attenuation.xyz; // Update weight based on material attenuation
        }

        // Calculate new ray
        if (texture_id_roughness > -1 && texture_id_normal > -1){
            ray = Ray(hit_point + normal*0.001, reflect(ray.direction,  get_texture_color(texture_id_normal, uv) + rngNextVec3InUnitSphere() * get_texture_color(texture_id_roughness, uv)));            
        } else if (texture_id_roughness > -1) {
            ray = Ray(hit_point + normal*0.001, reflect(ray.direction, normal + rngNextVec3InUnitSphere() * material.roughness * get_texture_color(texture_id_roughness, uv))); //normal*0.01 is a offset to fix z-fighting
        } else if (texture_id_normal > -1) {
            ray = Ray(hit_point + normal*0.001, reflect(ray.direction, normal * get_texture_color(texture_id_normal, uv)+ rngNextVec3InUnitSphere() * material.roughness)); //normal*0.01 is a offset to fix z-fighting
        } else if (material.ior > 0.0) {
            ray = dielectric_scatter(ray, hit_point, normal, material);
        } else {
            ray = Ray(hit_point + normal*0.001, reflect(ray.direction, normal + rngNextVec3InUnitSphere() * material.roughness)); //normal*0.01 is a offset to fix z-fighting
        }

        weight *= material.attenuation.x; // Update weight based on material attenuation
        depth += 1;
    }
    return vec4<f32>(pixel_color, 1.0);
}

fn tex_coord(tris1_pos: vec3<f32>, tris2_pos: vec3<f32>, tris3_pos: vec3<f32>, tex1: vec2<f32>, tex2: vec2<f32>, tex3: vec2<f32>, hit_point: vec3<f32>) -> vec2<f32> {
    // Barycentric coordinates calculation
    let v0 = tris2_pos - tris1_pos;
    let v1 = tris3_pos - tris1_pos;
    let v2 = hit_point - tris1_pos;

    let dot00 = dot(v0, v0);
    let dot01 = dot(v0, v1);
    let dot02 = dot(v0, v2);
    let dot11 = dot(v1, v1);
    let dot12 = dot(v1, v2);

    let invDenom = 1.0 / (dot00 * dot11 - dot01 * dot01);
    let u = (dot11 * dot02 - dot01 * dot12) * invDenom;
    let v = (dot00 * dot12 - dot01 * dot02) * invDenom;

    let texcoord = tex1 * (1.0 - u - v) +
                   tex2 * u +
                   tex3 * v;

    // Perform texture sampling using texcoord
    // Example: let color = textureSample(texture, texcoord);

    return vec2<f32>(texcoord);
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




// Textures
fn get_texture_color(texture_id: i32, uv: vec2<f32>) -> vec3<f32> {
    return textureSampleLevel(textures, texture_sampler, uv, texture_id, 0.0).xyz;
}

fn sphereUVMapping(hit_point: vec3<f32>, sphere: Sphere) -> vec2<f32> {
    let p: vec3<f32> = normalize(hit_point - sphere.center.xyz);
    let phi: f32 = atan2(p.z, p.x);
    let theta: f32 = acos(p.y);
    
    // Normalize phi and theta to the [0, 1] range
    let u: f32 = phi / (2.0 * pi);
    let v: f32 = (pi - theta) / pi;
    
    return vec2<f32>(u, v);
}

// Ray-box intersection function
fn intersectBox(ray: Ray, min: vec3<f32>, max: vec3<f32>, t_min: f32) -> f32 {
    let epsilon = 0.001; // * length(ray.origin - min); // Adaptive epsilon

    var t0 = (min - ray.origin) / ray.direction;
    var t1 = (max - ray.origin) / ray.direction;
    var tMinVec = min(t0, t1);
    var tMaxVec = max(t0, t1);

    var tEnter = max(max(tMinVec.x, tMinVec.y), tMinVec.z);
    var tExit = min(min(tMaxVec.x, tMaxVec.y), tMaxVec.z);

    tEnter = max(tEnter, t_min) - epsilon;
    tExit = min(tExit, config.max_ray_distance) + epsilon;

    if (tEnter <= tExit) && (tExit > 0.0) && (tEnter < config.max_ray_distance) {
        if (tEnter < 0.0) {
            return 0.0;
        }
        return tEnter;
    } else {
        return -1.0;
    }
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