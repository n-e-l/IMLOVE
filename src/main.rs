use std::sync::{Arc, Mutex};
use ash::vk::{Image, ImageView};
use cen::app::App;
use cen::app::app::AppConfig;
use cen::app::gui::GuiComponent;
use cen::graphics::Renderer;
use cen::graphics::renderer::RenderComponent;
use cen::vulkan::CommandBuffer;
use dotenv::dotenv;

struct Application {
}

impl Application {

    fn new() -> Application {
        Self {
        }
    }
}

impl GuiComponent for Application {
    fn gui(&mut self, context: &egui::Context) {

        // Gui code
        context.input(|_| {
        });

        egui::Window::new("Nodes")
            .resizable(true)
            .title_bar(true)
            .show(context, |_| {
            });
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
