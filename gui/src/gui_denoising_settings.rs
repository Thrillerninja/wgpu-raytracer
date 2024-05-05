use egui::{Context, InnerResponse, Margin};
use scene::ShaderConfig;

pub fn denoising_settings_gui(ui: &Context, shader_config: &mut ShaderConfig) -> InnerResponse<()> {
    egui::SidePanel::left("Denoising Settings")
        .frame(egui::Frame::default()
            .fill(egui::Color32::from_black_alpha(200))        
            .inner_margin(Margin{ left:10.0, right:10.0, top:10.0, bottom:10.0}))
        .show(ui, |ui| {
            ui.heading("Denoising Settings");
            ui.separator();
            ui.label("First Denoising Step");
            ui.radio_value(&mut shader_config.first_pass, 0, "Spatial denoising");
            ui.radio_value(&mut shader_config.first_pass, 1, "Bilateral denoising");
            ui.radio_value(&mut shader_config.first_pass, 2, "Non local means denoising");
            ui.radio_value(&mut shader_config.first_pass, 3, "Temporal denoising");
            ui.radio_value(&mut shader_config.first_pass, 4, "Adaptive Temporal denoising");
            ui.radio_value(&mut shader_config.first_pass, 5, "None");
            ui.separator();
            ui.label("Second Denoising Step");
            ui.radio_value(&mut shader_config.second_pass, 0, "Spatial denoising");
            ui.radio_value(&mut shader_config.second_pass, 1, "Bilateral denoising");
            ui.radio_value(&mut shader_config.second_pass, 2, "Non local means denoising");
            ui.radio_value(&mut shader_config.second_pass, 3, "Temporal denoising");
            ui.radio_value(&mut shader_config.second_pass, 4, "Adaptive Temporal denoising");
            ui.radio_value(&mut shader_config.second_pass, 5, "None");
            ui.separator();

            if shader_config.first_pass == 0 || shader_config.second_pass == 0 {
                ui.label("Basic Spatial Denoising Settings");
                ui.add(egui::Slider::new(&mut shader_config.spatial_den_cormpare_radius, 0..=100).text("Compare Radius"));
                ui.add(egui::Slider::new(&mut shader_config.spatial_den_patch_radius, 0..=100).text("Patch Radius"));
                ui.add(egui::Slider::new(&mut shader_config.spatial_den_significant_weight, 0.0..=0.1).text("Significant Weight"));
            }

            if shader_config.first_pass == 1 || shader_config.second_pass == 1 {
                ui.label("Bilateral Denoising Settings");
                ui.add(egui::Slider::new(&mut shader_config.spatial_bilat_space_sigma, 0.0..=200.0).text("Sigma Spatial"));
                ui.add(egui::Slider::new(&mut shader_config.spatial_bilat_color_sigma, 0.0..=200.0).text("Sigma Color"));
                ui.add(egui::Slider::new(&mut shader_config.spatial_bilat_radius, 0..=20).text("Sigma Range"));
            }

            if shader_config.first_pass == 2 || shader_config.second_pass == 2 {
                ui.label("Non Local Means Denoising Settings");
                ui.add(egui::Slider::new(&mut shader_config.spatial_den_cormpare_radius, 1..=100).text("Compare Radius"));
                ui.add(egui::Slider::new(&mut shader_config.spatial_den_patch_radius, 1..=100).text("Patch Radius"));
                ui.add(egui::Slider::new(&mut shader_config.spatial_den_significant_weight, 0.001..=0.1).text("Significant Weight"));
            }

            if shader_config.first_pass == 3 || shader_config.second_pass == 3 {
                ui.label("Adaptive Temporal Denoising Settings");
                ui.add(egui::Slider::new(&mut shader_config.temporal_basic_low_threshold, 0.0..=0.1).text("Low Threshold"));
                ui.add(egui::Slider::new(&mut shader_config.temporal_basic_high_threshold, 0.0..=0.1).text("High Threshold"));
                ui.add(egui::Slider::new(&mut shader_config.temporal_basic_low_blend_factor, 0.0..=0.1).text("Low Blend Factor"));
                ui.add(egui::Slider::new(&mut shader_config.temporal_basic_high_blend_factor, 0.0..=0.1).text("High Blend Factor"));
            }

            if shader_config.first_pass == 4 || shader_config.second_pass == 4 {
                ui.label("Adaptive Temporal Denoising Settings");
                ui.add(egui::Slider::new(&mut shader_config.temporal_adaptive_motion_threshold, 0.0..=0.1).text("Motion Threshold"));
                ui.add(egui::Slider::new(&mut shader_config.temporal_adaptive_direction_threshold, 0.0..=0.1).text("Direction Threshold"));
                ui.add(egui::Slider::new(&mut shader_config.temporal_adaptive_low_threshold, 0.0..=0.1).text("Low Threshold"));
                ui.add(egui::Slider::new(&mut shader_config.temporal_adaptive_low_blend_factor, 0.0..=0.1).text("Low Blend Factor"));
                ui.add(egui::Slider::new(&mut shader_config.temporal_adaptive_high_blend_factor, 0.0..=0.1).text("High Blend Factor"));
            }
            
            ui.separator();
            // Reset Button
            if ui.button("Reset denoising").clicked() {
                *shader_config = ShaderConfig::default_denoise(*shader_config);
            }
        })
}