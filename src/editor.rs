use std::collections::HashMap;
use std::ops::{Range, RangeInclusive};
use ash::vk;
use ash::vk::{AccessFlags, BufferImageCopy, BufferUsageFlags, DescriptorSet, DescriptorSetLayoutBinding, DescriptorType, DeviceSize, ImageAspectFlags, ImageCopy, ImageLayout, ImageSubresourceLayers, ImageUsageFlags, ImageView, Offset3D, PipelineStageFlags, PushConstantRange, Sampler, ShaderStageFlags, WriteDescriptorSet};
use bytemuck::{Pod, Zeroable};
use cen::app::gui::{GuiComponent, GuiSystem};
use cen::graphics::pipeline_store::{PipelineConfig, PipelineKey};
use cen::graphics::Renderer;
use cen::graphics::renderer::RenderComponent;
use cen::vulkan::{Buffer, CommandBuffer, ComputePipeline, DescriptorSetLayout, Image};
use egui::{Color32, ImageSize, ImageSource, Key, Pos2, Rect, Response, Scene, Sense, Slider, StrokeKind, TextureId, Vec2, Widget};
use egui::ecolor::Hsva;
use egui::emath::TSTransform;
use egui::load::SizedTexture;
use egui_dock::{DockArea, DockState, NodeIndex, Style};
use gpu_allocator::MemoryLocation;
use image::{EncodableLayout, GenericImageView};
use okhsl::Okhsl;

pub struct Editor {
    pub tree: DockState<String>,
    image: Option<Image>,
    texture_id: Option<TextureId>,
    tab_viewer: Option<TabViewer>,
    pipeline: Option<PipelineKey>,
    draw_buffer: Option<Image>,
    stencil_buffer: Option<Image>
}

impl Editor {
    pub(crate) fn new() -> Self {

        let mut tree = DockState::new(vec!["view".to_owned(), "extra".to_owned()]);

        let [a, b] =
            tree.main_surface_mut()
                .split_left(NodeIndex::root(), 0.3, vec!["tools".to_owned()]);

        Self {
            tree,
            texture_id: None,
            image: None,
            draw_buffer: None,
            stencil_buffer: None,
            pipeline: None,
            tab_viewer: None,
        }
    }
}

struct TabViewer {
    texture_id: TextureId,
    texture_size: Vec2,
    scene_rect: Rect,
    last_image_pointer: Vec2,
    image_pointer: Vec2,
    pointer_down: bool,
    pointer_held: bool,
    pointer_released: bool,
    space_down: bool,
    compute: bool,
    okhsl: Okhsl
}

impl TabViewer {
    fn multi_color_gradient_slider(
        ui: &mut egui::Ui,
        value: &mut f64,
        range: std::ops::RangeInclusive<f64>,
        colors: &[egui::Color32]
    ) {
        if colors.is_empty() {
            // Fallback to regular slider if no colors provided
            ui.add(egui::Slider::new(value, range));
            return;
        }

        let desired_size = egui::vec2(ui.available_width().min(200.0), 24.0);
        let (rect, _) = ui.allocate_exact_size(desired_size, egui::Sense::hover());

        // Draw multi-color gradient background
        let painter = ui.painter();
        let gradient_rect = rect.shrink(2.0);

        if colors.len() == 1 {
            // Single color - just fill with that color
            painter.rect_filled(gradient_rect, egui::Rounding::same(4), colors[0]);
        } else {
            // Multi-color gradient
            let segments = colors.len() - 1;
            let segment_width = gradient_rect.width() / segments as f32;

            for i in 0..segments {
                let x_start = gradient_rect.min.x + i as f32 * segment_width;
                let x_end = gradient_rect.min.x + (i + 1) as f32 * segment_width;

                let start_color = colors[i];
                let end_color = colors[i + 1];

                let segment_rect = egui::Rect::from_x_y_ranges(
                    x_start..=x_end,
                    gradient_rect.y_range()
                );

                // Create mesh for this segment
                let mut mesh = egui::Mesh::default();
                let idx = mesh.vertices.len() as u32;

                mesh.vertices.push(egui::epaint::Vertex {
                    pos: egui::pos2(x_start, gradient_rect.min.y),
                    uv: egui::pos2(0.0, 0.0),
                    color: start_color,
                });
                mesh.vertices.push(egui::epaint::Vertex {
                    pos: egui::pos2(x_end, gradient_rect.min.y),
                    uv: egui::pos2(1.0, 0.0),
                    color: end_color,
                });
                mesh.vertices.push(egui::epaint::Vertex {
                    pos: egui::pos2(x_end, gradient_rect.max.y),
                    uv: egui::pos2(1.0, 1.0),
                    color: end_color,
                });
                mesh.vertices.push(egui::epaint::Vertex {
                    pos: egui::pos2(x_start, gradient_rect.max.y),
                    uv: egui::pos2(0.0, 1.0),
                    color: start_color,
                });

                mesh.indices.extend_from_slice(&[idx, idx + 1, idx + 2, idx, idx + 2, idx + 3]);
                painter.add(egui::Shape::mesh(mesh));
            }
        }

        // Add border
        painter.rect_stroke(gradient_rect, egui::Rounding::same(4),
                            egui::Stroke::new(1.0, egui::Color32::GRAY), StrokeKind::Inside);

        // Place transparent slider on top
        let layout = egui::Layout::left_to_right(egui::Align::Center);
        ui.allocate_ui_with_layout(rect.size(), layout, |ui| {
            ui.style_mut().visuals.widgets.inactive.bg_fill = egui::Color32::TRANSPARENT;
            ui.style_mut().visuals.widgets.hovered.bg_fill = egui::Color32::TRANSPARENT;
            ui.style_mut().visuals.widgets.active.bg_fill = egui::Color32::TRANSPARENT;
            ui.add_sized(rect.size(), egui::Slider::new(value, range).show_value(false));
        });
    }
}

