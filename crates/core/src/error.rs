//! Error types for the core encode/decode/validate API.

/// Failure to encode a digit string into a [`crate::Sentence`].
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum EncodeError {
    /// The input contained no digits.
    #[error("input is empty; expected one or more digits 0-9")]
    Empty,
    /// The input contained a character that is not an ASCII digit.
    #[error("invalid character {ch:?} at position {pos}; only ASCII digits 0-9 are allowed")]
    NonDigit {
        /// The offending character.
        ch: char,
        /// Byte/char position of the offending character.
        pos: usize,
    },
}

/// Failure to decode a sentence back into its digit string.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum DecodeError {
    /// The input contained no recognizable words.
    #[error("input contains no recognizable words")]
    Empty,
    /// A token was not found anywhere in the peg tables.
    #[error("unknown word {word:?} at word index {index}; not in the peg list")]
    UnknownWord {
        /// The unrecognized token.
        word: String,
        /// Zero-based word index in the sentence.
        index: usize,
    },
}

/// Result of a data-integrity check over the embedded peg tables.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("peg data validation failed with {} error(s)", .errors.len())]
pub struct ValidationReport {
    /// Human-readable description of each problem found.
    pub errors: Vec<String>,
}
