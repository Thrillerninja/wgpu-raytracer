use std::collections::VecDeque;
use egui::{Align2, Context};
use egui_plot::{AxisHints, GridMark, PlotPoints};
use std::ops::RangeInclusive;
use scene::structs::ShaderConfig;

pub fn gui(ui: &Context, fps: &VecDeque<f32>, settings_open: &mut bool, shader_config: &mut ShaderConfig) {
    // Top bar
    egui::TopBottomPanel::top("top").show(ui, |ui| {
        ui.horizontal(|ui| {
            ui.heading("Top bar");
            ui.separator();
            if ui.button("Settings").clicked() {
                *settings_open = !*settings_open;
            }
        });
    });

    // Frame info window
    egui::Window::new("Frame Info")
        .default_open(true)
        .max_width(1000.0)
        .max_height(800.0)
        .default_width(800.0)
        .anchor(Align2::RIGHT_TOP, [0.0, 0.0])
        .frame(egui::Frame::default().fill(egui::Color32::from_black_alpha(150)))
        .title_bar(false)
        .interactable(false)
        .show(&ui, |ui| {
            // show fps counter
            // average fps over the last 100 frames
            let avg_fps: f32 = fps.iter().sum::<f32>() / fps.len() as f32;
            let color = if avg_fps > 60.0 {
                egui::Color32::from_rgb(0, 255, 0) // green
            } else if avg_fps > 30.0 {
                egui::Color32::from_rgb(255, 165, 0) // orange
            } else {
                egui::Color32::from_rgb(255, 0, 0) // red
            };
            ui.colored_label(color, format!("FPS: {:.1}", avg_fps));
            // next line
            
            let mut frame_times: Vec<f32> = fps.iter().map(|x| *x).collect();
            frame_times.reverse();

            let ms_formatter = |mark: GridMark, _digits: usize, _range : &'_ RangeInclusive<f64>| {
                format!("{:}ms", mark.value)
            };

            let y_axis = vec![
                AxisHints::new_y()
                // .label("Frametime")
                .formatter(ms_formatter)
                .max_digits(4),
                // AxisHints::new_y()
                // .formatter(empty_formatter)
                // .placement(egui_plot::HPlacement::Right)
                // .max_digits(1)
                ];
            let x_axis = vec![
                AxisHints::new_x()
                .label("Last 100 Frames")
                .formatter(|mark, _digits, _range| format!("{:}", mark.value))
                .max_digits(3)];
            
            ui.vertical(|ui| {
                ui.colored_label(egui::Color32::WHITE, "Frametimes (ms):");
                egui_plot::Plot::new("plot")
                    .allow_zoom(false)
                    .allow_boxed_zoom(false)
                    .allow_drag(false)
                    .allow_scroll(false)
                    .show_x(false)
                    .show_y(false)
                    .width(200.0)
                    .height(100.0)
                    .custom_y_axes(y_axis)
                    .custom_x_axes(x_axis)
                    .show(ui, |plot_ui| {
                        // get plotpoints from fps
                        let plot_points: PlotPoints = (0..100).map(|i| {
                            [i as f64, ((1.0/frame_times[i])*1000.0) as f64]
                        }).collect();
                        plot_ui.line(egui_plot::Line::new(plot_points).name("Frametimes"));
                    })
            });
        });

    // Settings window
    if *settings_open {
        egui::SidePanel::left("Settings")
            .frame(egui::Frame::default().fill(egui::Color32::from_black_alpha(200)))
            .show(ui, |ui| {
                ui.heading("Settings");
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
                ui.add(egui::Slider::new(&mut shader_config.temporal_den_motion_threshold, 0.0..=0.1).text("Temporal Denoise Motion Threshold"));
                ui.add(egui::Slider::new(&mut shader_config.temporal_den_direction_threshold, 0.0..=0.1).text("Temporal Denoise Direction Threshold"));
                ui.add(egui::Slider::new(&mut shader_config.temporal_den_low_threshold, 0.0..=0.1).text("Temporal Denoise Low Threshold"));
                ui.add(egui::Slider::new(&mut shader_config.temporal_den_low_blend_factor, 0.0..=0.1).text("Temporal Denoise Low Blend Factor"));
                ui.add(egui::Slider::new(&mut shader_config.temporal_den_high_blend_factor, 0.0..=0.1).text("Temporal Denoise High Blend Factor"));
                ui.add(egui::Slider::new(&mut shader_config.spatial_den_cormpare_radius, 0..=100).text("Spatial Denoise Compare Radius"));
                ui.add(egui::Slider::new(&mut shader_config.spatial_den_patch_radius, 0..=100).text("Spatial Denoise Patch Radius"));
                ui.add(egui::Slider::new(&mut shader_config.spatial_den_significant_weight, 0.0..=0.1).text("Spatial Denoise Significant Weight"));
            });
    }
}