impl egui_dock::TabViewer for TabViewer {
    type Tab = String;

    fn title(&mut self, tab: &mut Self::Tab) -> egui::WidgetText {
        (&*tab).into()
    }

    fn ui(&mut self, ui: &mut egui::Ui, tab: &mut Self::Tab) {
        if tab == "tools" {
            ui.label("H");
            ui.add(Slider::new(&mut self.okhsl.h, RangeInclusive::new(0.0, 1.0) ));
            ui.label("S");
            ui.add(Slider::new(&mut self.okhsl.s, RangeInclusive::new(0.0, 1.0) ));
            ui.label("L");
            ui.add(Slider::new(&mut self.okhsl.l, RangeInclusive::new(0.0, 1.0) ));
        }

        if tab == "view" {

            ui.input(|input| {
                if let Some(p) = input.pointer.hover_pos() {
                    // Read where we are on the image
                    let frame_rect = ui.min_rect();
                    let mouse_frame_pos = p - frame_rect.min;
                    self.last_image_pointer = mouse_frame_pos;
                    self.image_pointer = mouse_frame_pos / frame_rect.size() * self.scene_rect.size() + self.scene_rect.min.to_vec2();
                }

                self.pointer_held = input.pointer.primary_down() && self.pointer_down;
                self.pointer_down = input.pointer.primary_down();
                self.pointer_released = input.pointer.primary_released();
                self.space_down = input.key_down(Key::Space);
            });

            egui::Frame::group(ui.style())
                .inner_margin(0.0)
                .show(ui, |ui| {
                    let mut scene = Scene::new()
                        .max_inner_size([350.0, 1000.0])
                        .zoom_range(0.1..=30.0);

                    if !self.space_down {
                        scene = scene.sense(Sense::focusable_noninteractive());
                    }

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
            assert!(self.draw_buffer.is_some());
            self.texture_id = Some(gui.create_texture(self.draw_buffer.as_ref().unwrap()));
        }

        self.tab_viewer = Some(TabViewer {
            texture_id: self.texture_id.unwrap(),
            scene_rect: Rect::ZERO,
            texture_size: Vec2::new(self.image.as_ref().unwrap().width as f32, self.image.as_ref().unwrap().height as f32),
            last_image_pointer: Default::default(),
            image_pointer: Default::default(),
            pointer_down: false,
            pointer_held: false,
            pointer_released: false,
            space_down: false,
            compute: false,
            okhsl: Okhsl {
                h: 1.0,
                s: 1.0,
                l: 1.0,
            }
        });
    }

    fn gui(&mut self, gui: &GuiSystem, context: &egui::Context) {
        DockArea::new(&mut self.tree)
            .style(Style::from_egui(context.style().as_ref()))
            .show(context, self.tab_viewer.as_mut().unwrap());
    }
}

#[repr(C)]
#[derive(Copy, Clone, Pod, Zeroable)]
struct PushConstants {
    color: [f32; 4],
    cursor: Vec2,
}

impl RenderComponent for Editor {
    fn initialize(&mut self, renderer: &mut Renderer) {

        // Initialize shader
        let bindings = [
            DescriptorSetLayoutBinding::default()
                .binding(0)
                .descriptor_count(1)
                .descriptor_type(DescriptorType::STORAGE_IMAGE)
                .stage_flags(ShaderStageFlags::COMPUTE),
            DescriptorSetLayoutBinding::default()
                .binding(1)
                .descriptor_count(1)
                .descriptor_type(DescriptorType::STORAGE_IMAGE)
                .stage_flags(ShaderStageFlags::COMPUTE),
            DescriptorSetLayoutBinding::default()
                .binding(2)
                .descriptor_count(1)
                .descriptor_type(DescriptorType::STORAGE_IMAGE)
                .stage_flags(ShaderStageFlags::COMPUTE)
        ];

        let layout = vec![DescriptorSetLayout::new_push_descriptor(
            &renderer.device,
            &bindings
        )];

        let push_constants = vec![PushConstantRange::default()
            .size(size_of::<PushConstants>() as u32)
            .stage_flags(ShaderStageFlags::COMPUTE)
            .offset(0)
        ];

        let macros: HashMap<String, String> = HashMap::new();
        self.pipeline = Some(renderer.pipeline_store().insert(PipelineConfig {
            shader_path: "shaders/brush.comp".parse().unwrap(),
            descriptor_set_layouts: layout,
            push_constant_ranges: push_constants,
            macros,
        }).unwrap());

        // Load image from disk
        let im = image::open("./black.png").expect("Couldn't load image").to_rgba8();
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

        let mut map = buf.mapped().unwrap();
        let pixel_data = im.as_bytes();
        unsafe { std::ptr::copy_nonoverlapping(pixel_data.as_ptr(), map.as_mut_slice().as_mut_ptr(), pixel_data.len()); }

        self.image = Some(Image::new(
            &renderer.device,
            &mut renderer.allocator,
            im.width(),
            im.height(),
            ImageUsageFlags::STORAGE | ImageUsageFlags::SAMPLED | ImageUsageFlags::TRANSFER_DST
        ));
        
        self.draw_buffer = Some(Image::new(
            &renderer.device,
            &mut renderer.allocator,
            im.width(),
            im.height(),
            ImageUsageFlags::STORAGE | ImageUsageFlags::SAMPLED | ImageUsageFlags::TRANSFER_DST | ImageUsageFlags::TRANSFER_SRC
        ));

        self.stencil_buffer = Some(Image::new(
            &renderer.device,
            &mut renderer.allocator,
            im.width(),
            im.height(),
            ImageUsageFlags::STORAGE | ImageUsageFlags::SAMPLED | ImageUsageFlags::TRANSFER_DST | ImageUsageFlags::TRANSFER_SRC
        ));

        let mut command_buffer = renderer.create_command_buffer();
        command_buffer.begin();

        renderer.transition_image(
            &command_buffer,
            self.draw_buffer.as_ref().unwrap().handle(),
            ImageLayout::UNDEFINED,
            ImageLayout::TRANSFER_DST_OPTIMAL,
            PipelineStageFlags::TRANSFER,
            PipelineStageFlags::TRANSFER,
            AccessFlags::TRANSFER_READ,
            AccessFlags::TRANSFER_WRITE,
        );

        renderer.transition_image(
            &command_buffer,
            self.stencil_buffer.as_ref().unwrap().handle(),
            ImageLayout::UNDEFINED,
            ImageLayout::GENERAL,
            PipelineStageFlags::TRANSFER,
            PipelineStageFlags::TRANSFER,
            AccessFlags::TRANSFER_READ,
            AccessFlags::TRANSFER_WRITE,
        );

        command_buffer.clear_color_image(
            self.stencil_buffer.as_ref().unwrap(),
            ImageLayout::GENERAL,
            [0.0, 0.0, 0.0, 1.0]
        );

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
        command_buffer.copy_buffer_to_image(
            &buf,
            self.draw_buffer.as_ref().unwrap(),
            ImageLayout::TRANSFER_DST_OPTIMAL,
            &regions
        );
        renderer.transition_image(
            &command_buffer,
            self.image.as_ref().unwrap().handle(),
            ImageLayout::TRANSFER_DST_OPTIMAL,
            ImageLayout::GENERAL,
            PipelineStageFlags::TRANSFER,
            PipelineStageFlags::TRANSFER,
            AccessFlags::TRANSFER_READ,
            AccessFlags::TRANSFER_WRITE,
        );
        renderer.transition_image(
            &command_buffer,
            self.draw_buffer.as_ref().unwrap().handle(),
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

        if self.tab_viewer.as_ref().unwrap().pointer_released {

            // Clear brush stencil

            // command_buffer.clear_color_image(
            //     self.stencil_buffer.as_ref().unwrap(),
            //     ImageLayout::GENERAL,
            //     [0.0, 0.0, 0.0, 1.0]
            // );

            // Copy the draw buffer into the image buffer

            renderer.transition_image(
                &command_buffer,
                self.draw_buffer.as_ref().unwrap().handle(),
                ImageLayout::SHADER_READ_ONLY_OPTIMAL,
                ImageLayout::TRANSFER_SRC_OPTIMAL,
                PipelineStageFlags::TOP_OF_PIPE,
                PipelineStageFlags::COMPUTE_SHADER,
                AccessFlags::NONE,
                AccessFlags::SHADER_WRITE,
            );

            let width = self.image.as_ref().unwrap().width;
            let height = self.image.as_ref().unwrap().height;
            let regions = [
                ImageCopy::default()
                    .src_offset(Offset3D { x: 0, y: 0, z: 0 })
                    .dst_offset(Offset3D { x: 0, y: 0, z: 0 })
                    .extent(vk::Extent3D { width, height, depth: 1 })
                    .src_subresource(ImageSubresourceLayers {
                        aspect_mask: ImageAspectFlags::COLOR,
                        mip_level: 0,
                        base_array_layer: 0,
                        layer_count: 1,
                    })
                    .dst_subresource(ImageSubresourceLayers {
                        aspect_mask: ImageAspectFlags::COLOR,
                        mip_level: 0,
                        base_array_layer: 0,
                        layer_count: 1,
                    })
            ];
            command_buffer.copy_image(
                self.draw_buffer.as_ref().unwrap(),
                ImageLayout::TRANSFER_SRC_OPTIMAL,
                self.image.as_ref().unwrap(),
                ImageLayout::GENERAL,
                &regions
            );

            renderer.transition_image(
                &command_buffer,
                self.draw_buffer.as_ref().unwrap().handle(),
                ImageLayout::TRANSFER_SRC_OPTIMAL,
                ImageLayout::SHADER_READ_ONLY_OPTIMAL,
                PipelineStageFlags::COMPUTE_SHADER,
                PipelineStageFlags::BOTTOM_OF_PIPE,
                AccessFlags::SHADER_WRITE,
                AccessFlags::NONE,
            );
        }

        if !self.tab_viewer.as_ref().unwrap().pointer_down || self.tab_viewer.as_ref().unwrap().space_down {
            return;
        }

        renderer.transition_image(
            &command_buffer,
            self.draw_buffer.as_ref().unwrap().handle(),
            ImageLayout::SHADER_READ_ONLY_OPTIMAL,
            ImageLayout::GENERAL,
            PipelineStageFlags::TOP_OF_PIPE,
            PipelineStageFlags::COMPUTE_SHADER,
            AccessFlags::NONE,
            AccessFlags::SHADER_WRITE,
        );

        let binding = renderer.pipeline_store().get(self.pipeline.unwrap());
        let pipeline = binding.as_ref().unwrap();
        command_buffer.bind_pipeline(pipeline);

        let rgb = self.tab_viewer.as_ref().unwrap().okhsl.to_srgb();
        let push_constants = PushConstants {
            cursor: self.tab_viewer.as_ref().unwrap().image_pointer,
            color: [rgb.r as f32 / 255.0, rgb.g as f32 / 255.0, rgb.b as f32 / 255.0, 1.0]
        };
        command_buffer.push_constants(pipeline, ShaderStageFlags::COMPUTE, 0, &bytemuck::cast_slice(std::slice::from_ref(&push_constants)));

        let bindings = [
            self.image.as_ref().unwrap().binding(vk::ImageLayout::GENERAL),
            self.draw_buffer.as_ref().unwrap().binding(vk::ImageLayout::GENERAL),
            self.stencil_buffer.as_ref().unwrap().binding(vk::ImageLayout::GENERAL)
        ];

        let write_descriptor_set = WriteDescriptorSet::default()
            .dst_binding(0)
            .dst_array_element(0)
            .descriptor_type(vk::DescriptorType::STORAGE_IMAGE)
            .image_info(&bindings);

        command_buffer.bind_push_descriptor(
            pipeline,
            0,
            &[write_descriptor_set]
        );
        command_buffer.dispatch(500, 500, 1 );

        renderer.transition_image(
            &command_buffer,
            self.draw_buffer.as_ref().unwrap().handle(),
            ImageLayout::GENERAL,
            ImageLayout::SHADER_READ_ONLY_OPTIMAL,
            PipelineStageFlags::COMPUTE_SHADER,
            PipelineStageFlags::BOTTOM_OF_PIPE,
            AccessFlags::SHADER_WRITE,
            AccessFlags::NONE,
        );
    }
}