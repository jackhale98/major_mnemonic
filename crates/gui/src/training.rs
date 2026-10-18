//! The Train tab: a small, timed quiz over the peg system.
//!
//! A session moves through three states — pick settings, play rounds, see the
//! score. Each sub-view takes ownership of its state and returns the next one,
//! which keeps the transitions explicit and the borrow checker happy.

use std::time::Duration;

use eframe::egui;
use major_mnemonic_core::{Challenge, Direction, RoundOutcome};
use rand::rngs::StdRng;
use rand::{Rng, SeedableRng};
// `std::time::Instant` panics on wasm32; this works on native and web alike.
use web_time::Instant;

pub struct TrainingView {
    direction: Direction,
    digits: usize,
    rounds: usize,
    state: State,
}

impl Default for TrainingView {
    fn default() -> Self {
        Self {
            direction: Direction::WordsToNumber,
            digits: 4,
            rounds: 10,
            state: State::Setup,
        }
    }
}

enum State {
    Setup,
    // Boxed: a Session is much larger than the other variants (it owns an RNG).
    Active(Box<Session>),
    Done(Summary),
}

/// A round in progress.
struct Session {
    rng: StdRng,
    direction: Direction,
    digits: usize,
    total: usize,
    /// Zero-based index of the current round.
    index: usize,
    challenge: Challenge,
    input: String,
    started: Instant,
    /// Every round graded so far, in order (drives both the inline feedback and
    /// the end-of-session review).
    reviews: Vec<RoundOutcome>,
    /// Request focus on the input on the next frame.
    focus: bool,
    /// Set when the learner clicks Quit, ending the session early.
    quit: bool,
}

struct Summary {
    /// Every round completed, in order (may be < requested if quit early).
    reviews: Vec<RoundOutcome>,
    elapsed: Duration,
}

impl TrainingView {
    pub fn ui(&mut self, ui: &mut egui::Ui) {
        // Take the state out, render, and store whatever state comes next.
        let state = std::mem::replace(&mut self.state, State::Setup);
        self.state = match state {
            State::Setup => self.setup_ui(ui),
            State::Active(session) => self.active_ui(ui, session),
            State::Done(summary) => self.done_ui(ui, summary),
        };
    }

    fn setup_ui(&mut self, ui: &mut egui::Ui) -> State {
        ui.add_space(4.0);
        ui.heading("Practice");
        ui.add_space(10.0);

        egui::Grid::new("train_settings")
            .num_columns(2)
            .spacing([14.0, 10.0])
            .show(ui, |ui| {
                ui.label("Direction");
                ui.vertical(|ui| {
                    ui.radio_value(
                        &mut self.direction,
                        Direction::WordsToNumber,
                        "Words \u{2192} number",
                    );
                    ui.radio_value(
                        &mut self.direction,
                        Direction::NumberToWords,
                        "Number \u{2192} words",
                    );
                });
                ui.end_row();

                ui.label("Digits per round");
                ui.add(egui::DragValue::new(&mut self.digits).range(1..=12).speed(0.1));
                ui.end_row();

                ui.label("Rounds");
                ui.add(egui::DragValue::new(&mut self.rounds).range(1..=50).speed(0.2));
                ui.end_row();
            });

        ui.add_space(8.0);
        let hint = match self.direction {
            Direction::WordsToNumber => "You'll see a sentence; type the number it stands for.",
            Direction::NumberToWords => {
                "You'll see a number; type any words that encode it \
                 (e.g. single-digit words are fine)."
            }
        };
        ui.label(egui::RichText::new(hint).weak());

        ui.add_space(14.0);
        if ui.button("Start").clicked() {
            State::Active(Box::new(self.start_session()))
        } else {
            State::Setup
        }
    }

