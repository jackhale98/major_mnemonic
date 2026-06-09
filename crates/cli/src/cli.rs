//! Command-line argument definitions.

use clap::{Parser, Subcommand, ValueEnum};

/// Mnemonic Major System encoder/decoder.
///
/// A bare number with no subcommand is treated as `encode`, so
/// `mnemonic 314159` is shorthand for `mnemonic encode 314159`.
#[derive(Debug, Parser)]
#[command(name = "mnemonic", version, about, long_about = None)]
#[command(args_conflicts_with_subcommands = true)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Option<Command>,

    /// Digits to encode when no subcommand is given (implicit `encode`).
    #[command(flatten)]
    pub encode: EncodeArgs,
}

/// Top-level encode arguments, shared by the implicit `encode` shorthand.
#[derive(Debug, clap::Args)]
pub struct EncodeArgs {
    /// The digits to encode. If omitted, read from stdin.
    pub digits: Option<String>,
    /// Output format.
    #[arg(long, value_enum, default_value_t = Format::Pretty)]
    pub format: Format,
}

#[derive(Debug, Subcommand)]
pub enum Command {
    /// Encode a number into a memorable sentence.
    Encode(EncodeArgs),
    /// Decode a sentence back into its number.
    Decode {
        /// The sentence to decode. If omitted, read from stdin.
        sentence: Option<String>,
    },
    /// Practice recalling numbers from mnemonics (or vice versa).
    Train(TrainArgs),
    /// Validate the embedded peg data.
    Validate,
}

/// Options for an interactive training session.
#[derive(Debug, clap::Args)]
pub struct TrainArgs {
    /// How many digits each round's number has.
    #[arg(long, default_value_t = 4)]
    pub digits: usize,
    /// How many rounds to play.
    #[arg(long, default_value_t = 10)]
    pub rounds: usize,
    /// Practice direction.
    #[arg(long, value_enum, default_value_t = TrainMode::Decode)]
    pub mode: TrainMode,
    /// Seed the RNG for a reproducible session (mainly for testing).
    #[arg(long)]
    pub seed: Option<u64>,
}

/// Direction of a training drill.
#[derive(Debug, Clone, Copy, ValueEnum)]
pub enum TrainMode {
    /// Shown the words, recall the number (the usual drill).
    Decode,
    /// Shown the number, recall the words (advanced).
    Encode,
}

/// Rendering style for an encoded sentence.
#[derive(Debug, Clone, Copy, ValueEnum)]
pub enum Format {
    /// Capitalized, clause-grouped, punctuated (default).
    Pretty,
    /// Lowercase, space-joined words, no punctuation.
    Plain,
    /// Structured JSON (words with part of speech and code).
    Json,
}
