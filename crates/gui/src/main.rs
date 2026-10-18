//! `mnemonic-gui` — a small native desktop front end for the Mnemonic Major
//! System core. Encode live as you type, or drill yourself in the Train tab.

// Don't pop up a console window alongside the GUI in release builds.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod training;

use eframe::egui;

use training::TrainingView;

fn main() -> eframe::Result<()> {
    let native_options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([480.0, 380.0])
            .with_min_inner_size([380.0, 300.0])
            .with_title("Mnemonic"),
        ..Default::default()
    };
    eframe::run_native(
        "Mnemonic",
        native_options,
        Box::new(|_cc| Ok(Box::<MnemonicApp>::default())),
    )
}

#[derive(PartialEq, Eq)]
enum Tab {
    Encode,
    Train,
}

struct MnemonicApp {
    tab: Tab,
    /// Live encoder input (kept sanitized to ASCII digits only).
    digits: String,
    /// Whether the latest sentence was just copied (reset on edit).
    copied: bool,
    /// Whether we've grabbed initial keyboard focus for the encode input.
    focused_once: bool,
    training: TrainingView,
}

impl Default for MnemonicApp {
    fn default() -> Self {
        Self {
            tab: Tab::Encode,
            digits: String::new(),
            copied: false,
            focused_once: false,
            training: TrainingView::default(),
        }
    }
}

impl eframe::App for MnemonicApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.add_space(8.0);
            ui.horizontal(|ui| {
                ui.selectable_value(&mut self.tab, Tab::Encode, "Encode");
                ui.selectable_value(&mut self.tab, Tab::Train, "Train");
            });
            ui.separator();
            ui.add_space(6.0);

            match self.tab {
                Tab::Encode => self.encode_view(ui),
                Tab::Train => self.training.ui(ui),
            }
        });
    }
}

impl MnemonicApp {
    fn encode_view(&mut self, ui: &mut egui::Ui) {
        ui.label(egui::RichText::new("Enter a number").weak());
        ui.add_space(4.0);

        let response = ui.add(
            egui::TextEdit::singleline(&mut self.digits)
                .hint_text("e.g. 8675309")
                .font(egui::FontId::monospace(22.0))
                .desired_width(f32::INFINITY)
                .margin(egui::vec2(8.0, 6.0)),
        );
        if !self.focused_once {
            response.request_focus();
            self.focused_once = true;
        }

        // Live-filter to digits: typing a non-digit simply does nothing.
        let cleaned: String = self.digits.chars().filter(char::is_ascii_digit).collect();
        if cleaned != self.digits {
            self.digits = cleaned;
        }
        if response.changed() {
            self.copied = false;
        }

        ui.add_space(10.0);
        ui.separator();
        ui.add_space(10.0);

        if self.digits.is_empty() {
            ui.label(
                egui::RichText::new("Your memorable sentence will appear here as you type.")
                    .weak()
                    .italics(),
            );
            return;
        }

        match major_mnemonic_core::encode(&self.digits) {
            Ok(sentence) => {
                let pretty = sentence.to_pretty();
                ui.label(egui::RichText::new(&pretty).size(26.0).strong());

                ui.add_space(12.0);
                ui.horizontal(|ui| {
                    let n = self.digits.len();
                    let plural = if n == 1 { "" } else { "s" };
                    ui.label(egui::RichText::new(format!("{n} digit{plural}")).weak());

                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui.button("Copy").clicked() {
                            ui.ctx().copy_text(pretty.clone());
                            self.copied = true;
                        }
                        if self.copied {
                            ui.label(egui::RichText::new("Copied \u{2713}").weak());
                        }
                    });
                });
            }
            Err(err) => {
                ui.colored_label(egui::Color32::from_rgb(220, 90, 90), err.to_string());
            }
        }
    }
}
