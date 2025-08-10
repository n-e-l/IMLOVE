use ash::vk;
use ash::vk::{AccessFlags, DescriptorSet, ImageLayout, ImageUsageFlags, ImageView, PipelineStageFlags, Sampler};
use cen::app::gui::{GuiComponent, GuiSystem};
use cen::graphics::Renderer;
use cen::graphics::renderer::RenderComponent;
use cen::vulkan::{CommandBuffer, DescriptorSetLayout, Image};
use egui::{ImageSource, TextureId, Vec2, Widget};
use egui::load::SizedTexture;
use egui_dock::{DockArea, DockState, NodeIndex, Style};

pub struct Editor {
    pub tree: DockState<String>,
    image: Option<Image>,
    texture_id: Option<TextureId>,
}

impl Editor {
    pub(crate) fn new() -> Self {

        let mut tree = DockState::new(vec!["tab1".to_owned(), "tab2".to_owned()]);

        let [a, b] =
            tree.main_surface_mut()
                .split_left(NodeIndex::root(), 0.3, vec!["tab3".to_owned()]);

        Self { tree, texture_id: None, image: None }
    }
}

struct TabViewer {
    texture_id: TextureId,
}

impl egui_dock::TabViewer for TabViewer {
    type Tab = String;

    fn title(&mut self, tab: &mut Self::Tab) -> egui::WidgetText {
        (&*tab).into()
    }

    fn ui(&mut self, ui: &mut egui::Ui, tab: &mut Self::Tab) {
        ui.label(format!("Content of {tab}"));
        if tab == "tab1" {
            egui::Image::new(ImageSource::Texture(SizedTexture {
                id: self.texture_id,
                size: Vec2 { x: 100., y: 100. }
            })).ui(ui);

            ui.label(format!("Title: {tab}"));
        }
    }
}


impl GuiComponent for Editor {
    fn initialize_gui(&mut self, gui: &mut GuiSystem) {
        if self.texture_id.is_none() {
            assert!(self.image.is_some());
            self.texture_id = Some(gui.create_texture(self.image.as_ref().unwrap()));
        }
    }

    fn gui(&mut self, gui: &GuiSystem, context: &egui::Context) {

        DockArea::new(&mut self.tree)
            .style(Style::from_egui(context.style().as_ref()))
            .show(context, &mut TabViewer { texture_id: self.texture_id.unwrap() });
    }
}

impl RenderComponent for Editor {
    fn initialize(&mut self, renderer: &mut Renderer) {
        self.image = Some(Image::new(
            &renderer.device,
            &mut renderer.allocator,
            100,
            100,
            ImageUsageFlags::STORAGE | ImageUsageFlags::SAMPLED | ImageUsageFlags::TRANSFER_DST
        ));

        let mut command_buffer = renderer.create_command_buffer();
        command_buffer.begin();
        renderer.transition_image(
            &command_buffer,
            self.image.as_ref().unwrap().handle(),
            ImageLayout::UNDEFINED,
            ImageLayout::TRANSFER_DST_OPTIMAL,
            PipelineStageFlags::TRANSFER,
            PipelineStageFlags::TRANSFER,
            AccessFlags::TRANSFER_READ,
            AccessFlags::TRANSFER_WRITE,
        );
        command_buffer.clear_color_image(&self.image.as_ref().unwrap(), ImageLayout::TRANSFER_DST_OPTIMAL, [1., 1., 0., 1.]);
        renderer.transition_image(
            &command_buffer,
            self.image.as_ref().unwrap().handle(),
            ImageLayout::TRANSFER_DST_OPTIMAL,
            ImageLayout::SHADER_READ_ONLY_OPTIMAL,
            PipelineStageFlags::TRANSFER,
            PipelineStageFlags::TRANSFER,
            AccessFlags::TRANSFER_READ,
            AccessFlags::TRANSFER_WRITE,
        );
        command_buffer.end();
        renderer.submit_single_time_command_buffer(command_buffer, Box::new(|| {}));
    }

    fn render(&mut self, renderer: &mut Renderer, command_buffer: &mut CommandBuffer, swapchain_image: &ash::vk::Image, swapchain_image_view: &ImageView) {
    }
}