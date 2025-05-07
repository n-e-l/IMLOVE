mod editor;

use std::sync::{Arc, Mutex};
use ash::vk::{Image, ImageView};
use cen::app::App;
use cen::app::app::AppConfig;
use cen::app::gui::GuiComponent;
use cen::graphics::Renderer;
use cen::graphics::renderer::RenderComponent;
use cen::vulkan::CommandBuffer;
use dotenv::dotenv;
use crate::editor::Editor;

struct Application {
    editor: Editor
}

impl Application {

    fn new() -> Application {
        Self {
            editor: Editor::new()
        }
    }
}

impl GuiComponent for Application {
    fn gui(&mut self, context: &egui::Context) {
        self.editor.gui(context);
    }
}

impl RenderComponent for Application {
    fn initialize(&mut self, _: &mut Renderer) {
    }

    fn render(&mut self, _: &mut Renderer, _: &mut CommandBuffer, _: &Image, _: &ImageView) {
    }
}

fn main() {
    // Initialize .env environment variables
    dotenv().ok();

    let application = Arc::new(Mutex::new(Application::new()));
    App::run(
        AppConfig::default()
            .width(1180)
            .height(1180)
            .log_fps(true)
            .vsync(true),
        application.clone(),
        Some(application.clone())
    );
}
