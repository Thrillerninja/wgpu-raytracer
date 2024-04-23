use egui::{Context, InnerResponse, Margin};
use scene::structs::ShaderConfig;

pub fn denoising_settings_gui(ui: &Context, shader_config: &mut ShaderConfig) -> InnerResponse<()> {
    egui::SidePanel::left("Denoising Settings")
        .frame(egui::Frame::default()
            .fill(egui::Color32::from_black_alpha(200))        
            .inner_margin(Margin{ left:10.0, right:10.0, top:10.0, bottom:10.0}))
        .show(ui, |ui| {
            ui.heading("Denoising Settings");
            ui.label("Temporal Denoising");
            ui.add(egui::Slider::new(&mut shader_config.temporal_den_motion_threshold, 0.0..=0.1).text("Motion Threshold"));
            ui.add(egui::Slider::new(&mut shader_config.temporal_den_direction_threshold, 0.0..=0.1).text("Direction Threshold"));
            ui.add(egui::Slider::new(&mut shader_config.temporal_den_low_threshold, 0.0..=0.1).text("Low Threshold"));
            ui.add(egui::Slider::new(&mut shader_config.temporal_den_low_blend_factor, 0.0..=0.1).text("Low Blend Factor"));
            ui.add(egui::Slider::new(&mut shader_config.temporal_den_high_blend_factor, 0.0..=0.1).text("High Blend Factor"));
            ui.separator();
            ui.label("Spatial Denoising");
            ui.add(egui::Slider::new(&mut shader_config.spatial_den_cormpare_radius, 0..=100).text("Compare Radius"));
            ui.add(egui::Slider::new(&mut shader_config.spatial_den_patch_radius, 0..=100).text("Patch Radius"));
            ui.add(egui::Slider::new(&mut shader_config.spatial_den_significant_weight, 0.0..=0.1).text("Significant Weight"));
            
            ui.separator();
            // Reset Button
            if ui.button("Reset").clicked() {
                *shader_config = ShaderConfig::default_denoise(*shader_config);
            }
        })
}