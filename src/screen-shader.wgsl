@group(0) @binding(0) var screen_sampler : sampler;
@group(0) @binding(1) var color_buffer : texture_2d<f32>;

struct VertexOutput {
    @builtin(position) Position : vec4<f32>,
    @location(0) TexCoord : vec2<f32>,
}

@vertex
fn vs_main(@builtin(vertex_index) VertexIndex : u32) -> VertexOutput {

    var positions = array<vec2<f32>, 6>(
        vec2<f32>( 1.0,  1.0),
        vec2<f32>( 1.0, -1.0),
        vec2<f32>(-1.0, -1.0),
        vec2<f32>( 1.0,  1.0),
        vec2<f32>(-1.0, -1.0),
        vec2<f32>(-1.0,  1.0)
    );

    var texCoords = array<vec2<f32>, 6>(
        vec2<f32>(1.0, 0.0),
        vec2<f32>(1.0, 1.0),
        vec2<f32>(0.0, 1.0),
        vec2<f32>(1.0, 0.0),
        vec2<f32>(0.0, 1.0),
        vec2<f32>(0.0, 0.0)
    );

    var output : VertexOutput;
    output.Position = vec4<f32>(positions[VertexIndex], 0.0, 1.0);
    output.TexCoord = texCoords[VertexIndex];
    return output;
}

@fragment
// fn fs_main(@location(0) TexCoord: vec2<f32>) -> @location(0) vec4<f32> {
//     // Define a 3x3 kernel for denoising
//     var kernel: array<vec2<i32>, 9> = array<vec2<i32>, 9>(
//         vec2<i32>(-1, -1), vec2<i32>(0, -1), vec2<i32>(1, -1),
//         vec2<i32>(-1,  0), vec2<i32>(0,  0), vec2<i32>(1,  0),
//         vec2<i32>(-1,  1), vec2<i32>(0,  1), vec2<i32>(1,  1)
//     );

//     // Calculate the division result outside the loop
//     var texture_dims: vec2<f32> = vec2<f32>(textureDimensions(color_buffer));

//     // Accumulate colors for denoising
//     var denoised_color: vec3<f32> = vec3<f32>(0.0);
//     for (var i = 0; i < 9; i = i + 1) {
//         var offset: vec2<i32> = kernel[i];
//         var neighbor_coord: vec2<f32> = TexCoord + vec2<f32>(offset) / texture_dims;
//         var neighbor_color: vec4<f32> = textureSample(color_buffer, screen_sampler, neighbor_coord);
//         denoised_color += neighbor_color.rgb;
//     }

//     // Normalize the accumulated color
//     denoised_color /= 9.0;

//     // Create the final denoised color
//     var final_color: vec4<f32> = textureSample(color_buffer, screen_sampler, TexCoord);
//     final_color = vec4<f32>(denoised_color, 1.0);

//     return final_color;
// }

fn fs_main(@location(0) TexCoord : vec2<f32>) -> @location(0) vec4<f32> {
  return textureSample(color_buffer, screen_sampler, TexCoord);
}