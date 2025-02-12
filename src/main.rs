#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")] // hide console window on Windows in release
#![allow(rustdoc::missing_crate_level_docs)] // don't generate warning if there aren't comments

mod enums;
mod display_manager;
use crate::enums::StreamingState;
use eframe::egui::Rounding;
use eframe::egui::{
    self, Color32, ColorImage, KeyboardShortcut, ModifierNames, Modifiers, PointerButton, Pos2,
    Rect,
};
use local_ip_address::local_ip;
use display_manager::network::net::*;
use display_manager::screen::loop_logic;
use display_manager::screen::ScreenState;
use std::sync::Arc;

#[derive(PartialEq, Debug, Default)]
enum CastRecEnum {
    #[default]
    None,
    Caster,
    Receiver,
}

#[derive(Default)]
enum Pages {
    #[default]
    HOME,
    CASTER,
    SHORTCUT,
    RECEIVER,
}

#[derive(Default, PartialEq, Clone, Copy)]
enum Drawing {
    #[default]
    NONE,
    LINE,
    CIRCLE,
    TEXT,
}

fn main() -> eframe::Result<()> {
    env_logger::init(); // Log to stderr (if you run with `RUST_LOG=debug`).
    let options = eframe::NativeOptions {
        renderer: eframe::Renderer::Wgpu, //WGPU recent API for graphics
        ..Default::default()
    };

    eframe::run_native(
        "Streaming Application",
        options,
        Box::new(|_cc| Box::<MyApp>::default()),
    )
}

#[derive(Default)]
struct MyApp {
    current_page: Pages,
    texture: Option<egui::TextureHandle>,
    screenshot: Option<ColorImage>,
    temp_shortcut: Option<KeyboardShortcut>,
    start_shortcut: Option<KeyboardShortcut>,
    pause_shortcut: Option<KeyboardShortcut>,
    blank_shortcut: Option<KeyboardShortcut>,
    stop_shortcut: Option<KeyboardShortcut>,
    my_enum: CastRecEnum,
    host_server_address: String,
    state: Arc<ScreenState>,
    flag_thread: bool,
    x: String,
    y: String,
    f: String,
    x_value: u32,
    y_value: u32,
    reduction_value: u32,
    annotations_enabled: bool,
    line_annotations: Vec<(egui::Pos2, egui::Pos2)>,
    circle_annotations: Vec<(egui::Pos2, egui::Pos2)>,
    text_annotation: Vec<(egui::Pos2, String)>,
    is_drawing: bool,
    drawings: Drawing,
    last_pressed_shortcut: Option<egui::KeyboardShortcut>,
    active_shortcut_field: Option<String>,
    temp_shortcut_display: String, 
    img_rect: Option<Rect>,
    annotation_color: Color32,
}

impl MyApp {
    
    fn normalize_position(pos: Pos2, rect: Rect) -> Pos2 {
        Pos2 {
            x: (pos.x - rect.min.x) / rect.width(),
            y: (pos.y - rect.min.y) / rect.height(),
        }
    }
    
    fn handle_mouse_input(&mut self, ui: &egui::Ui) {
        if let Some(rect) = self.img_rect {
            let response = ui.interact(rect, ui.id(), egui::Sense::drag());
            let hover_pos = response.hover_pos();
            
            // Start drawing on click
            if response.drag_started() {
                self.is_drawing = true;
                if let Some(pos) = hover_pos {
                    let normalized_pos = Self::normalize_position(pos, rect);
                    match self.drawings {
                        Drawing::LINE => {
                            self.line_annotations.push((normalized_pos, normalized_pos));
                        }
                        Drawing::CIRCLE => {
                            self.circle_annotations.push((normalized_pos, normalized_pos));
                        }
                        _ => {}
                    }
                }
            }
            
            // Update while dragging
            if response.dragged() && self.is_drawing {
                if let Some(pos) = hover_pos {
                    let normalized_pos = Self::normalize_position(pos, rect);
                    match self.drawings {
                        Drawing::LINE => {
                            if let Some(last) = self.line_annotations.last_mut() {
                                last.1 = normalized_pos;
                            }
                        }
                        Drawing::CIRCLE => {
                            if let Some(last) = self.circle_annotations.last_mut() {
                                last.1 = normalized_pos;
                            }
                        }
                        _ => {}
                    }
                }
            }
            
            // Stop drawing
            if response.drag_released() {
                self.is_drawing = false;
            }
        }
    }


    fn handle_text_input(&mut self, ui: &egui::Ui) {
        ui.input(|i| {
            for event in &i.raw.events {
                if let Some(rect) = self.img_rect {
                    let min = rect.min;
                    let max = rect.max;
                    let w = max.x - min.x;
                    let h = max.y - min.y;
    
                    match event {
                        egui::Event::PointerButton {
                            pos,
                            button: PointerButton::Primary,
                            pressed: true,
                            ..
                        } => {
                            let x = (pos.x - min.x) / w;
                            let y = (pos.y - min.y) / h;
                            self.text_annotation.push((Pos2 { x, y }, String::new()));
                        },
                        egui::Event::Text(text) => {
                            if let Some(last_ann) = self.text_annotation.last_mut() {
                                last_ann.1.push_str(text);
                            }
                        },
                        _ => {}
                    }
                }
            }
        });
    }


