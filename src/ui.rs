use std::fs::DirEntry;
use std::path::PathBuf;
use std::str::FromStr;
use std::sync::Arc;

use eframe::egui;
use eframe::egui::{Align, ColorImage, ImageData, ImageSource, Slider, SliderOrientation, TextureOptions};
use eframe::egui::load::Bytes;
use fraction::Fraction;
use image::DynamicImage;
use image::imageops::FilterType;

use crate::imports::directory_to_files;
use crate::process::{process_image_from_path, process_in_memory_image};
use crate::structs::Args;

pub fn run(settings: Args) {
    let native_options = eframe::NativeOptions::default();
    let _ = eframe::run_native("Batched Lazy Image Processing Binary", native_options, Box::new(|cc| Box::new(App::new(cc, settings))));
}

struct App {
    image_filter: FilterType,
    jpeg_quality: u32,
    max_width: u32,
    aspect_ratio: String,
    batch_size: usize,
    extensions: String,
    crop: bool,
    metadata: bool,
    resize: bool,
    preview: bool,
    input: String,
    output: String,
    files: Vec<std::io::Result<DirEntry>>,
    source_file_name: Option<String>,
    source_path: Option<PathBuf>,
    target_file_path: Option<String>,
    source_image: Option<DynamicImage>,
    target_image: Option<DynamicImage>,
}

impl App {
    fn new(cc: &eframe::CreationContext<'_>, settings: Args) -> Self {
        // Customize egui here with cc.egui_ctx.set_fonts and cc.egui_ctx.set_visuals.
        // Restore app state using cc.storage (requires the "persistence" feature).
        // Use the cc.gl (a glow::Context) to create graphics shaders and buffers that you can use
        // for e.g. egui::PaintCallback.
        egui_extras::install_image_loaders(&cc.egui_ctx);
        let input_directory = settings.input.as_str();
        let extensions: Vec<&str> = settings.extensions.split("|").collect();
        let files = directory_to_files(input_directory, &extensions);
        let file_name_and_path = if files.iter().count() > 0 {
            let file = files.get(0).unwrap();
            let path = file.as_ref().map(|f| { f.path() }).unwrap();
            let file_name = path.file_name().map(|s| s.to_os_string().into_string().unwrap());
            (file_name, Some(path))
        } else {
            (None, None)
        };
        let (source_file_name, source_path) = file_name_and_path;
        let this = App {
            image_filter: FilterType::Lanczos3,
            jpeg_quality: (settings.quality as u32),
            max_width: settings.max_width,
            aspect_ratio: settings.aspect_ratio.to_string(),
            batch_size: settings.batch_size,
            extensions: settings.extensions,
            crop: !settings.no_crop,
            metadata: !settings.no_metadata,
            resize: !settings.no_resize,
            preview: false,
            input: settings.input.clone(),
            output: settings.output,
            files,
            source_path,
            source_file_name,
            target_file_path: None,
            source_image: None,
            target_image: None,
        };
        this
    }
}

