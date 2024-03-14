@group(0) @binding(0) var color_buffer: texture_storage_2d<rgba8unorm, write>;
@group(3) @binding(0) var temporal_color_buffer: sampler;
// Camera
struct Camera {
    frame: vec4<f32>,
    view_pos: vec4<f32>, // 4. is a frame counter
    view_proj: mat4x4<f32>,
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
    uv1: vec4<f32>,
    uv2: vec4<f32>, // Z of first tris is count of triangles
    material_texture_id: vec4<f32>, //material_id, texture_id, 0.0, 0.0
}
@group(2) @binding(0) var<storage> triangles : array<Triangle>;

struct Sphere {
    center: vec4<f32>,
    radius: vec4<f32>,
    material_texture_id: vec4<f32>, //material_id, texture_id, 0.0, 0.0
}
@group(2) @binding(1) var<storage> spheres : array<Sphere>;

struct Ray {
    origin: vec3<f32>,
    direction: vec3<f32>,
}

struct BVHTraversal {
    nodeIdx: i32,
}

@group(3) @binding(0) var texture_sampler : sampler;
@group(3) @binding(1) var diffuse: texture_2d_array<f32>;
@group(3) @binding(2) var normal: texture_2d_array<f32>;
@group(3) @binding(3) var roughness: texture_2d_array<f32>;
@group(3) @binding(4) var<storage> materials: array<Material>;

struct BVHNodes {
    min: vec4<f32>,
    max: vec4<f32>,
    extra1: vec4<f32>,
    extra2: vec4<f32>, 
}

@group(4) @binding(0) var<storage> bvh: array<BVHNodes>;
@group(4) @binding(1) var<storage> bvh_prim_indices: array<f32>;

var<private> seed: f32;
var<private> screen_size: vec2<u32>;
var<private> screen_pos: vec2<u32>;
var<private> rand_val: vec2<f32>;
var<private> pi: f32 = 3.1415926535897932384626433832795;

// Constants
var<private> _SAMPLES: i32 = 1; // Adjust the number of samples as needed

// Flag to indicate if it's the first frame (for buffer initialization)
var<private> first_frame: bool = true;
var<private> sample_count: i32 = 0;

// Initialize pixel_color to zero
var<private> pixel_color: vec3<f32> = vec3<f32>(0.0, 0.0, 0.0);

