use ash::vk;
use ash::vk::{AccessFlags, BufferImageCopy, BufferUsageFlags, DescriptorSet, DeviceSize, ImageLayout, ImageUsageFlags, ImageView, PipelineStageFlags, Sampler};
use cen::app::gui::{GuiComponent, GuiSystem};
use cen::graphics::Renderer;
use cen::graphics::renderer::RenderComponent;
use cen::vulkan::{Buffer, CommandBuffer, DescriptorSetLayout, Image};
use egui::{ImageSize, ImageSource, Pos2, Rect, Scene, TextureId, Vec2, Widget};
use egui::load::SizedTexture;
use egui_dock::{DockArea, DockState, NodeIndex, Style};
use gpu_allocator::MemoryLocation;
use image::{EncodableLayout, GenericImageView};

pub struct Editor {
    pub tree: DockState<String>,
    image: Option<Image>,
    texture_id: Option<TextureId>,
    tab_viewer: Option<TabViewer>,
}

impl Editor {
    pub(crate) fn new() -> Self {

        let mut tree = DockState::new(vec!["view".to_owned(), "extra".to_owned()]);

        let [a, b] =
            tree.main_surface_mut()
                .split_left(NodeIndex::root(), 0.3, vec!["tools".to_owned()]);

        Self { tree, texture_id: None, image: None,
            tab_viewer: None
        }
    }
}

struct TabViewer {
    texture_id: TextureId,
    texture_size: Vec2,
    scene_rect: Rect
}

impl egui_dock::TabViewer for TabViewer {
    type Tab = String;

    fn title(&mut self, tab: &mut Self::Tab) -> egui::WidgetText {
        (&*tab).into()
    }

    fn ui(&mut self, ui: &mut egui::Ui, tab: &mut Self::Tab) {
        if tab == "view" {
            egui::Frame::group(ui.style())
                .inner_margin(0.0)
                .show(ui, |ui| {
                    let scene = Scene::new()
                        .max_inner_size([350.0, 1000.0])
                        .zoom_range(0.1..=30.0);

                    let mut inner_rect = Rect::NAN;
                    let response = scene
                        .show(ui, &mut self.scene_rect, |ui| {
                            egui::Image::new(ImageSource::Texture(SizedTexture {
                                id: self.texture_id,
                                size: self.texture_size
                            })).ui(ui);
                            inner_rect = ui.min_rect();
                        })
                        .response;

                    if response.double_clicked() {
                        self.scene_rect = inner_rect;
                    }
                });
        }
    }
}


impl GuiComponent for Editor {
    fn initialize_gui(&mut self, gui: &mut GuiSystem) {
        if self.texture_id.is_none() {
            assert!(self.image.is_some());
            self.texture_id = Some(gui.create_texture(self.image.as_ref().unwrap()));
        }

        self.tab_viewer = Some(TabViewer {  texture_id: self.texture_id.unwrap(), scene_rect: Rect::ZERO, texture_size: Vec2::new(self.image.as_ref().unwrap().width as f32, self.image.as_ref().unwrap().height as f32) });
    }

    fn gui(&mut self, gui: &GuiSystem, context: &egui::Context) {

        DockArea::new(&mut self.tree)
            .style(Style::from_egui(context.style().as_ref()))
            .show(context, self.tab_viewer.as_mut().unwrap());
    }
}

impl RenderComponent for Editor {
    fn initialize(&mut self, renderer: &mut Renderer) {

        // Load image from disk
        let im = image::open("./solstice.png").expect("Couldn't load image").to_rgba8();
        let width = im.width();
        let height = im.height();

        // Load image into buffer
        let mut buf = Buffer::new(
            &renderer.device,
            &mut renderer.allocator,
            MemoryLocation::CpuToGpu,
            (width * height * 4) as DeviceSize,
            BufferUsageFlags::TRANSFER_SRC | BufferUsageFlags::TRANSFER_DST
        );

        let map = buf.mapped();
        let pixel_data = im.as_bytes();
        unsafe { std::ptr::copy_nonoverlapping(pixel_data.as_ptr(), map.as_mut_ptr(), pixel_data.len()); }

        self.image = Some(Image::new(
            &renderer.device,
            &mut renderer.allocator,
            im.width(),
            im.height(),
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
        let regions = [
            BufferImageCopy::default()
                .buffer_offset(0)
                .buffer_row_length(0)
                .buffer_image_height(height)
                .image_subresource(vk::ImageSubresourceLayers {
                    aspect_mask: vk::ImageAspectFlags::COLOR,
                    mip_level: 0,
                    base_array_layer: 0,
                    layer_count: 1,
                })
                .image_offset(vk::Offset3D { x: 0, y: 0, z: 0 })
                .image_extent(vk::Extent3D { width, height, depth: 1 })
        ];
        command_buffer.copy_buffer_to_image(
            &buf,
            self.image.as_ref().unwrap(),
            ImageLayout::TRANSFER_DST_OPTIMAL,
            &regions
        );
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