//! `mnemonic` — command-line front end for the Mnemonic Major System core.

mod cli;
mod train;

use std::io::Read;
use std::process::ExitCode;

use clap::Parser;

use cli::{Cli, Command, EncodeArgs, Format};
use major_mnemonic_core::{decode, encode, validate, Sentence};

// Exit-code contract (kept stable; covered by integration tests).
const EXIT_INPUT_ERROR: u8 = 2; // bad/empty input to encode
const EXIT_DECODE_ERROR: u8 = 3; // unrecognized sentence
const EXIT_VALIDATE_ERROR: u8 = 4; // embedded data failed validation

fn main() -> ExitCode {
    run(Cli::parse())
}

fn run(cli: Cli) -> ExitCode {
    // No subcommand: a bare number (or stdin) is an implicit `encode`.
    let command = cli.command.unwrap_or(Command::Encode(cli.encode));
    match command {
        Command::Encode(EncodeArgs { digits, format }) => {
            let input = match read_input(digits) {
                Ok(s) => s,
                Err(e) => return fail(EXIT_INPUT_ERROR, &e),
            };
            match encode(input.trim()) {
                Ok(sentence) => {
                    print_sentence(&sentence, format);
                    ExitCode::SUCCESS
                }
                Err(e) => fail(EXIT_INPUT_ERROR, &e),
            }
        }
        Command::Decode { sentence } => {
            let input = match read_input(sentence) {
                Ok(s) => s,
                Err(e) => return fail(EXIT_INPUT_ERROR, &e),
            };
            match decode(&input) {
                Ok(digits) => {
                    println!("{digits}");
                    ExitCode::SUCCESS
                }
                Err(e) => fail(EXIT_DECODE_ERROR, &e),
            }
        }
        Command::Train(args) => train::run(args),
        Command::Validate => match validate() {
            Ok(()) => {
                println!("ok: embedded peg data is valid");
                ExitCode::SUCCESS
            }
            Err(report) => {
                for e in &report.errors {
                    eprintln!("error: {e}");
                }
                ExitCode::from(EXIT_VALIDATE_ERROR)
            }
        },
    }
}

/// Use the positional argument if given, otherwise read all of stdin.
fn read_input(arg: Option<String>) -> std::io::Result<String> {
    match arg {
        Some(s) => Ok(s),
        None => {
            let mut buf = String::new();
            std::io::stdin().read_to_string(&mut buf)?;
            Ok(buf)
        }
    }
}

fn print_sentence(sentence: &Sentence, format: Format) {
    match format {
        Format::Pretty => println!("{}", sentence.to_pretty()),
        Format::Plain => println!("{}", sentence.to_plain()),
        Format::Json => {
            println!(
                "{}",
                serde_json::to_string_pretty(sentence).expect("Sentence serializes")
            )
        }
    }
}

fn fail(code: u8, err: &dyn std::fmt::Display) -> ExitCode {
    eprintln!("error: {err}");
    ExitCode::from(code)
}
