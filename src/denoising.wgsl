// Bindings
@group(0) @binding(0) var color_buffer: texture_storage_2d<rgba8unorm, read_write>;
@group(0) @binding(1) var temporal_buffer: texture_storage_2d<rgba8unorm, read_write>;

struct Camera {
    current_frame_counter: f32,
    view_pos: vec4<f32>,
    view_proj: mat4x4<f32>,
}
@group(0) @binding(2) var<uniform> current_camera: Camera;
@group(0) @binding(3) var<uniform> lastframe_camera: Camera;

@group(0) @binding(4) var<uniform> current_denoising_pass: u32;

@compute @workgroup_size(1, 1, 1)
fn main(@builtin(global_invocation_id) GlobalInvocationID: vec3<u32>) {
    let screen_pos: vec2<u32> = vec2<u32>(GlobalInvocationID.xy);
    let screen_size: vec2<u32> = vec2<u32>(textureDimensions(color_buffer));

    // Sample the central pixel
    let centralColor: vec4<f32> = textureLoad(color_buffer, vec2<i32>(screen_pos));
    let previousColor: vec4<f32> = textureLoad(temporal_buffer, vec2<i32>(screen_pos));

    // Calculate relative movement between frames
    let relative_movement: vec4<f32> = calculate_relative_movement(current_camera, lastframe_camera);

    let relative_direction: f32 = calculate_relative_direction(current_camera, lastframe_camera);

    

    // Combine denoised results based on regions (you can modify this logic)
    var final_color: vec4<f32> = vec4<f32>(0.0);

    if current_denoising_pass == 0u {

        //----------Temporal Denoising----------//
        final_color = adaptive_temporal_denoising(centralColor, screen_pos, previousColor, relative_movement, relative_direction);
        textureStore(temporal_buffer, vec2<i32>(screen_pos), final_color);
    } else {
        //----------Spacial Denoising----------//
        final_color = non_local_means_denoising(centralColor, screen_pos);
    }

    // Store the calculated relative movement as color in color_buffer
    textureStore(color_buffer, vec2<i32>(screen_pos), final_color);
}


//---------Helper Functions---------//
// Function to calculate relative movement between frames
fn calculate_relative_movement(
    current_camera: Camera,
    lastframe_camera: Camera,
) -> vec4<f32> {
    // Calculate the difference between the view or projection matrices
    let view_proj_diff = current_camera.view_proj - lastframe_camera.view_proj;

    // Create a vec4 where the x, y, z components represent translational movement
    // and the w component represents rotational movement.
    let relative_movement = vec4<f32>(current_camera.view_pos - lastframe_camera.view_pos);

    return vec4<f32>(relative_movement);
}

fn calculate_relative_direction(
    current_camera: Camera,
    lastframe_camera: Camera,
) -> f32 {
    // Calculate the difference between the view or projection matrices
    let view_proj_diff = current_camera.view_proj - lastframe_camera.view_proj;
    
    // You can calculate the Frobenius norm (L2 norm) of the difference matrix
    // to represent the magnitude of rotational movement.
    // This is just one way to quantify the movement; you can adjust it as needed.
    let rotation_magnitude = sqrt(
        view_proj_diff[0][0] * view_proj_diff[0][0] +
        view_proj_diff[0][1] * view_proj_diff[0][1] +
        view_proj_diff[0][2] * view_proj_diff[0][2] +
        view_proj_diff[0][3] * view_proj_diff[0][3] +
        view_proj_diff[1][0] * view_proj_diff[1][0] +
        view_proj_diff[1][1] * view_proj_diff[1][1] +
        view_proj_diff[1][2] * view_proj_diff[1][2] +
        view_proj_diff[1][3] * view_proj_diff[1][3] +
        view_proj_diff[2][0] * view_proj_diff[2][0] +
        view_proj_diff[2][1] * view_proj_diff[2][1] +
        view_proj_diff[2][2] * view_proj_diff[2][2] +
        view_proj_diff[2][3] * view_proj_diff[2][3] +
        view_proj_diff[3][0] * view_proj_diff[3][0] +
        view_proj_diff[3][1] * view_proj_diff[3][1] +
        view_proj_diff[3][2] * view_proj_diff[3][2] +
        view_proj_diff[3][3] * view_proj_diff[3][3]
    );

    return rotation_magnitude*10.0;
}