// Main ray tracing function
@compute @workgroup_size(1, 1, 1)
fn main(@builtin(global_invocation_id) GlobalInvocationID: vec3<u32>) {
    // Get the screen size
    let screen_size: vec2<u32> = vec2<u32>(textureDimensions(color_buffer));
    // Calculate screen position
    let screen_pos: vec2<u32> = vec2<u32>(GlobalInvocationID.xy);


    // Start rand seed
    seed = f32(initRng(screen_pos, screen_size, u32(camera.frame[0])));

    // // Multiple Samples as Antialiasing
    // for (var color_samples = 0; color_samples < _SAMPLES; color_samples += 1) {
    //     // Calculate Ray
    //     var ray = calc_ray(screen_pos, screen_size);

    //     pixel_color += color(ray, 10, 10000.0).xyz;
    // }

    // Define colors for each BVH layer
    let bvh_colors: array<vec3<f32>, 4> = array<vec3<f32>, 4>(
        vec3<f32>(1.0, 0.0, 0.0), // Red
        vec3<f32>(0.0, 1.0, 0.0), // Green
        vec3<f32>(0.0, 0.0, 1.0), // Blue
        vec3<f32>(1.0, 1.0, 0.0)  // Yellow
    );

    // Multiple Samples as Antialiasing
    for (var color_samples = 0; color_samples < _SAMPLES; color_samples += 1) {
        // Calculate Ray
        var ray = calc_ray(screen_pos, screen_size);

        // Check if a BVH node is hit
        var hit_bvh = -1.0;

        hit_bvh = f32(intersectBVH(ray));
        
        // for (var i = 0; i < i32(arrayLength(&bvh)); i = i + 1) {
        //     var temp = intersectBVHNode(bvh[i], ray, 0.0, 10000.0);
        //     if (temp > -1.0) {
        //         hit_bvh += 1.0;
        //     }
            
        // }
        pixel_color += vec3<f32>(0.2, 0.0, 0.0) * hit_bvh;
        if (hit_bvh > 0.0) {
            // BVH node hit, color the pixel red
            let color_index = i32(hit_bvh) % 4; // Adjust 4 based on the number of layers
            if color_index == 0 {
                pixel_color += vec3<f32>(1.0, 0.0, 0.0);
            } else if color_index == 1 {
                pixel_color += vec3<f32>(0.0, 1.0, 0.0);
            } else if color_index == 2 {
                pixel_color += vec3<f32>(0.0, 0.0, 1.0);
            } else {
                pixel_color += vec3<f32>(1.0, 1.0, 0.0);
            }
            // pixel_color += mix(color(ray, 10, 10000.0).xyz, vec3<f32>(0.3, 0.0, 0.0) * hit_bvh, 0.9);
        } else {
            // No BVH node hit, color the pixel based on your ray tracing logic
            pixel_color += mix(color(ray, 10, 10000.0).xyz, vec3<f32>(0.0, 0.0, 0.0), 0.4);
        }
    }

    // Weighted average of pixel colors
    pixel_color /= f32(_SAMPLES);

    //pixel_color = rand_color();   

    // Store the pixel color in the color buffer
    textureStore(color_buffer, vec2<i32>(screen_pos), vec4<f32>(pixel_color, 1.0));
}

// New intersection function for BVH
fn intersectBVH(ray: Ray) -> i32 {
    var hit_bvh: i32 = -1;
    var hit_bvh_count: i32 = 0;
    var colsest_prim: f32 = 10000.0;

    // Traverse the BVH
    var todo: array<BVHTraversal, 32>;
    var stacknr: i32 = 0;
    todo[stacknr].nodeIdx = 0;
    
    while (stacknr >= 0) {
        let nodeIdx = todo[stacknr].nodeIdx;
        stacknr = stacknr - 1;

        let node = bvh[nodeIdx];
        // if (rayIntersectsBox(ray, node.min.xyz, node.max.xyz, 0.0, 10000.0) != -1.0) {
            // If the ray intersects the BVH node's bounding box
            if (node.extra1.x > -1.0) {
                // If it's a leaf node
                //check if the triangel is hit
                for (var i = 0; i < i32(node.extra1.x); i = i + 1) {
                    let primID = i32(bvh_prim_indices[i32(node.extra2.x)+i]);
                    var hit: f32 = hit_tri(ray, triangles[primID]);
                    // for (var j = 0; j < i32(triangles[0].vertex1.x); j = j + 1) {   // Amount of triangles -> i32(triangles[0].texture_coords2.z)
                    //     hit += hit_tri(ray, triangles[j]);
                    // }
                    if (hit > 0.0) {
                        pixel_color = vec3<f32>(1.0, 1.0, 1.0);
                        if (hit < colsest_prim){
                            colsest_prim = hit;
                            hit_bvh = primID;
                        }
                        hit_bvh_count += 1;
                    }
                    // hit_bvh = i32(hit);
                }
                // let primID = i32(bvh_prim_indices[i32(node.extra1.x)+0]);
                // var hit: f32 = hit_tri(ray, triangles[primID]);
                // if (hit > 0.0) {
                //     pixel_color = vec3<f32>(1.0, 1.0, 1.0);
                // }
                // hit_bvh = i32(hit);

            } else {
                let leftChildIdx = i32(node.extra2.x);
                let rightChildIdx = leftChildIdx + 1;

                let left_hit = intersectBVHNode(bvh[i32(node.extra2.x)], ray, 0.0, 10000.0);
                let right_hit = intersectBVHNode(bvh[i32(node.extra2.x)+1 ], ray, 0.0, 10000.0);

                if (left_hit != -1.0 && right_hit != -1.0) {
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
                }

                if (left_hit > -1.0) {
                    stacknr = stacknr + 1;
                    todo[stacknr].nodeIdx = i32(node.extra2.x);
                }

                if (right_hit > -1.0) {
                    stacknr = stacknr + 1;
                    todo[stacknr].nodeIdx = i32(node.extra2.x) + 1;
                }

                // // If it's an internal node, push its children to the stack for traversal
                // let leftChildIdx = i32(node.extra2.x);
                // let rightChildIdx = leftChildIdx + 1;
                // todo[stacknr + 1].nodeIdx = leftChildIdx;
                // stacknr = stacknr + 1;
                // todo[stacknr + 1].nodeIdx = rightChildIdx;
                // stacknr = stacknr + 1;
            }
        // }
    }

    return hit_bvh_count;
}



