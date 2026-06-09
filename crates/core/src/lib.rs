//! Reversible Mnemonic Major System core.
//!
//! Turns a digit string into a memorable, grammatical-ish sentence — and turns
//! that sentence back into the exact original digits. The encode/decode mapping
//! is a strict bijection backed by curated peg tables (see `data/pegs.v1.json`).
//!
//! ```
//! use major_mnemonic_core::{encode, decode};
//! let sentence = encode("314159").unwrap();
//! assert_eq!(decode(&sentence.to_text()).unwrap(), "314159");
//! ```
//!
//! This crate is intentionally free of any UI/CLI/FFI dependencies so it can be
//! reused across the CLI, desktop (Tauri), and mobile (UniFFI) front ends.

mod decode;
mod encode;
mod error;
mod major;
mod sentence;
mod tables;
pub mod training;

pub use decode::decode;
pub use encode::encode;
pub use error::{DecodeError, EncodeError, ValidationReport};
pub use sentence::{Code, PartOfSpeech, Sentence, Word};
pub use tables::validate;
pub use training::{Challenge, Direction, RoundOutcome};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn end_to_end_roundtrip_example() {
        let sentence = encode("314159265").unwrap();
        assert_eq!(decode(&sentence.to_text()).unwrap(), "314159265");
    }
}