    fn active_ui(&mut self, ui: &mut egui::Ui, mut session: Box<Session>) -> State {
        // Keep the clock ticking without a busy-loop.
        ui.ctx().request_repaint_after(Duration::from_millis(250));

        ui.horizontal(|ui| {
            ui.label(
                egui::RichText::new(format!("Round {}/{}", session.index + 1, session.total))
                    .strong(),
            );
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui.button("Quit").clicked() {
                    session.quit = true;
                }
                ui.label(egui::RichText::new(fmt_clock(session.started.elapsed())).weak());
            });
        });

        // Feedback on the previous round.
        match session.reviews.last() {
            Some(r) if r.correct => {
                // In the number->words drill, show the program's own encoding as
                // a learning aid even though the learner's answer was accepted.
                let text = if session.direction == Direction::NumberToWords {
                    format!("\u{2713} correct \u{00b7} e.g. {}", r.expected)
                } else {
                    "\u{2713} correct".to_string()
                };
                ui.colored_label(egui::Color32::from_rgb(80, 170, 90), text);
            }
            Some(r) => {
                ui.colored_label(
                    egui::Color32::from_rgb(210, 110, 110),
                    format!("\u{2717} was {}", r.expected),
                );
            }
            None => {
                ui.add_space(ui.text_style_height(&egui::TextStyle::Body));
            }
        }

        ui.add_space(10.0);

        // The prompt.
        let prompt = session.challenge.prompt(session.direction);
        let prompt_text = match session.direction {
            Direction::WordsToNumber => egui::RichText::new(prompt).size(26.0).strong(),
            Direction::NumberToWords => egui::RichText::new(prompt).monospace().size(28.0).strong(),
        };
        ui.label(prompt_text);
        ui.add_space(12.0);

        // The answer field.
        let hint = match session.direction {
            Direction::WordsToNumber => "type the number",
            Direction::NumberToWords => "type the words",
        };
        let response = ui.add(
            egui::TextEdit::singleline(&mut session.input)
                .hint_text(hint)
                .font(egui::FontId::proportional(18.0))
                .desired_width(f32::INFINITY)
                .margin(egui::vec2(8.0, 6.0)),
        );
        if session.focus {
            response.request_focus();
            session.focus = false;
        }
        // In the words->number drill the answer is digits only.
        if session.direction == Direction::WordsToNumber {
            let cleaned: String = session.input.chars().filter(char::is_ascii_digit).collect();
            if cleaned != session.input {
                session.input = cleaned;
            }
        }

        ui.add_space(10.0);
        let enter = response.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter));
        let submit = ui.button("Submit").clicked() || enter;

        if session.quit {
            return State::Done(Summary {
                reviews: session.reviews,
                elapsed: session.started.elapsed(),
            });
        }
        if submit && !session.input.trim().is_empty() {
            return self.advance(session);
        }
        State::Active(session)
    }

    /// Grade the current answer and move to the next round (or finish).
    fn advance(&self, mut session: Box<Session>) -> State {
        let outcome = session
            .challenge
            .review(session.direction, &session.input);
        session.reviews.push(outcome);
        session.index += 1;

        if session.index >= session.total {
            return State::Done(Summary {
                reviews: session.reviews,
                elapsed: session.started.elapsed(),
            });
        }
        session.challenge = make_challenge(&mut session.rng, session.digits);
        session.input.clear();
        session.focus = true;
        State::Active(session)
    }

    fn done_ui(&mut self, ui: &mut egui::Ui, summary: Summary) -> State {
        ui.add_space(6.0);
        ui.heading("Session complete");
        ui.add_space(10.0);

        let total = summary.reviews.len();
        if total == 0 {
            ui.label("No rounds completed.");
        } else {
            let correct = summary.reviews.iter().filter(|o| o.correct).count();
            let pct = correct as f64 / total as f64 * 100.0;
            ui.label(
                egui::RichText::new(format!("Score: {correct}/{total} ({pct:.0}%)"))
                    .size(22.0)
                    .strong(),
            );
            ui.add_space(4.0);
            let avg = summary.elapsed / total as u32;
            ui.label(
                egui::RichText::new(format!(
                    "Time: {} total \u{00b7} {:.1}s avg/round",
                    fmt_clock(summary.elapsed),
                    avg.as_secs_f64()
                ))
                .weak(),
            );
        }

        // Action buttons stay above the (possibly long) review list.
        ui.add_space(12.0);
        let mut next = None;
        ui.horizontal(|ui| {
            if ui.button("Train again").clicked() {
                next = Some(State::Active(Box::new(self.start_session())));
            }
            if ui.button("Change settings").clicked() {
                next = Some(State::Setup);
            }
        });

        if total > 0 {
            ui.add_space(8.0);
            ui.separator();
            ui.add_space(6.0);
            ui.label(egui::RichText::new("Review").strong());
            ui.add_space(4.0);
            egui::ScrollArea::vertical()
                .auto_shrink([false, false])
                .show(ui, |ui| {
                    for (i, o) in summary.reviews.iter().enumerate() {
                        review_row(ui, i + 1, o);
                    }
                });
        }

        next.unwrap_or(State::Done(summary))
    }

    fn start_session(&self) -> Session {
        let mut rng = StdRng::from_entropy();
        let challenge = make_challenge(&mut rng, self.digits);
        Session {
            rng,
            direction: self.direction,
            digits: self.digits,
            total: self.rounds,
            index: 0,
            challenge,
            input: String::new(),
            started: Instant::now(),
            reviews: Vec::new(),
            focus: true,
            quit: false,
        }
    }
}

/// A challenge for a random `digits`-long number (leading zeros allowed).
fn make_challenge(rng: &mut StdRng, digits: usize) -> Challenge {
    let number: String = (0..digits)
        .map(|_| char::from(b'0' + rng.gen_range(0..10u8)))
        .collect();
    Challenge::new(number).expect("random digits are always valid")
}

/// One entry in the end-of-session review: the prompt with a pass/fail mark,
/// then the learner's guess and the canonical answer beneath it.
fn review_row(ui: &mut egui::Ui, n: usize, o: &RoundOutcome) {
    let (mark, color) = if o.correct {
        ("\u{2713}", egui::Color32::from_rgb(80, 170, 90))
    } else {
        ("\u{2717}", egui::Color32::from_rgb(210, 110, 110))
    };
    ui.horizontal(|ui| {
        ui.colored_label(color, mark);
        ui.label(egui::RichText::new(format!("{n}. {}", o.prompt)).strong());
    });
    let guess = if o.guess.is_empty() { "(blank)" } else { &o.guess };
    ui.label(
        egui::RichText::new(format!("    you: {guess}    answer: {}", o.expected)).weak(),
    );
    ui.add_space(4.0);
}

fn fmt_clock(d: Duration) -> String {
    let secs = d.as_secs();
    format!("{}:{:02}", secs / 60, secs % 60)
}