//---------Denoising Functions---------//
fn spacial_denoising(centralColor: vec4<f32>, screen_pos: vec2<u32>) -> vec4<f32>{
    
    // Initialize an accumulator for the sum of colors
    var sumColor: vec4<f32> = centralColor;
    
    // Define a kernel size (box filter radius)
    let kernelSize: i32 = 2; // Adjust as needed
    
    // Iterate through the neighboring pixels
    for (var dx: i32 = -kernelSize; dx <= kernelSize; dx = dx + 1) {
        for (var dy: i32 = -kernelSize; dy <= kernelSize; dy = dy + 1) {
            let offset: vec2<i32> = vec2<i32>(dx, dy);
            let neighborColor: vec4<f32> = textureLoad(color_buffer, vec2<i32>(screen_pos) + offset);
            sumColor = sumColor + neighborColor;
        }
    }

    // Calculate the average color by dividing by the number of samples
    let numSamples: f32 = f32((2 * kernelSize + 1) * (2 * kernelSize + 1));
    let denoisedColor: vec4<f32> = sumColor / numSamples;
    
    return denoisedColor;
}

fn bilateral_denoising(centralColor: vec4<f32>, screen_pos: vec2<u32>) -> vec4<f32> {
     // Bilateral filter parameters
     let spatialSigma: f32 = 100.0;  // Spatial standard deviation
     let colorSigma: f32 = 20.0;    // Color standard deviation

    // Initialize an accumulator for the weighted sum of colors
     var weightedSum: vec4<f32> = vec4<f32>(0.0);
     var totalWeight: f32 = 0.0;
    
     // Define a kernel size (box filter radius)
     let kernelSize: i32 = 3; // Adjust as needed
    
     // Iterate through the neighboring pixels
     for (var dx: i32 = -kernelSize; dx <= kernelSize; dx = dx + 1) {
         for (var dy: i32 = -kernelSize; dy <= kernelSize; dy = dy + 1) {
             let offset: vec2<i32> = vec2<i32>(dx, dy);
             let neighborPos: vec2<i32> = vec2<i32>(screen_pos) + offset;
            
             // Sample the color of the neighboring pixel
             let neighborColor: vec4<f32> = textureLoad(temporal_buffer, neighborPos);
            
             // Calculate the spatial and color weights
             let spatialDist: f32 = length(vec2<f32>(offset));
             let colorDist: f32 = length(centralColor.rgb - neighborColor.rgb);
            
             let spatialWeight: f32 = exp(-spatialDist * spatialDist / (2.0 * spatialSigma * spatialSigma));
             let colorWeight: f32 = exp(-colorDist * colorDist / (2.0 * colorSigma * colorSigma));
            
             // Combine the weights and accumulate the weighted color
             let weight: f32 = spatialWeight * colorWeight;
             weightedSum = weightedSum + neighborColor * weight;
             totalWeight = totalWeight + weight;
         }
     }
    
     // Normalize the weighted sum by the total weight to get the denoised color
     let denoisedColor: vec4<f32> = weightedSum / totalWeight;
     return denoisedColor;
}

