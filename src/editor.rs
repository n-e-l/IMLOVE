use cen::app::gui::GuiComponent;
use egui::Context;

pub struct Editor {

}

impl Editor {
    pub(crate) fn new() -> Editor {
        Editor {}
    }
}

impl Editor {

}

impl GuiComponent for Editor {
    fn gui(&mut self, context: &Context) {
        egui::CentralPanel::default()
            .show(context, |ui| {
                ui.label("IMLOVE");
            });
    }
}