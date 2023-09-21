// Entry point for the temporal denoising shader
@compute
fn main(
    [[block]] current_frame_color_buffer: buffer<vec4<f32>>,
    [[block]] output_frame_color_buffer: buffer<vec4<f32>>,
    [[group(0), binding(0)]] previous_frame_color_buffer: texture_2d<f32>,
    [[group(0), binding(1)]] previous_frame_sampler: sampler,
    @builtin(global_invocation_id) GlobalInvocationID: vec3<u32>
) {
    let thread_id = vec2<u32>(GlobalInvocationID.x, GlobalInvocationID.y);
    
    // Calculate the normalized texture coordinates based on the thread ID
    let tex_coord = vec2<f32>(f32(thread_id.x) / f32(output_frame_color_buffer.size.x),
                              f32(thread_id.y) / f32(output_frame_color_buffer.size.y));

    // Sample the previous frame's color
    let previous_frame_color: vec4<f32> = textureSample(previous_frame_color_buffer, previous_frame_sampler, tex_coord);

    // Read the current frame's color from the buffer
    let current_frame_color: vec4<f32> = current_frame_color_buffer.read(thread_id);

    // Weight for blending between current and previous frames
    let temporal_weight: f32 = 0.8;  // Adjust this weight as needed

    // Combine the current and previous colors using a weighted average
    let denoised_color: vec4<f32> = (current_frame_color * (1.0 - temporal_weight)) + (previous_frame_color * temporal_weight);

    // Write the denoised color to the output buffer
    output_frame_color_buffer.write(denoised_color, thread_id);
}
