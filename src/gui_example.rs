use std::collections::VecDeque;
use egui::{Align2, Context};

pub fn GUI(ui: &Context, fps: &VecDeque<u32>) {
    egui::Window::new("Streamline CFD")
        // .vscroll(true)
        .default_open(true)
        .max_width(1000.0)
        .max_height(800.0)
        .default_width(800.0)
        .resizable(true)
        .anchor(Align2::LEFT_TOP, [0.0, 0.0])
        .show(&ui, |mut ui| {
            //show fps counter
            //average fps over the last 100 frames
            let avg_fps: f32 = fps.iter().sum::<u32>() as f32 / fps.len() as f32;
            ui.label(format!("FPS: {:.1}", avg_fps));

            if ui.add(egui::Button::new("Click me")).clicked() {
                println!("PRESSED")
            }

            ui.label("Slider");
            // ui.add(egui::Slider::new(_, 0..=120).text("age"));
            ui.end_row();

            // proto_scene.egui(ui);
        });
}