fn intersectPrimitive(ray: Ray, prim_index: i32) -> f32 {
    // Check if a Triangle is hit
    var hit: f32 = hit_tri(ray, triangles[prim_index]);
    if (hit > 0.0) {
        return hit;
    }

    return -1.0;
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

    // Redefine Lookat from cameralet 
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
        for (var j = 0; j < i32(triangles[0].uv2.z); j = j + 1) {   // Amount of triangles -> i32(triangles[0].texture_coords2.z)
            var hit: f32 = hit_tri(ray, triangles[j]);
            if (hit > 0.0 && hit < t) {
                t = hit;
                closest_tris = triangles[j];
                is_sphere = false;
            }
        }

        // Check if a BVH node is hit
        // var hit_bvh = intersectBVH(ray);
        // if (hit_bvh > -1) {
        //     // Set 'bvh_hit' to the index of the hit BVH node
        //     t = 20.0;
        //     closest_tris = triangles[hit_bvh];
        //     is_sphere = false;
        // }
        

        // Return background color if no object is hit
        if (t == t_max) {
            if (depth == 0){
                return vec4<f32>(sky_color(ray), 1.0);
            } else {
                pixel_color = mix(pixel_color, sky_color(ray), weight);
                return vec4<f32>(pixel_color, 1.0);
            }
        }
        
        let hit_point: vec3<f32> = ray.origin + ray.direction * t;
        var uv: vec2<f32> = sphereUVMapping(hit_point, closest_sphere);
        var normal: vec3<f32>;
        var material: Material;
        var texture_id: i32;
        if (is_sphere){
            normal = normalize(hit_point - closest_sphere.center.xyz);
            material = materials[i32(closest_sphere.material_texture_id[0])];
            uv = sphereUVMapping(hit_point, closest_sphere);
            texture_id = i32(closest_sphere.material_texture_id[1]);
        } else {
            normal = normalize(closest_tris.normals.xyz);
            material = materials[i32(closest_tris.material_texture_id[0])];
            uv = trisUVMapping(hit_point, closest_tris);
            texture_id = i32(closest_tris.material_texture_id[1]);
        }

        // Update color
        if texture_id > -1 {
            pixel_color *= get_texture_color(texture_id, uv, 0);
        } else if (material.emission > 0.0) {
            // Handle emissive material directly
            pixel_color += material.albedo.xyz * material.emission * weight;
            return vec4<f32>(pixel_color, 1.0); // Terminate the loop when an emissive object is hit
        } else {
            pixel_color *= material.albedo.xyz;
        }

        // Calculate new ray
        if (texture_id > -1){
            ray = Ray(hit_point + normal*0.0001, reflect(ray.direction, normal * get_texture_color(texture_id, uv, 1) + rngNextVec3InUnitSphere() * (vec3<f32>(1.0)-get_texture_color(texture_id, uv, 2))));
        } else if (material.ior > 0.0) {
            ray = dielectric_scatter(ray, hit_point, normal, material);
        } else {
            ray = Ray(hit_point + normal*0.0001, reflect(ray.direction, normal + rngNextVec3InUnitSphere() * material.roughness)); //normal*0.01 is a offset to fix z-fighting
        }

        weight *= material.attenuation.x; // Update weight based on material attenuation
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




// Textures
fn get_texture_color(texture_id: i32, uv: vec2<f32>, tex_flavour: i32) -> vec3<f32> {
    var texture_color : vec3<f32>;
    if tex_flavour == 0{
        texture_color = textureSampleLevel(diffuse, texture_sampler, uv, texture_id, 0.0).xyz;   
    } else if tex_flavour == 1{
        texture_color = textureSampleLevel(normal, texture_sampler, uv, texture_id, 0.0).xyz;   
    } else {
        texture_color = textureSampleLevel(roughness, texture_sampler, uv, texture_id, 0.0).xyz; 
    }

    return texture_color;
}

fn sphereUVMapping(hit_point: vec3<f32>, sphere: Sphere) -> vec2<f32> {
    let p: vec3<f32> = normalize(hit_point - sphere.center.xyz);
    let phi: f32 = atan2(p.z, p.x);
    let theta: f32 = asin(p.y);
    
    // Normalize phi to the [0, 1] range
    let u: f32 = (phi + pi) / (1.0 * pi);
    
    // Normalize theta to the [0, 1] range, and correct for aspect ratio
    let aspect_ratio: f32 = 1.0; // Adjust this based on your texture
    let v: f32 = (theta + pi / 2.0) / pi * aspect_ratio;
    
    return vec2<f32>(u, v);
}

fn trisUVMapping(hit_point: vec3<f32>, closest_tris: Triangle) -> vec2<f32> {
    let p0 = closest_tris.vertex1.xyz;
    let p1 = closest_tris.vertex2.xyz;	
    let p2 = closest_tris.vertex3.xyz;

    let uv0 = closest_tris.uv1.xy;
    let uv1 = closest_tris.uv1.zw;
    let uv2 = closest_tris.uv2.xy;

    let v0 = p1 - p0;
    let v1 = p2 - p0;
    let v2 = hit_point - p0;

    let dot00 = dot(v0,v0);
    let dot01 = dot(v0,v1);
    let dot02 = dot(v0,v2);
    let dot11 = dot(v1,v1);
    let dot12 = dot(v1,v2);

    let denom = dot00 * dot11 - dot01 * dot01;

    // Calculate barycentric coordinates
    let u = (dot11 * dot02 - dot01 * dot12) / denom;
    let v = (dot00 * dot12 - dot01 * dot02) / denom;

    // Interpolate the UV coordinates
    let interpolated_uv = uv0 * (1.0 - u - v) + uv1 * u + uv2 * v;

    return vec2<f32>(interpolated_uv[0], interpolated_uv[1]);
}


//BVH

// Intersection function for BVH nodes
// Intersection function for BVH nodes
fn intersectBVHNode(node: BVHNodes, ray: Ray, t_min: f32, t_max: f32) -> f32 {
   return rayIntersectsBox(ray, node.min.xyz, node.max.xyz, t_min, t_max);
}

// Ray-box intersection function
fn rayIntersectsBox(ray: Ray, min: vec3<f32>, max: vec3<f32>, t_min: f32, t_max: f32) -> f32 {
    // Ray-box intersection
    var invDirection = 1.0 / ray.direction;
    var t0 = (min - ray.origin) * invDirection;
    var t1 = (max - ray.origin) * invDirection;

    var tMinVec = min(t0, t1);
    var tMaxVec = max(t0, t1);

    var tEnter = max(max(tMinVec.x, tMinVec.y), tMinVec.z);
    var tExit = min(min(tMaxVec.x, tMaxVec.y), tMaxVec.z);

    if (tEnter < tExit) && (tExit > 0.0) && (tEnter < 10000.0) {
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