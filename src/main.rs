mod backends;
mod core;

use eframe::egui;
use egui::{CentralPanel, Key, Label, RichText, Slider, ViewportBuilder, ViewportId};
use rfd::FileDialog;

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use crate::backends::chip8::CHIP8;
use crate::backends::nes::NES;
use crate::core::{AudioEngine, Backend, ConfigValue, InputEvent, InputKind};

fn main() {
    let native_options = eframe::NativeOptions::default();
    let _ = eframe::run_native(
        "Silk Suite",
        native_options,
        Box::new(|cc| Ok(Box::new(SilkSuite::new(cc)))),
    );
}

struct SilkSuite {
    backend: Box<dyn Backend>,
    screen_texture: Option<egui::TextureHandle>,
    audio: AudioEngine,
    rom_loaded: bool,
    show_config_window: Arc<AtomicBool>,
}

impl SilkSuite {
    fn new(_cc: &eframe::CreationContext<'_>) -> Self {
        Self {
            backend: Box::new(CHIP8::new()),
            screen_texture: None,
            audio: AudioEngine::new(44100),
            rom_loaded: false,
            show_config_window: Arc::new(AtomicBool::new(false)),
        }
    }
}

impl eframe::App for SilkSuite {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        // Only run emulator if a ROM and backend are loaded
        if self.rom_loaded {
            // Run emulator
            self.backend.frame_tick();

            // Update Audio
            let mut samples = Vec::new();
            self.backend.flush_audio_samples(&mut samples);
            self.audio.push_samples(&samples);

            // Update Graphics
            let (w, h) = self.backend.framebuffer_size();

            let image =
                egui::ColorImage::from_rgb([w, h], self.backend.framebuffer_rgba());

            let tex = self.screen_texture.get_or_insert_with(|| {
                ui.ctx()
                    .load_texture("screen", image.clone(), egui::TextureOptions::NEAREST)
            });

            tex.set(image, egui::TextureOptions::NEAREST);
        }

        // Draw window contents
        egui::MenuBar::new().ui(ui, |ui| {
            ui.menu_button("File", |ui| {
                if ui.button("Load ROM").clicked() {
                    let file = FileDialog::new().set_directory("./roms").pick_file();

                    if let Some(path) = file {
                        // Check the file extension and map it to a backend if possible
                        let extension = path
                            .extension()
                            .expect("Failed to retrieve file extension!")
                            .to_string_lossy()
                            .to_ascii_lowercase();
                        if CHIP8::file_extensions().contains(&extension.as_str()) {
                            let config = self.backend.config();
                            self.backend = Box::new(CHIP8::new());
                            *self.backend.config().lock().unwrap() = config.lock().unwrap().clone();
                        }
                        if NES::file_extensions().contains(&extension.as_str()) {
                            let config = self.backend.config();
                            self.backend = Box::new(NES::new());
                            *self.backend.config().lock().unwrap() = config.lock().unwrap().clone();
                        }

                        self.audio.clear();
                        self.backend.reset();

                        let rom_bytes = std::fs::read(path).expect("Failed to read ROM from path");
                        self.backend.load_rom(&rom_bytes);
                        self.rom_loaded = true;
                    }
                }

                if ui.button("Reset ROM").clicked() {
                    self.backend.reset_rom();
                }

                if ui.button("Quit").clicked() {
                    ui.send_viewport_cmd(egui::ViewportCommand::Close);
                }
            });

            ui.menu_button("Edit", |ui| {
                if self.rom_loaded && ui.button("Open Config").clicked() {
                    self.show_config_window.store(true, Ordering::Relaxed);
                }
            });
        });

        egui::CentralPanel::default().show(ui, |ui| {
            if let Some(tex) = &self.screen_texture {
                let (w, h) = self.backend.framebuffer_size();
                let scale = 2.0;

                ui.centered_and_justified(|ui| {
                    ui.image((tex.id(), egui::vec2(w as f32 * scale, h as f32 * scale)));
                });
            } else {
                ui.label("No ROM loaded");
            }
        });

        let ctx = ui.ctx();
        ctx.request_repaint();

        if self.show_config_window.load(Ordering::Relaxed) {
            let show_config_window = self.show_config_window.clone();
            let config = self.backend.config().clone();
            ui.show_viewport_deferred(
                ViewportId::from_hash_of("config"),
                ViewportBuilder::default()
                    .with_title("Config")
                    .with_inner_size([640.0, 480.0]),
                move |ctx, _| {
                    CentralPanel::default().show(ctx, |ui| {
                        ui.vertical_centered(|ui| {
                            ui.add(Label::new(RichText::new("Config").size(24.0)));
                            config
                                .lock()
                                .expect("Lock poisoned!")
                                .iter_mut()
                                .for_each(|entry| {
                                    ui.horizontal(|ui| {
                                        ui.add(Label::new(
                                            RichText::new(entry.label.clone()).size(16.0),
                                        ));
                                        match &mut entry.value {
                                            ConfigValue::Bool(value) => {
                                                ui.checkbox(value, "Checked");
                                            }
                                            ConfigValue::Integer {
                                                raw: value,
                                                range: (min, max),
                                            } => {
                                                ui.add(Slider::new(value, *min..=*max));
                                            }
                                            ConfigValue::Float {
                                                raw: value,
                                                range: (min, max),
                                            } => {
                                                ui.add(Slider::new(value, *min..=*max));
                                            }
                                            ConfigValue::Text(value) => {
                                                ui.text_edit_singleline(value);
                                            }
                                            ConfigValue::Choice { selected, options } => {
                                                let visual_selection =
                                                    if *selected == 999 { 0 } else { *selected };
                                                egui::ComboBox::from_label("Select one!")
                                                    .selected_text(format!(
                                                        "{:?}",
                                                        options[visual_selection as usize]
                                                    ))
                                                    .show_ui(ui, |ui| {
                                                        options.iter().enumerate().for_each(
                                                            |(idx, option)| {
                                                                ui.selectable_value(
                                                                    selected, idx as u32, option,
                                                                );
                                                            },
                                                        );
                                                    });
                                            }
                                        }
                                    });
                                });
                        });
                    });

                    if ctx.input(|i| i.viewport().close_requested()) {
                        show_config_window.store(false, Ordering::Relaxed);
                    }
                },
            );
        }

        // Handle input only if emulator is running
        if self.rom_loaded {
            let mut events = Vec::new();

            for (idx, input) in self.backend.input_map().iter().enumerate() {
                match input.kind {
                    InputKind::Digital => {
                        let key = Key::from_name(input.name).expect("Invalid key mapped to input!");

                        if ctx.input(|i| i.key_pressed(key)) {
                            events.push(InputEvent::Digital(idx, true));
                        }

                        if ctx.input(|i| i.key_released(key)) {
                            events.push(InputEvent::Digital(idx, false));
                        }
                    }

                    InputKind::Analog => todo!(),
                }
            }

            for event in events {
                self.backend.set_input(event);
            }
        }
    }
}