    fn start_cast_function(&mut self) {
        if !self.flag_thread {
            let my_local_ip = local_ip().unwrap();
            self.state.set_ip_rec(my_local_ip.to_string() + ":8080");

            let host_server = Server::new(my_local_ip.to_string() + ":8080");
            let state_clone1 = self.state.clone();
            let _ = host_server.bind_to_ip(state_clone1);

            self.state.set_host_server(Some(host_server));

            self.state.set_screen_state(StreamingState::START);
            self.flag_thread = true; //Avoid to initialize more instances

            let state_clone = self.state.clone();
            std::thread::spawn(move || {
                let _ = loop_logic("caster".to_string(), state_clone);
            });
        } else {
            self.state.set_screen_state(StreamingState::START);
            self.state.cv.notify_all();
        }
    }

    pub fn start_rec_function(&mut self) {
        self.state.drop_client();
    
        let client = Client::new(self.host_server_address.clone());
        match client.connect_to_ip() {
            Ok(stream) => {
                self.state.set_client(Some((stream, client)));
                self.state.set_ip_send(self.host_server_address.clone());
                let state_clone = self.state.clone();
                self.current_page = Pages::RECEIVER;
                
                if !self.flag_thread {
                    self.state.set_screen_state(StreamingState::START);
                    self.flag_thread = true;
                    std::thread::spawn(move || {
                        println!("Starting receiver loop logic");
                        let _ = loop_logic("receiver".to_string(), state_clone);
                    });
                } else {
                    self.state.set_screen_state(StreamingState::START);
                    self.state.cv.notify_all();
                }
            }
            Err(e) => {
                println!("Connection failed: {}", e);
                self.current_page = Pages::HOME;
            }
        }
    }

    // capture actual frame on the screen and convert it in a viewable format
    fn screenshot(&mut self) -> ColorImage {
        let st = self.state.clone();

        let img = st.get_frame();

        let (width, height) = img.dimensions();
        let pixels = img.into_raw();

        ColorImage::from_rgba_unmultiplied([width as usize, height as usize], &pixels)
    }

    fn take_SHORTCUT_icon(&self, path: &str) -> ColorImage {
        let img = image::open(path).expect("Image does not exist");

        let img_buf = img.into_rgba8();
        let (height, width) = img_buf.dimensions();
        let pixels = img_buf.into_raw();

        ColorImage::from_rgba_unmultiplied([width as usize, height as usize], &pixels)
    }

    // textual representation of Ctrl, Alt, Shift for shortcut
    fn get_mod_simbol(&self, modifier: Modifiers) -> String {
        let modnames = ModifierNames::NAMES;
        let modnamesformat = modnames.format(&modifier, false);
        modnamesformat
    }
}

