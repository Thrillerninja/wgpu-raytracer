use std::collections::VecDeque;
use egui::{Align2, Context};
use egui_plot::{AxisHints, GridMark, PlotPoints};
use std::ops::RangeInclusive;
use scene::ShaderConfig;

use crate::gui_raytracing_settings::raytracing_settings_gui;
use crate::gui_denoising_settings::denoising_settings_gui;
use crate::gui_info::info_gui;


pub struct GuiConfig {
    pub ray_settings_open: bool,
    pub denoise_settings_open: bool,
    pub info_open: bool,
    pub frame_limit: u32,
    pub frame_limit_unlimited: bool
}

impl Default for GuiConfig {
    fn default() -> Self {
        Self {
            ray_settings_open: false,
            denoise_settings_open: false,
            info_open: false,
            frame_limit: 60,
            frame_limit_unlimited: false
        }
    }
}


pub fn gui(ui: &Context, fps: &VecDeque<f32>, gui_config: &mut GuiConfig, shader_config: &mut ShaderConfig) {
    // Top bar
    egui::TopBottomPanel::top("top").show(ui, |ui| {
        ui.horizontal(|ui| {
            ui.label("Settings:");
            if ui.button("Raytracing").clicked() {
                gui_config.ray_settings_open = !gui_config.ray_settings_open;
            }
            ui.separator();

            if ui.button("Denoising").clicked() {
                gui_config.denoise_settings_open = !gui_config.denoise_settings_open;
            }
            ui.separator();
            
            if ui.button("Info").clicked() {
                gui_config.info_open = !gui_config.info_open;
            }
            ui.separator();
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
            // average fps over the last 20 frames
            let avg_fps: f32 = fps.iter().rev().take(20).sum::<f32>() / 20.0;
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

    // Setting windows
    if gui_config.ray_settings_open {
        raytracing_settings_gui(ui, gui_config, shader_config);
    }
    if gui_config.denoise_settings_open {
        denoising_settings_gui(ui, shader_config);
    }
    if gui_config.info_open {
        info_gui(ui);
    }

}