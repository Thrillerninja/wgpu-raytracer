use egui::{Context, InnerResponse, Margin, RichText};
use scene::ShaderConfig;


pub fn info_gui(ui: &Context, shader_config: &mut ShaderConfig) -> InnerResponse<()> {

    egui::SidePanel::left("Info")
        .frame(egui::Frame::default()
            .fill(egui::Color32::from_black_alpha(200))
            .inner_margin(Margin{ left:10.0, right:10.0, top:10.0, bottom:10.0})
            )
        .show(ui, |ui| {
            ui.heading("Info");
            ui.label(RichText::new("Controlls").strong());
            ui.label("Movement: WASD");
            ui.label("Up/Down: Space/Shift");
            ui.label("Camera: MouseMovement+Lbutton");
            ui.label(RichText::new("Performance/Safety").strong());
            ui.label("Reduce Shader Setting to min:'x'");
            ui.label(RichText::new("Exit").strong());
            ui.label("Close Programm: 'ESC'");
        })
}
