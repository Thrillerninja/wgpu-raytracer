mod gui;
mod gui_structure;
mod gui_raytracing_settings;
mod gui_denoising_settings;

pub use gui::EguiRenderer;
pub use gui_structure::{GuiConfig, gui};
pub use gui_raytracing_settings::raytracing_settings_gui;
pub use gui_denoising_settings::denoising_settings_gui;




pub fn add(left: usize, right: usize) -> usize {
    left + right
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_works() {
        let result = add(2, 2);
        assert_eq!(result, 4);
    }
}