impl eframe::App for MyApp {
    fn update(&mut self, ctx: &egui::Context, _eframe: &mut eframe::Frame) {
        // setup_custom_fonts(&ctx);

        let frame = egui::Frame::default()
        .fill(Color32::from_rgb(230, 230, 250))  // Light purple background
        .inner_margin(10.0)
        .outer_margin(0.0)
        .rounding(0.0);

        egui::CentralPanel::default()
        .frame(frame)
        .show(ctx, |ui| {
            if self.state.get_sc_state() == StreamingState::STOP {
                self.annotation_color = Color32::from_rgb(255, 0, 0)
            }
            let mut shapes: Vec<egui::Shape> = Vec::new();

            // let button_width = ui.available_width() / 5.0;
            let button_height = ui.available_height() / 8.0;
            if let Some(screenshot) = &self.screenshot {
                self.texture = Some(ui.ctx().load_texture(
                    "screenshot",
                    screenshot.clone(),
                    Default::default(),
                ));
            } else {
                self.texture = None;
            }
            let logo = Some(ui.ctx().load_texture(
                "logo",
                self.take_SHORTCUT_icon("logo.png").clone(),
                Default::default(),
            ))
            .unwrap();

            let rec_img = Some(ui.ctx().load_texture(
                "rec_img",
                self.take_SHORTCUT_icon("rec.png").clone(),
                Default::default(),
            ))
            .unwrap();

            let stop_img = Some(ui.ctx().load_texture(
                "stop_img",
                self.take_SHORTCUT_icon("stop_rec.png").clone(),
                Default::default(),
            ))
            .unwrap();

            match self.current_page {
                Pages::HOME => {
                    // Top logo section
                    ui.vertical_centered(|ui| {
                        // Logo button with proper sizing and alignment
                        let original_size = logo.size_vec2();
                        let x_size = original_size.x / 2.0;
                        let y_size = original_size.y / 2.0;
                        let logo_button = egui::ImageButton::new((
                            logo.id(),
                            egui::vec2(x_size, y_size),
                        ));
                        
                        if ui.add(logo_button).clicked() {
                            // Handle back button click
                            self.current_page = Pages::HOME;
                            self.my_enum = CastRecEnum::None;
                        }
                        
                        // Add visible spacing after button
                        ui.add_space(50.0);
                
                
                    // Dynamic header text
                    let header_text = if self.my_enum == CastRecEnum::None {
                        String::from("Choose a mode")
                    } else {
                        format!("Selected mode: {:?}", self.my_enum)
                    };

                    ui.heading(
                        egui::RichText::new(header_text)
                            .color(match self.my_enum {
                                CastRecEnum::None => Color32::DARK_GREEN,
                                CastRecEnum::Caster => Color32::DARK_BLUE,
                                CastRecEnum::Receiver => Color32::DARK_RED,
                            })
                            .size(18.0)
                    );
                    ui.add_space(20.0);
                });
                    // Two-row layout
                    ui.vertical_centered(|ui| {
                        // Row 1 - Mode buttons side by side
                        ui.horizontal(|ui| {
                            ui.add_space(ui.available_width() / 4.0); // Center buttons
                            
                            let cast_button = egui::Button::new(
                                egui::RichText::new("Caster")
                                    .size(18.0)
                                    .text_style(egui::TextStyle::Heading)
                            )
                            .min_size(egui::vec2(120.0, button_height))
                            .rounding(Rounding::same(20.0))
                            .fill(if self.my_enum == CastRecEnum::Caster {
                                Color32::from_rgb(0xAD, 0xD8, 0xE6)
                            } else {
                                Color32::LIGHT_BLUE
                            });
                            
                            if ui.add(cast_button).clicked() {
                                self.my_enum = CastRecEnum::Caster;
                            }

                            ui.add_space(20.0);

                            let rec_button = egui::Button::new(
                            egui::RichText::new("Receiver")
                                    .size(18.0)
                                    .text_style(egui::TextStyle::Heading)
                            )
                                .min_size(egui::vec2(120.0, button_height))
                                .rounding(Rounding::same(20.0))  
                                .fill(if self.my_enum == CastRecEnum::Receiver {
                                    Color32::from_rgb(255, 255, 0xE0)
                                } else {
                                    Color32::LIGHT_YELLOW
                                });
                            if ui.add(rec_button).clicked() {
                                self.my_enum = CastRecEnum::Receiver;
                            }
                        });

                        ui.add_space(30.0);

                        // Row 2 - Dynamic content based on selection
                        match self.my_enum {
                            CastRecEnum::None => {
                                ui.centered_and_justified(|ui| {
                                    ui.label("");
                                });
                            }
                            CastRecEnum::Caster => {
                                ui.vertical_centered(|ui| {
                                    // Display IP label
                                    let my_local_ip = local_ip().unwrap();
                                    ui.label(
                                        egui::RichText::new(format!("You're connected at IP: {}", my_local_ip.to_string() + " at port 8080"))
                                            .size(16.0)
                                            .color(Color32::BLACK)
                                    );
                                    
                                    ui.add_space(20.0);
                        
                                    // Centered start button
                                    let share_button = egui::Button::new(
                                        egui::RichText::new("Start Sharing")
                                            .size(14.0)
                                            .text_style(egui::TextStyle::Heading)
                                    )
                                    .min_size(egui::vec2(100.0, button_height))
                                    .rounding(Rounding::same(20.0))
                                    .fill(Color32::from_rgb(136,252,201));
                                    
                                    if ui.add(share_button).clicked() {
                                        self.current_page = Pages::CASTER;
                                    }
                                });
                            }
                            CastRecEnum::Receiver => {
                                ui.vertical_centered(|ui| {
                                    // IP input field
                                    ui.label(
                                        egui::RichText::new("IP Server:")
                                            .size(16.0)
                                            .text_style(egui::TextStyle::Heading)
                                    );
                                    
                                    // Display IP without port but keep internal value
                                    let mut display_ip = self.host_server_address.split(':').next().unwrap_or("").to_string();
                                    ui.text_edit_singleline(&mut display_ip);
                                    
                                    self.host_server_address = display_ip;

                                    ui.add_space(20.0);

                                    // Create temporary IP with port
                                    let is_ip_valid = !self.host_server_address.is_empty();
                                    // Centered start button
                                    let view_button = egui::Button::new(
                                        egui::RichText::new("Start Connection")
                                            .size(14.0)
                                            .text_style(egui::TextStyle::Heading)
                                    )
                                    .min_size(egui::vec2(100.0, button_height))
                                    .rounding(Rounding::same(20.0))
                                    .fill(if is_ip_valid {
                                        Color32::from_rgb(136,252,201)
                                    } else {
                                        Color32::GRAY
                                    });
                                    
                                    if ui.add(view_button).clicked() && is_ip_valid {
                                        self.host_server_address = format!("{}:8080", self.host_server_address);
                                        self.start_rec_function();
                                    }
                                });
                            }
                        }
                    });
                }

             
                Pages::CASTER => {
                    // Top section with back button and logo
                    ui.horizontal(|ui| {
                        let original_size = logo.size_vec2();
                        let x_size = original_size.x / 4.0;
                        let y_size = original_size.y / 4.0;
                        let logo_button = egui::ImageButton::new((
                            logo.id(),
                            egui::vec2(x_size, y_size),
                        ));
                        
                        if ui.add(logo_button).clicked() {
                            // Handle back button click
                            self.state.set_screen_state(StreamingState::STOP);
                            self.screenshot = None;
                            self.flag_thread = false;
                            self.current_page = Pages::HOME;
                            self.my_enum = CastRecEnum::None;
                        }
                    });
                   
                    // Row 1: Screen preview and controls
                    ui.horizontal(|ui| {
                        // Left side - Screen preview
                        ui.vertical(|ui| {
                            let frame_size = egui::vec2(ui.available_width() * 0.6, 250.0);
                            
                            if let Some(texture) = self.texture.as_ref() {
                                // Show actual stream when available
                                self.img_rect = Some(
                                    ui.image((texture.id(), frame_size)).rect
                                );
                            } else {
                                // Show placeholder white rectangle
                                let (rect, _response) = ui.allocate_exact_size(
                                    frame_size,
                                    egui::Sense::hover()
                                );
                                ui.painter().rect_filled(
                                    rect,
                                    0.0,
                                    Color32::WHITE
                                );
                                self.img_rect = Some(rect);
                            }
                        });
                
                        // Right side - Control sliders
                        ui.group(|ui| {
                            ui.vertical(|ui| {
                                ui.vertical(|ui|{
                                // X coordinate
                                ui.label("X coordinate");
                                ui.add_space(2.0);
                                let x_response = ui.add_sized(
                                    [120.0, 20.0],
                                    egui::Slider::new(&mut self.x_value, 0..=1998)
                                        .step_by(2.0)
                                        .text("px")
                                );
                                if x_response.changed() {
                                    self.state.set_x(self.x_value);
                                    self.x = self.x_value.to_string();
                                }
                                ui.add_space(5.0);
                                
                                // Y coordinate
                                ui.label("Y coordinate");
                                ui.add_space(2.0);
                                let y_response = ui.add_sized(
                                    [120.0, 20.0],
                                    egui::Slider::new(&mut self.y_value, 0..=998)
                                        .step_by(2.0)
                                        .text("px")
                                );
                                if y_response.changed() {
                                    self.state.set_y(self.y_value);
                                    self.y = self.y_value.to_string();
                                }
                                ui.add_space(5.0);
                                

                                
                                ui.label("Screen reduction");
                                let prev_value = self.reduction_value;
                                ui.add_space(2.0);
                                let reduction_response = ui.add_sized(
                                    [120.0, 20.0],
                                    egui::Slider::new(&mut self.reduction_value, 1..=100)
                                        .suffix("%")
                                        .clamp_to_range(true)
                                );
                                if reduction_response.changed() {
                                    let new_width = (1920.0 * (self.reduction_value as f32 / 100.0)) as u32;
                                    let new_height = (1080.0 * (self.reduction_value as f32 / 100.0)) as u32;
                                    
                                    if new_width < 16 || new_height < 16 || new_width % 2 != 0 || new_height % 2 != 0 {
                                        self.reduction_value = prev_value;
                                    } else {
                                        self.state.set_f(self.reduction_value);
                                        self.f = self.reduction_value.to_string();
                                    }
                                }
                                });
                                ui.separator();
                                ui.add_space(10.0);

                                    if let Some(texture) = self.texture.as_ref() {
                                        ui.vertical(|ui| {
                                            // Right side - Tools panel
                                            ui.horizontal(|ui| {
                                                ui.heading("Annotations");
                                                ui.add_space(5.0);
                                                ui.checkbox(&mut self.annotations_enabled, "Enable");
                                                
                                            });
                                                
                                            if self.annotations_enabled {
                                                ui.add_space(5.0);
                                                
                                                // Color picker
                                                ui.horizontal(|ui| {
                                                    ui.label("Color:");
                                                    ui.color_edit_button_srgba(&mut self.annotation_color);
                                                });
                                                self.state.set_overlay_color(self.annotation_color.to_srgba_unmultiplied());
                                                ui.add_space(10.0);
                                
                                                // First row: Line and Circle
                                                ui.horizontal(|ui| {
                                                    let button_size = egui::vec2(80.0, 30.0);
                                                    
                                                    // Line button
                                                    if ui.add(egui::Button::new("Line")
                                                        .min_size(button_size)
                                                        .fill(if self.drawings == Drawing::LINE {
                                                            Color32::LIGHT_GREEN
                                                        } else {
                                                            Color32::GRAY
                                                        }))
                                                        .clicked() 
                                                    {
                                                        self.drawings = if self.drawings == Drawing::LINE {
                                                            Drawing::NONE
                                                        } else {
                                                            Drawing::LINE
                                                        };
                                                    }
                                                    ui.add_space(5.0);

                                                    // Circle button
                                                    if ui.add(egui::Button::new("Circle")
                                                        .min_size(button_size)
                                                        .fill(if self.drawings == Drawing::CIRCLE {
                                                            Color32::LIGHT_GREEN
                                                        } else {
                                                            Color32::GRAY
                                                        }))
                                                        .clicked() 
                                                    {
                                                        self.drawings = if self.drawings == Drawing::CIRCLE {
                                                            Drawing::NONE
                                                        } else {
                                                            Drawing::CIRCLE
                                                        };
                                                    }
                                                });

                                                ui.add_space(5.0);

                                                // Second row: Text and Clear
                                                ui.horizontal(|ui| {
                                                    let button_size = egui::vec2(80.0, 30.0);
                                                    
                                                    // Text button
                                                    if ui.add(egui::Button::new("Text")
                                                        .min_size(button_size)
                                                        .fill(if self.drawings == Drawing::TEXT {
                                                            Color32::LIGHT_GREEN
                                                        } else {
                                                            Color32::GRAY
                                                        }))
                                                        .clicked() 
                                                    {
                                                        self.drawings = if self.drawings == Drawing::TEXT {
                                                            Drawing::NONE
                                                        } else {
                                                            Drawing::TEXT
                                                        };
                                                    }
                                                    ui.add_space(5.0);

                                                    // Clear button
                                                    if ui.add(egui::Button::new("Clear All")
                                                        .min_size(button_size)
                                                        .fill(Color32::LIGHT_RED))
                                                        .clicked() 
                                                    {
                                                        self.line_annotations.clear();
                                                        self.circle_annotations.clear();
                                                        self.text_annotation.clear();
                                                    }
                                                });
                                            }
                                    });
                            
                                    // Render existing annotations
                                    if let Some(rect) = self.img_rect {
                                        let min = rect.min;
                                        let max = rect.max;
                                        let w = max.x - min.x;
                                        let h = max.y - min.y;
                            
                                        // Update annotations in state
                                        self.state.set_line_ann(
                                            self.line_annotations
                                                .iter()
                                                .map(|(p1, p2)| (p1.x, p1.y, p2.x, p2.y))
                                                .collect(),
                                        );
                                        self.state.set_circle_ann(
                                            self.circle_annotations
                                                .iter()
                                                .map(|(p1, p2)| (p1.x, p1.y, p2.x, p2.y))
                                                .collect(),
                                        );
                                        self.state.set_text_ann(
                                            self.text_annotation
                                                .iter()
                                                .map(|(p1, t)| (p1.x, p1.y, t.to_owned()))
                                                .collect(),
                                        );
                            
                                        // Draw lines
                                        for &(start, end) in &self.line_annotations {
                                            let x1 = start.x * w + min.x;
                                            let y1 = start.y * h + min.y;
                                            let x2 = end.x * w + min.x;
                                            let y2 = end.y * h + min.y;
                                            shapes.push(egui::Shape::line_segment(
                                                [Pos2 { x: x1, y: y1 }, Pos2 { x: x2, y: y2 }],
                                                egui::Stroke::new(2.0, self.annotation_color),
                                            ));
                                        }
                            
                                        // Draw circles
                                        for &(start, end) in &self.circle_annotations {
                                            let x1 = start.x * w + min.x;
                                            let y1 = start.y * h + min.y;
                                            let x2 = end.x * w + min.x;
                                            let y2 = end.y * h + min.y;
                                            shapes.push(egui::Shape::circle_stroke(
                                                Pos2 { x: x1, y: y1 },
                                                ((x1 - x2).powi(2) + (y1 - y2).powi(2)).sqrt(),
                                                egui::Stroke::new(2.0, self.annotation_color),
                                            ));
                                        }
                            
                                        // Draw text
                                        for (start, text) in &mut self.text_annotation {
                                            let x1 = start.x * w + min.x;
                                            let y1 = start.y * h + min.y;
                                            ui.fonts(|f| {
                                                shapes.push(egui::Shape::text(
                                                    f,
                                                    Pos2 { x: x1, y: y1 },
                                                    egui::Align2::CENTER_CENTER,
                                                    text,
                                                    egui::FontId::proportional(15.0),
                                                    self.annotation_color,
                                                ));
                                            });
                                        }
                            
                                        // Paint all shapes
                                        let painter = ui.painter_at(rect);
                                        if self.annotations_enabled {  // Mostra le annotazioni solo se enabled
                                            painter.extend(shapes);
                                        }
                                        
                                    }
                                } 
                                
                                match self.drawings {
                                    Drawing::NONE => {}
                                    Drawing::LINE | Drawing::CIRCLE => {
                                        self.handle_mouse_input(ui);
                                    }
                                    Drawing::TEXT => {
                                        self.handle_text_input(ui);
                                    }
                                }
                            });
                        });

                    });
                    
                    ui.add_space(20.0);

                    if self.state.get_sc_state() == StreamingState::START {
                        self.screenshot = Some(self.screenshot());
                    }
                    
                    // Add IP display
                    ui.vertical_centered(|ui| {
                        ui.horizontal(|ui|{
                        let my_local_ip = local_ip().unwrap();
                        ui.label(
                            egui::RichText::new(format!("Your IP: {}", my_local_ip.to_string() + " at port 8080"))
                                .size(16.0)
                                .color(Color32::BLACK)
                        );
                        });
                    });

                    ui.add_space(10.0);
                
                    // Row 2: Control buttons and annotations
                    //HANDLE BUTTONS
                    ui.horizontal(|ui| {
                        let button_size = egui::vec2(80.0, 30.0);
                        let start_button = egui::Button::new(match self.state.get_sc_state() {
                            StreamingState::START => "Start",
                            StreamingState::PAUSE => "Resume",
                            StreamingState::BLANK => "Resume",
                            StreamingState::STOP => "Start",
                        })
                        .min_size(button_size)
                        .rounding(Rounding::same(20.0))
                        .fill(
                            if self.state.get_sc_state() == StreamingState::START {
                                Color32::LIGHT_GREEN
                            } else {
                                Color32::GRAY
                            },
                        );
                        if ui.add(start_button).clicked() {
                            self.start_cast_function();
                        }

                        let pause_button = egui::Button::new("Pause")
                            .min_size(button_size)
                            .rounding(Rounding::same(20.0))
                            .fill(if self.state.get_sc_state() == StreamingState::PAUSE {
                                Color32::LIGHT_GRAY
                            } else {
                                Color32::GRAY
                            });
                        if ui.add(pause_button).clicked() {
                            self.state.set_screen_state(StreamingState::PAUSE);
                            self.state.cv.notify_all();
                        }

                        let blank_button = egui::Button::new("Blank")
                            .min_size(button_size)
                            .rounding(Rounding::same(20.0))
                            .fill(if self.state.get_sc_state() == StreamingState::BLANK {
                                Color32::LIGHT_BLUE
                            } else {
                                Color32::GRAY
                            });
                       
                            if ui.add(blank_button).clicked() {
                                self.state.set_screen_state(StreamingState::BLANK);
                                self.state.cv.notify_all();
                            }


                        let stop_button = egui::Button::new("Stop")
                            .min_size(button_size)
                            .rounding(Rounding::same(20.0))
                            .fill(
                                if self.state.get_sc_state() == StreamingState::STOP {
                                    Color32::LIGHT_RED
                                } else {
                                    Color32::GRAY
                                },
                            );
                        if ui.add(stop_button).clicked() {
                            self.state.set_screen_state(StreamingState::STOP);
                            self.state.set_kill_connection_listener(true);
                            self.screenshot = None;
                            self.flag_thread = false;
                            self.current_page = Pages::HOME;
                            self.state.cv.notify_all();
                        }

                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            let shortcut_button = egui::Button::new(
                                egui::RichText::new("Shortcuts")
                                    .size(16.0)
                            )
                            .min_size(button_size)
                            .fill(Color32::GOLD)
                            .rounding(Rounding::same(20.0));
                            
                            if ui.add(shortcut_button).clicked() {
                                self.current_page = Pages::SHORTCUT;
                            }
                        });
                    });

                    match self.drawings {
                        Drawing::NONE => {}
                        Drawing::LINE => {
                            self.handle_mouse_input(ui);
                        }
                        Drawing::CIRCLE => {
                            self.handle_mouse_input(ui);
                        }
                        Drawing::TEXT => {
                            self.handle_text_input(ui);
                        }
                    }
                    //HANDLE SHORTCUTS
                    ui.input(|i| {
                        for event in &i.raw.events {
                            if let egui::Event::Key {
                                key,
                                pressed,
                                modifiers,
                                ..
                            } = event
                            {
                                if *pressed {
                                    self.temp_shortcut = Some(egui::KeyboardShortcut::new(
                                        modifiers.clone(),
                                        key.clone(),
                                    ));
                                }
                            }
                        }
                        //check inserted shorcut
                        if let Some(sct) = self.temp_shortcut {
                            if let Some(sc) = self.start_shortcut {
                                if sct == sc {
                                    self.start_cast_function();
                                    self.temp_shortcut = None;
                                }
                            }
                            if let Some(sc) = self.pause_shortcut {
                                if sct == sc {
                                    self.state.set_screen_state(StreamingState::PAUSE);
                                    self.temp_shortcut = None;
                                }
                            }
                            if let Some(sc) = self.blank_shortcut {
                                if sct == sc {
                                    self.state.set_screen_state(StreamingState::BLANK);
                                    self.temp_shortcut = None;
                                }
                            }
                            if let Some(sc) = self.stop_shortcut {
                                if sct == sc {
                                    self.state.set_screen_state(StreamingState::STOP);
                                    self.current_page = Pages::HOME;
                                    self.temp_shortcut = None;
                                }
                            }
                        }

                        self.temp_shortcut = None;
                    });
                    ctx.request_repaint();
                }

                Pages::RECEIVER => {
                    ui.horizontal(|ui| {
                        // Left side - logo and IP text
                        ui.vertical(|ui| {
                            let original_size = logo.size_vec2();
                            let x_size = original_size.x / 4.0;
                            let y_size = original_size.y / 4.0;
                            let logo_button = egui::ImageButton::new((
                                logo.id(),
                                egui::vec2(x_size, y_size),
                            ));
                            
                            if ui.add(logo_button).clicked() {
                                // Handle back button click
                                self.state.set_screen_state(StreamingState::STOP);
                                self.screenshot = None;
                                self.flag_thread = false;
                                self.current_page = Pages::HOME;
                            }
                            ui.label(
                                egui::RichText::new(format!("You're connected with IP: {}", self.state.get_ip_send()))
                                    .size(16.0)
                                    .color(Color32::BLACK)
                            );
                        });
                    
                        // Push buttons to the right and align vertically center
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {

                            let shortcut_button = egui::Button::new(
                                egui::RichText::new("Shortcuts")
                                    .size(16.0)
                            )
                            .min_size(egui::vec2(80.0, 30.0))
                            .fill(Color32::GOLD)
                            .rounding(Rounding::same(20.0));
                            if ui.add(shortcut_button).clicked() {
                                self.current_page = Pages::SHORTCUT;
                            }
                    
                            ui.add_space(10.0);
                    
                            let exit_button = egui::Button::new("Exit")
                                .min_size(egui::vec2(60.0, 30.0))
                                .rounding(Rounding::same(20.0));
                            if ui.add(exit_button).clicked() {
                                self.state.set_screen_state(StreamingState::STOP);
                                self.screenshot = None;
                                self.flag_thread = false;
                                self.current_page = Pages::HOME;
                            }
                    
                            ui.add_space(5.0);
                    
                            // Vertical container for rec button + label
                            ui.vertical_centered(|ui| {
                                ui.add_space(30.0);
                                // Small label over button
                                ui.label(
                                    egui::RichText::new(
                                        if self.state.get_rec().unwrap_or(false) {
                                            "click to stop"
                                        } else {
                                            "click to record"
                                        }
                                    )
                                    .size(12.0)
                                    .color(Color32::GRAY)
                                );

                                let rec_button = egui::ImageButton::new(
                                    if self.state.get_rec().unwrap_or(false) {
                                        (stop_img.id(), egui::vec2(button_height / 1.5, button_height / 1.5))
                                    } else {
                                        (rec_img.id(), egui::vec2(button_height / 1.5, button_height / 1.5))
                                    }
                                )
                                .rounding(5.0);
                                if ui.add(rec_button).clicked() {
                                    if let Some(rec) = self.state.get_rec() {
                                        self.state.set_rec(Some(!rec));
                                    } else {
                                        self.state.set_rec(Some(true));
                                    }
                                }
                    
                            });
                        });
                    });

                    // Video display
                    if let Some(texture) = self.texture.as_ref() {
                        self.img_rect = Some(ui.image((texture.id(), ui.available_size())).rect);
                    } else {
                        ui.with_layout(
                            egui::Layout::centered_and_justified(egui::Direction::BottomUp),
                            |ui| {
                                ui.spinner();
                            },
                        );
                    }


                    if let Some(texture) = self.texture.as_ref() {
                        if let Some(rect) = self.img_rect {
                            let min = rect.min;
                            let max = rect.max;
                            let w = max.x - min.x;
                            let h = max.y - min.y;

                            if let Some(color) = self.state.get_overlay_color() {
                                let ann_color = Color32::from_rgba_unmultiplied(
                                    color[0], color[1], color[2], color[3],
                                );

                                if let Some(ann) = self.state.get_line_ann() {
                                    for &(x1, y1, x2, y2) in &ann {
                                        let x1 = x1 * w + min.x;
                                        let y1 = y1 * h + min.y;
                                        let x2 = x2 * w + min.x;
                                        let y2 = y2 * h + min.y;
                                        shapes.push(egui::Shape::line_segment(
                                            [Pos2 { x: x1, y: y1 }, Pos2 { x: x2, y: y2 }],
                                            egui::Stroke::new(2.0, ann_color),
                                        ));
                                    }
                                }

                                if let Some(ann) = self.state.get_circle_ann() {
                                    for &(x1, y1, x2, y2) in &ann {
                                        let x1 = x1 * w + min.x;
                                        let y1 = y1 * h + min.y;
                                        let x2 = x2 * w + min.x;
                                        let y2 = y2 * h + min.y;

                                        shapes.push(egui::Shape::circle_stroke(
                                            Pos2 { x: x1, y: y1 },
                                            ((x1 - x2).powi(2) + (y1 - y2).powi(2)).sqrt(),
                                            egui::Stroke::new(2.0, ann_color),
                                        ));
                                    }
                                }

                                if let Some(ann) = self.state.get_text_ann() {
                                    for (x1, y1, text) in &ann {
                                        let x1 = x1 * w + min.x;
                                        let y1 = y1 * h + min.y;
                                        ui.fonts(|f| {
                                            let t = egui::Shape::text(
                                                f,
                                                Pos2 { x: x1, y: y1 },
                                                egui::Align2::CENTER_CENTER,
                                                text,
                                                egui::FontId::proportional(15.0),
                                                ann_color,
                                            );
                                            shapes.push(t);
                                        });
                                    }
                                }
                            }
                        }
                        self.img_rect = Some(ui.image((texture.id(), ui.available_size())).rect);

                        if let Some(rect) = self.img_rect {
                            let painter = ui.painter_at(rect);
                            painter.extend(shapes);
                        }
                    } else {
                        ui.with_layout(
                            egui::Layout::centered_and_justified(egui::Direction::BottomUp),
                            |ui| {
                                ui.spinner();
                            },
                        );
                    }

                    if self.state.get_sc_state() == StreamingState::STOP {
                        self.screenshot = None;
                        self.flag_thread = false;
                        self.current_page = Pages::HOME;
                    } else {
                        self.screenshot = Some(self.screenshot());
                    }

                    //HANDLE SHORTCUTS
                    ui.input(|i| {
                        for event in &i.raw.events {
                            if let egui::Event::Key {
                                key,
                                pressed,
                                modifiers,
                                ..
                            } = event
                            {
                                if *pressed {
                                    self.temp_shortcut = Some(egui::KeyboardShortcut::new(
                                        modifiers.clone(),
                                        key.clone(),
                                    ));
                                }
                            }
                        }
                        //check inserted shorcut
                        if let Some(sct) = self.temp_shortcut {
                            if let Some(sc) = self.stop_shortcut {
                                if sct == sc {
                                    self.state.set_screen_state(StreamingState::STOP);
                                    self.current_page = Pages::HOME;
                                    self.temp_shortcut = None;
                                }
                            }
                        }

                        self.temp_shortcut = None;
                    });

                    ctx.request_repaint();
                }

                Pages::SHORTCUT => {
                    ui.vertical_centered(|ui| {
                        // Exit button and title
                        ui.horizontal(|ui| {
                            if ui.add(
                                egui::Button::new(
                                    egui::RichText::new("Exit")
                                        .size(18.0)
                                )
                                .min_size(egui::vec2(60.0, 30.0))
                                .rounding(Rounding::same(20.0))
                            ).clicked() {
                                match self.my_enum {
                                    CastRecEnum::None => self.current_page = Pages::HOME,
                                    CastRecEnum::Caster => self.current_page = Pages::CASTER,
                                    CastRecEnum::Receiver => self.current_page = Pages::RECEIVER,
                                }
                            }
                            ui.with_layout(egui::Layout::centered_and_justified(egui::Direction::LeftToRight), |ui| {
                                ui.heading(egui::RichText::new("Shortcuts").size(32.0).strong());
                            });
                        });
                        ui.add_space(50.0);
                
                        let field_width = 200.0;
                        let button_width = 80.0;
                
                        // Start shortcut
                        ui.horizontal(|ui| {
                            ui.label(egui::RichText::new("Start:").size(24.0).strong());
                            ui.add_space(10.0);
                
                            let display_text = if self.active_shortcut_field.as_ref().map_or(false, |f| f == "start") {
                                self.temp_shortcut_display.clone()
                            } else {
                                self.start_shortcut.as_ref().map_or(String::new(), |sc| {
                                    format!("{}+{:?}", self.get_mod_simbol(sc.modifiers), sc.logical_key)
                                })
                            };

                            // Color the TextEdit content
                            let text_color = if self.active_shortcut_field.as_ref().map_or(false, |f| f == "start") {
                                egui::Color32::RED
                            } else {
                                egui::Color32::GREEN
                            };

                            let response = ui.add_sized(
                                [field_width, 30.0],
                                egui::TextEdit::singleline(&mut display_text.clone())
                                    .font(egui::TextStyle::Heading)
                                    .text_color(text_color)
                                    .hint_text("Press keys...")
                            );
                
                            if response.gained_focus() {
                                self.active_shortcut_field = Some("start".to_string());
                                self.temp_shortcut_display.clear();
                            }
                
                            if ui.add_sized([button_width, 30.0], 
                                egui::Button::new(egui::RichText::new("Save").size(20.0))
                            ).clicked() && self.active_shortcut_field.as_ref().map_or(false, |f| f == "start") {
                                if let Some(sc) = self.last_pressed_shortcut.take() {
                                    self.start_shortcut = Some(sc);
                                    self.active_shortcut_field = None;
                                    self.temp_shortcut_display.clear();
                                }
                            }
                        });
                        ui.add_space(20.0);
                
                        // Pause shortcut
                        ui.horizontal(|ui| {
                            ui.label(egui::RichText::new("Pause:").size(24.0).strong());
                            ui.add_space(10.0);
                
                            let display_text = if self.active_shortcut_field.as_ref().map_or(false, |f| f == "pause") {
                                self.temp_shortcut_display.clone()
                            } else {
                                self.pause_shortcut.as_ref().map_or(String::new(), |sc| {
                                    format!("{}+{:?}", self.get_mod_simbol(sc.modifiers), sc.logical_key)
                                })
                            };
                
                            // Color the TextEdit content
                            let text_color = if self.active_shortcut_field.as_ref().map_or(false, |f| f == "pause") {
                                egui::Color32::RED
                            } else {
                                egui::Color32::GREEN
                            };

                            let response = ui.add_sized(
                                [field_width, 30.0],
                                egui::TextEdit::singleline(&mut display_text.clone())
                                    .font(egui::TextStyle::Heading)
                                    .text_color(text_color)
                                    .hint_text("Press keys...")
                            );
                
                            if response.gained_focus() {
                                self.active_shortcut_field = Some("pause".to_string());
                                self.temp_shortcut_display.clear();
                            }
                
                            if ui.add_sized([button_width, 30.0], 
                                egui::Button::new(egui::RichText::new("Save").size(20.0))
                            ).clicked() && self.active_shortcut_field.as_ref().map_or(false, |f| f == "pause") {
                                if let Some(sc) = self.last_pressed_shortcut.take() {
                                    self.pause_shortcut = Some(sc);
                                    self.active_shortcut_field = None;
                                    self.temp_shortcut_display.clear();
                                }
                            }
                        });
                        ui.add_space(20.0);
                
                        // Blank shortcut
                        ui.horizontal(|ui| {
                            ui.label(egui::RichText::new("Blank:").size(24.0).strong());
                            ui.add_space(10.0);
                
                            let display_text = if self.active_shortcut_field.as_ref().map_or(false, |f| f == "blank") {
                                self.temp_shortcut_display.clone()
                            } else {
                                self.blank_shortcut.as_ref().map_or(String::new(), |sc| {
                                    format!("{}+{:?}", self.get_mod_simbol(sc.modifiers), sc.logical_key)
                                })
                            };
                
                            // Color the TextEdit content
                            let text_color = if self.active_shortcut_field.as_ref().map_or(false, |f| f == "blank") {
                                egui::Color32::RED
                            } else {
                                egui::Color32::GREEN
                            };

                            let response = ui.add_sized(
                                [field_width, 30.0],
                                egui::TextEdit::singleline(&mut display_text.clone())
                                    .font(egui::TextStyle::Heading)
                                    .text_color(text_color)
                                    .hint_text("Press keys...")
                            );
                
                            if response.gained_focus() {
                                self.active_shortcut_field = Some("blank".to_string());
                                self.temp_shortcut_display.clear();
                            }
                
                            if ui.add_sized([button_width, 30.0], 
                                egui::Button::new(egui::RichText::new("Save").size(20.0))
                            ).clicked() && self.active_shortcut_field.as_ref().map_or(false, |f| f == "blank") {
                                if let Some(sc) = self.last_pressed_shortcut.take() {
                                    self.blank_shortcut = Some(sc);
                                    self.active_shortcut_field = None;
                                    self.temp_shortcut_display.clear();
                                }
                            }
                        });
                        ui.add_space(20.0);
                
                        // Stop shortcut
                        ui.horizontal(|ui| {
                            ui.label(egui::RichText::new("Stop:").size(24.0).strong());
                            ui.add_space(10.0);
                
                            let display_text = if self.active_shortcut_field.as_ref().map_or(false, |f| f == "stop") {
                                self.temp_shortcut_display.clone()
                            } else {
                                self.stop_shortcut.as_ref().map_or(String::new(), |sc| {
                                    format!("{}+{:?}", self.get_mod_simbol(sc.modifiers), sc.logical_key)
                                })
                            };
                
                            // Color the TextEdit content
                            let text_color = if self.active_shortcut_field.as_ref().map_or(false, |f| f == "stop") {
                                egui::Color32::RED
                            } else {
                                egui::Color32::GREEN
                            };

                            let response = ui.add_sized(
                                [field_width, 30.0],
                                egui::TextEdit::singleline(&mut display_text.clone())
                                    .font(egui::TextStyle::Heading)
                                    .text_color(text_color)
                                    .hint_text("Press keys...")
                            );
                
                            if response.gained_focus() {
                                self.active_shortcut_field = Some("stop".to_string());
                                self.temp_shortcut_display.clear();
                            }
                
                            if ui.add_sized([button_width, 30.0], 
                                egui::Button::new(egui::RichText::new("Save").size(20.0))
                            ).clicked() && self.active_shortcut_field.as_ref().map_or(false, |f| f == "stop") {
                                if let Some(sc) = self.last_pressed_shortcut.take() {
                                    self.stop_shortcut = Some(sc);
                                    self.active_shortcut_field = None;
                                    self.temp_shortcut_display.clear();
                                }
                            }
                        });
                
                        // Keyboard event handling
                        ui.input(|i| {
                            for event in &i.raw.events {
                                if let egui::Event::Key {
                                    key,
                                    pressed: true,
                                    modifiers,
                                    ..
                                } = event {
                                    if self.active_shortcut_field.is_some() {
                                        let shortcut = egui::KeyboardShortcut::new(modifiers.clone(), key.clone());
                                        self.last_pressed_shortcut = Some(shortcut);
                                        self.temp_shortcut_display = format!("{}+{:?}", 
                                            self.get_mod_simbol(modifiers.clone()),
                                            key
                                        );
                                    }
                                }
                            }
                        });
                    });
                }
                    
            }
        });
    }
}