fn non_local_means_denoising(centralColor: vec4<f32>, screen_pos: vec2<u32>) -> vec4<f32> {

    /// Initialize an accumulator for the weighted sum of colors
    var weightedSum: vec4<f32> = vec4<f32>(0.0);
    var totalWeight: f32 = 0.0;
    
    // NLM denoising parameters
    let searchWindowRadius: i32 = 13;   // Radius of the search window
    let patchRadius: i32 = 3;          // Radius of the comparison patch
    let h: f32 = 0.08;                  // Filtering parameter (adjust as needed)


     for (var dx: i32 = -patchRadius; dx <= patchRadius; dx = dx + 1) {
         for (var dy: i32 = -patchRadius; dy <= patchRadius; dy = dy + 1) {
            let offset: vec2<i32> = vec2<i32>(dx, dy);
            let neighborPos: vec2<i32> = vec2<i32>(screen_pos) + offset;
            
            // Sample the color of the neighboring pixel
            let neighborColor: vec4<f32> = textureLoad(color_buffer, neighborPos);
            
            // Calculate the color similarity between the central pixel and the neighbor
            let colorDist: f32 = length(centralColor.rgb - neighborColor.rgb);
            let colorSimilarity: f32 = exp(-(colorDist * colorDist) / (2.0 * h * h));
            
            // Calculate the spatial similarity based on the distance
            let spatialDist: f32 = length(vec2<f32>(offset));
            let spatialSimilarity: f32 = exp(-(spatialDist * spatialDist) / (2.0 * f32(patchRadius * patchRadius)));
            
            // Combine the color and spatial similarities and accumulate the weighted color
            let weight: f32 = colorSimilarity * spatialSimilarity;
            weightedSum += neighborColor * weight;
            totalWeight += weight;
         }
     }
    
     // Normalize the weighted sum by the total weight to get the denoised color
     let denoisedColor: vec4<f32> = weightedSum / totalWeight;
     return denoisedColor;
}

fn temporal_denoising(centralColor: vec4<f32>, screen_pos: vec2<u32>, previousColor: vec4<f32>) -> vec4<f32> {
    // Calculate the color difference between centralColor and previousColor
    let colorDifference: f32 = length(centralColor.rgb - previousColor.rgb);
    
    // Define blend factor thresholds (adjust as needed)
    let lowThreshold: f32 = 0.05; // Example threshold for low color difference
    let highThreshold: f32 = 0.2; // Example threshold for high color difference
    
    // Define blend factors for different cases
    let lowBlendFactor: f32 = 0.03; // Adjust as needed
    let highBlendFactor: f32 = 0.2; // Adjust as needed
    
    // Choose the appropriate blend factor based on color difference
    let blendFactor: f32 = mix(lowBlendFactor, highBlendFactor, smoothstep(lowThreshold, highThreshold, colorDifference));
    
    // Blend the current and previous frames
    let finalColor: vec4<f32> = mix(previousColor, centralColor, blendFactor);
    
    return finalColor;
}

fn adaptive_temporal_denoising(centralColor: vec4<f32>, screen_pos: vec2<u32>, previousColor: vec4<f32>, relative_movement: vec4<f32>, relative_direction: f32) -> vec4<f32> {
    // Calculate the color difference between centralColor and previousColor
    let colorDifference: f32 = length(centralColor.rgb - previousColor.rgb);
    
    // Define thresholds for motion detection (adjust as needed)
    let motionThreshold: f32 = 0.02; // Example threshold for motion detection
    let directionThreshold: f32 = 0.01; // Example threshold for direction detection
    let lowThreshold: f32 = 0.05;    // Example threshold for low color difference
    
    // Determine if there's significant camera motion
    let significantMotion: bool = length(relative_movement.xyz) > motionThreshold;
    let significantDirection: bool = relative_direction > directionThreshold;
    
    // Define blend factors for different cases
    let lowBlendFactor: f32 = 0.03; // Adjust as needed
    let highBlendFactor: f32 = 0.2; // Adjust as needed
    
    // Choose the appropriate blend factor based on motion and color difference
    let blendFactor: f32 = mix(
        lowBlendFactor, 
        highBlendFactor, 
        smoothstep(lowThreshold, motionThreshold, colorDifference)
    );
    
    // Apply stronger temporal denoising if there's no significant motion
    var finalColor: vec4<f32> = vec4<f32>(0.0);
    if (significantMotion || significantDirection) {
        finalColor = centralColor;
    } else {
        finalColor = mix(previousColor, centralColor, blendFactor);
    }
    
    return finalColor;
}