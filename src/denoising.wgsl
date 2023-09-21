// Bindings
@group(0) @binding(0) var color_buffer: texture_storage_2d<rgba8unorm, read_write>;
@group(0) @binding(1) var temporal_buffer: texture_storage_2d<rgba8unorm, read_write>;

struct VertexOutput {
    @builtin(position) Position: vec4<f32>,
    @location(0) TexCoord: vec2<f32>,
};

// Temporal denoising compute shader
@compute @workgroup_size(1, 1, 1)
fn main( @builtin(global_invocation_id) GlobalInvocationID: vec3<u32>) {
    let screen_pos: vec2<u32> = vec2<u32>(GlobalInvocationID.xy);
    let screen_size = textureDimensions(color_buffer);
    
    // Sample the central pixel
    let centralColor: vec4<f32> = textureLoad(color_buffer, vec2<i32>(screen_pos));
    
    let spacial_denoised_color = spacial_denoising(centralColor, screen_pos);

    let bilateral_denoised_color = bilateral_denoising(centralColor, screen_pos);

    let non_local_means_denoised_color = non_local_means_denoising(centralColor, screen_pos);

    let temporal_denoised_color = temporal_denoising(centralColor, screen_pos, textureLoad(temporal_buffer, vec2<i32>(screen_pos)));
    
    // Store the denoised color back into color_buffer with different aproaches in all four corners
    if (f32(screen_pos.x) < f32(screen_size.x)*0.5 && f32(screen_pos.y) < f32(screen_size.y)*0.5){
        textureStore(color_buffer, vec2<i32>(screen_pos), temporal_denoised_color);
    } else if (f32(screen_pos.x) > f32(screen_size.x)*0.5 && f32(screen_pos.y) < f32(screen_size.y)*0.5){
        textureStore(color_buffer, vec2<i32>(screen_pos), bilateral_denoised_color);
    } else if (f32(screen_pos.x) < f32(screen_size.x)*0.5 && f32(screen_pos.y) > f32(screen_size.y)*0.5){
        textureStore(color_buffer, vec2<i32>(screen_pos), non_local_means_denoised_color);
    } else {
        textureStore(color_buffer, vec2<i32>(screen_pos), centralColor);
    }
}

fn spacial_denoising(centralColor: vec4<f32>, screen_pos: vec2<u32>) -> vec4<f32>{
    
    // Initialize an accumulator for the sum of colors
    var sumColor: vec4<f32> = centralColor;
    
    // Define a kernel size (box filter radius)
    let kernelSize: i32 = 3; // Adjust as needed
    
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
     let spatialSigma: f32 = 3.0;  // Spatial standard deviation
     let colorSigma: f32 = 0.8;    // Color standard deviation

    // Initialize an accumulator for the weighted sum of colors
     var weightedSum: vec4<f32> = vec4<f32>(0.0);
     var totalWeight: f32 = 0.0;
    
     // Define a kernel size (box filter radius)
     let kernelSize: i32 = 5; // Adjust as needed
    
     // Iterate through the neighboring pixels
     for (var dx: i32 = -kernelSize; dx <= kernelSize; dx = dx + 1) {
         for (var dy: i32 = -kernelSize; dy <= kernelSize; dy = dy + 1) {
             let offset: vec2<i32> = vec2<i32>(dx, dy);
             let neighborPos: vec2<i32> = vec2<i32>(screen_pos) + offset;
            
             // Sample the color of the neighboring pixel
             let neighborColor: vec4<f32> = textureLoad(color_buffer, neighborPos);
            
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
    let h: f32 = 0.8;                  // Filtering parameter (adjust as needed)


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
    // Blend the current and previous frames (you can adjust the blend factor as needed)
    let blendFactor: f32 = 0.5; // Adjust as needed
    let finalColor: vec4<f32> = mix(previousColor, centralColor, blendFactor);
    
    return finalColor;
}