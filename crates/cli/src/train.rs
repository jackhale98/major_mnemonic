//! Interactive training sessions for the `train` subcommand.

use std::io::{BufRead, Write};
use std::process::ExitCode;
use std::time::{Duration, Instant};

use rand::rngs::StdRng;
use rand::{Rng, SeedableRng};

use major_mnemonic_core::{Challenge, Direction, RoundOutcome};

use crate::cli::{TrainArgs, TrainMode};

/// Run a training session, reading answers a line at a time from stdin.
pub fn run(args: TrainArgs) -> ExitCode {
    if args.digits == 0 || args.rounds == 0 {
        eprintln!("error: --digits and --rounds must each be at least 1");
        return ExitCode::from(2);
    }

    let direction = match args.mode {
        TrainMode::Decode => Direction::WordsToNumber,
        TrainMode::Encode => Direction::NumberToWords,
    };
    let mut rng: StdRng = match args.seed {
        Some(seed) => StdRng::seed_from_u64(seed),
        None => StdRng::from_entropy(),
    };

    print_intro(&args, direction);

    let stdin = std::io::stdin();
    let mut lines = stdin.lock().lines();
    let mut reviews: Vec<RoundOutcome> = Vec::new();
    let start = Instant::now();

    for round in 1..=args.rounds {
        let number = random_number(&mut rng, args.digits);
        let challenge = Challenge::new(number).expect("random digits are always valid");

        println!("Round {round}/{}", args.rounds);
        println!("  {}", challenge.prompt(direction));
        print!("  your answer: ");
        let _ = std::io::stdout().flush();

        let answer = match lines.next() {
            Some(Ok(line)) => line,
            _ => {
                println!("\n(session ended early)");
                break;
            }
        };

        let outcome = challenge.review(direction, &answer);
        if outcome.correct {
            match direction {
                // Show the program's own encoding as a learning aid; the
                // learner's different-but-valid answer still counts.
                Direction::NumberToWords => println!("  correct! (e.g. {})\n", outcome.expected),
                Direction::WordsToNumber => println!("  correct!\n"),
            }
        } else {
            println!("  wrong - answer: {}\n", outcome.expected);
        }
        reviews.push(outcome);
    }

    print_summary(&reviews, start.elapsed());
    ExitCode::SUCCESS
}

fn print_intro(args: &TrainArgs, direction: Direction) {
    let how = match direction {
        Direction::WordsToNumber => "Read each sentence and type the number it encodes.",
        Direction::NumberToWords => {
            "Read each number and type words for it. Any words that encode the \
             number count - e.g. spelling it out with single-digit words is fine."
        }
    };
    println!(
        "Training: {} round(s), {} digit(s) each.",
        args.rounds, args.digits
    );
    println!("{how}");
    println!("Press Enter after each answer.\n");
}

fn print_summary(reviews: &[RoundOutcome], elapsed: Duration) {
    println!("----------");
    if reviews.is_empty() {
        println!("No rounds played.");
        return;
    }

    println!("Review:");
    for (i, o) in reviews.iter().enumerate() {
        let mark = if o.correct { "ok" } else { "X " };
        let guess = if o.guess.is_empty() { "(blank)" } else { &o.guess };
        println!("  {:>2}. [{mark}] {}", i + 1, o.prompt);
        println!("        you: {guess}   answer: {}", o.expected);
    }

    println!("----------");
    let correct = reviews.iter().filter(|o| o.correct).count();
    let n = reviews.len();
    let pct = (correct as f64 / n as f64) * 100.0;
    let avg = elapsed / n as u32;
    println!("Score: {correct}/{n} ({pct:.0}%)");
    println!("Time:  {} total, {} avg/round", secs(elapsed), secs(avg));
}

/// A random `n`-digit string (leading zeros allowed — they round-trip fine).
fn random_number(rng: &mut impl Rng, n: usize) -> String {
    (0..n)
        .map(|_| char::from(b'0' + rng.gen_range(0..10u8)))
        .collect()
}

fn secs(d: Duration) -> String {
    format!("{:.1}s", d.as_secs_f64())
}