impl eframe::App for App {
    fn update(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            let frame_size = frame.info().window_info.size;
            let half_frame_width = (frame_size.x / 2.0);
            let checkbox_spacing = half_frame_width - 10.0;
            let slider_spacing = half_frame_width - 43.0;
            ui.style_mut().spacing.slider_width = slider_spacing;
            ui.vertical_centered(|ui| {
                egui::TopBottomPanel::top("top_panel")
                    .resizable(false)
                    .min_height(100.0)
                    .show_inside(ui, |ui| {
                        ui.vertical_centered(|ui| {
                            ui.heading("Settings");
                        });
                        ui.horizontal_top(|ui| {
                            ui.set_height(20.0);
                            let mut size = ui.available_size();
                            size.x = half_frame_width;
                            let text = egui::TextEdit::singleline(&mut self.input)
                                .horizontal_align(Align::Center);
                            ui.add_sized(size, text);
                            if ui.button("Input folder...").clicked() {
                                if let Some(path) = rfd::FileDialog::new()
                                    .set_directory(self.input.clone())
                                    .pick_folder() {
                                    self.input = path.display().to_string();
                                }
                            }
                        });
                        ui.separator();
                        ui.horizontal_top(|ui| {
                            ui.set_height(20.0);
                            let mut size = ui.available_size();
                            size.x = half_frame_width;
                            let text = egui::TextEdit::singleline(&mut self.output)
                                .horizontal_align(Align::Center);
                            ui.add_sized(size, text);
                            if ui.button("Output folder...").clicked() {
                                if let Some(path) = rfd::FileDialog::new()
                                    .set_directory(self.output.clone())
                                    .pick_folder() {
                                    self.output = path.display().to_string();
                                }
                            }
                        });
                        ui.separator();
                        ui.horizontal_top(|ui| {
                            ui.add_space(checkbox_spacing);
                            ui.checkbox(&mut self.crop, "Crop");
                        });
                        ui.separator();
                        ui.horizontal_top(|ui| {
                            ui.add_space(checkbox_spacing);
                            ui.checkbox(&mut self.metadata, "Metadata");
                        });
                        ui.separator();
                        ui.horizontal_top(|ui| {
                            ui.add_space(checkbox_spacing);
                            ui.checkbox(&mut self.resize, "Resize");
                        });
                        ui.separator();
                        ui.horizontal_top(|ui| {
                            ui.add_space(checkbox_spacing);
                            ui.checkbox(&mut self.preview, "Live Preview");
                        });
                        ui.separator();
                        ui.horizontal_top(|ui| {
                            ui.set_height(20.0);
                            let mut size = ui.available_size();
                            size.x = half_frame_width;
                            let text = egui::TextEdit::singleline(&mut self.aspect_ratio)
                                .horizontal_align(Align::Center);
                            ui.add_sized(size, text);
                            ui.label("Aspect Ratio");
                        });
                        ui.separator();
                        let image_filter = &self.image_filter.clone();
                        ui.horizontal_top(|ui| {
                            ui.add_space(half_frame_width - 95.0);
                            egui::ComboBox::from_label("Image Filter Type")
                                .selected_text(format!("{image_filter:?}"))
                                .show_ui(ui, |ui| {
                                    ui.style_mut().wrap = Some(false);
                                    ui.selectable_value(&mut self.image_filter, FilterType::Nearest, "Nearest Neighbor");
                                    ui.selectable_value(&mut self.image_filter, FilterType::Triangle, "Linear Filter");
                                    ui.selectable_value(&mut self.image_filter, FilterType::CatmullRom, "Cubic Filter");
                                    ui.selectable_value(&mut self.image_filter, FilterType::Gaussian, "Gaussian Filter");
                                    ui.selectable_value(&mut self.image_filter, FilterType::Lanczos3, "Lanczos with window 3");
                                });
                        });
                        ui.separator();
                        ui.add(Slider::new(&mut self.max_width, 1000u32..=2048u32)
                            .orientation(SliderOrientation::Horizontal)
                            .text("Maximum Width")
                            .trailing_fill(false)
                        );
                        ui.separator();
                        ui.add(Slider::new(&mut self.jpeg_quality, 0u32..=100u32)
                            .clamp_to_range(true)
                            .smart_aim(true)
                            .trailing_fill(true)
                            .orientation(SliderOrientation::Horizontal)
                            .text("JPEG Quality")
                        );
                        ui.add_space(2.0);
                    });

                ui.columns(2, |cols| {
                    if self.preview {
                        let args = Args {
                            aspect_ratio: Fraction::from_str(self.aspect_ratio.clone().as_str()).unwrap(),
                            batch_size: self.batch_size.clone(),
                            extensions: self.extensions.clone(),
                            input: self.input.clone(),
                            max_width: self.max_width,
                            no_crop: !self.crop,
                            no_metadata: !self.metadata,
                            no_resize: !self.resize,
                            output: self.output.clone(),
                            quality: self.jpeg_quality as u8,
                            ui: true,
                        };
                        if self.source_image.is_some() {
                            self.target_image = process_in_memory_image(self.source_image.clone(), args.clone());
                        };
                    }
                    for (i, col) in cols.iter_mut().enumerate() {
                        if i == 0 {
                            col.vertical_centered_justified(|col| {
                                if self.source_file_name.is_some() && self.source_path.is_some() {
                                    col.label(format!("Source Image: {}", self.source_file_name.clone().unwrap()));
                                    self.source_image = match image::open(self.source_path.clone().unwrap()) {
                                        Ok(image) => Some(image),
                                        Err(error) => None
                                    };
                                    render_dynamic_image("source", self.source_image.clone(), col);
                                } else {
                                    col.label("Source Image: <None>");
                                }
                            });
                        } else {
                            col.vertical_centered_justified(|col| {
                                if self.preview && self.target_image.is_some() {
                                    //let target = self.target_file_path.clone().unwrap();
                                    col.label(format!("Target Image: {}", self.source_file_name.clone().unwrap()));
                                    // col.label(format!("Quality: {}", self.jpeg_quality));
                                    render_dynamic_image("preview", self.target_image.clone(), col);
                                    match self.image_filter {
                                        FilterType::Nearest => {}
                                        FilterType::Triangle => {}
                                        FilterType::CatmullRom => {}
                                        FilterType::Gaussian => {}
                                        FilterType::Lanczos3 => {}
                                    }
                                } else {
                                    col.label("Target Image: <None>");
                                }
                            });
                        }
                    }
                });
            });
        });
    }
}


fn render_dynamic_image(name: &str, optional_image: Option<DynamicImage>, ui: &mut egui::Ui) {
    match optional_image {
        Some(dynamic_image) => {
            let size = [dynamic_image.width() as _, dynamic_image.height() as _];
            let image_buffer = dynamic_image.to_rgba8();
            let pixels = image_buffer.as_flat_samples();
            let color_image_ok: Result<ColorImage, image::ImageError> = Ok(ColorImage::from_rgba_unmultiplied(
                size,
                pixels.as_slice(),
            ));
            match color_image_ok {
                Ok(color_image) => {
                    let arc_color_image = Arc::new(color_image);
                    let handle = ui.ctx().load_texture(
                        "preview",
                        ImageData::Color(
                            arc_color_image
                        ),
                        TextureOptions::default(),
                    );
                    ui.vertical_centered_justified(|col| {
                        egui::ScrollArea::both().show(col, |col| {
                            col.image((handle.id(), handle.size_vec2()));
                        });
                    });
                }
                Err(error) => {
                    println!("image error:{}", error);
                    return;
                }
            };
        }
        None => ()
    }
}