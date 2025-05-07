use cen::app::gui::GuiComponent;
use egui_dock::{DockArea, DockState, NodeIndex, Style};

pub struct Editor {
    pub tree: DockState<String>,
}

impl Editor {
    pub(crate) fn new() -> Self {

        let mut tree = DockState::new(vec!["tab1".to_owned(), "tab2".to_owned()]);

        // You can modify the tree before constructing the dock
        let [a, b] =
            tree.main_surface_mut()
                .split_left(NodeIndex::root(), 0.3, vec!["tab3".to_owned()]);
        let [_, _] = tree
            .main_surface_mut()
            .split_below(a, 0.7, vec!["tab4".to_owned()]);
        let [_, _] = tree
            .main_surface_mut()
            .split_below(b, 0.5, vec!["tab5".to_owned()]);

        Self { tree }
    }
}


impl Editor {

}

struct TabViewer {}

impl egui_dock::TabViewer for TabViewer {
    type Tab = String;

    fn title(&mut self, tab: &mut Self::Tab) -> egui_dock::egui::WidgetText {
        (&*tab).into()
    }

    fn ui(&mut self, ui: &mut egui_dock::egui::Ui, tab: &mut Self::Tab) {
        ui.label(format!("Content of {tab}"));
    }
}


impl GuiComponent for Editor {
    fn gui(&mut self, context: &egui_dock::egui::Context) {
        DockArea::new(&mut self.tree)
            .style(Style::from_egui(context.style().as_ref()))
            .show(context, &mut TabViewer {});
    }
}