use egui::{Context, InnerResponse, Margin};
use scene::structs::ShaderConfig;

pub fn raytracing_settings_gui(ui: &Context, shader_config: &mut ShaderConfig) -> InnerResponse<()> {
    egui::SidePanel::left("Raytracing Settings")
        .frame(egui::Frame::default()
            .fill(egui::Color32::from_black_alpha(200))
            .inner_margin(Margin{ left:10.0, right:10.0, top:10.0, bottom:10.0}))
        .show(ui, |ui| {
            ui.heading("Raytracing Settings");
            ui.add(egui::Slider::new(&mut shader_config.ray_max_bounces, 0..=1000).text("Max Bounces"));
            ui.add(egui::Slider::new(&mut shader_config.ray_samples_per_pixel, 1..=50).text("Samples per Pixel"));
            ui.add(egui::Slider::new(&mut shader_config.ray_max_ray_distance, 0.0..=100_000.0).text("Max Ray Distance"));
            ui.separator();
            ui.add(egui::Slider::new(&mut shader_config.ray_focus_distance, 0.0..=5.0).text("Focus Distance"));
            ui.add(egui::Slider::new(&mut shader_config.ray_aperture, 0.0..=0.6).text("Aperture"));
            ui.add(egui::Slider::new(&mut shader_config.ray_lens_radius, 0.0..=0.5).text("Lens Radius"));
            ui.separator();
            // convert to bool
            let mut ray_debug_rand_color: bool = shader_config.ray_debug_rand_color != 0;
            let mut ray_focus_viewer_visible: bool = shader_config.ray_focus_viewer_visible != 0;
            let mut ray_debug_bvh_bounding_box: bool = shader_config.ray_debug_bvh_bounding_box != 0;
            let mut ray_debug_bvh_bounding_color: bool = shader_config.ray_debug_bvh_bounding_color != 0;

            ui.checkbox(&mut ray_debug_rand_color, "Debug Random Colors");
            ui.checkbox(&mut ray_focus_viewer_visible,"Focus Viewer On/Off");
            ui.checkbox(&mut ray_debug_bvh_bounding_box, "Debug BVH Bounding Box");
            ui.checkbox(&mut ray_debug_bvh_bounding_color, "Debug BVH Bounding Color");

            //convert back to int for Pod trait implementation
            shader_config.ray_debug_rand_color = if ray_debug_rand_color { 1 } else { 0 };
            shader_config.ray_focus_viewer_visible = if ray_focus_viewer_visible { 1 } else { 0 };
            shader_config.ray_debug_bvh_bounding_box = if ray_debug_bvh_bounding_box { 1 } else { 0 };
            shader_config.ray_debug_bvh_bounding_color = if ray_debug_bvh_bounding_color { 1 } else { 0 };

            ui.separator();
            // Reset Button
            if ui.button("Reset").clicked() {
                *shader_config = ShaderConfig::default_raytrace(*shader_config);
            }
        })
}