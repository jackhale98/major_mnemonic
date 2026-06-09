//! Sentence model: parts of speech, per-pair codes, words, and rendering.

use std::fmt;

use serde::{Serialize, Serializer};

/// Part of speech assigned to a word by its position in the sentence.
///
/// Words cycle through the parts of speech in this order, forming clauses of
/// the shape `verb adjective noun`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum PartOfSpeech {
    /// First word of each clause.
    Verb,
    /// Second word of each clause.
    Adjective,
    /// Third word of each clause.
    Noun,
}

impl PartOfSpeech {
    /// The fixed cycle order used for both encoding and decoding.
    pub const CYCLE: [PartOfSpeech; 3] = [
        PartOfSpeech::Verb,
        PartOfSpeech::Adjective,
        PartOfSpeech::Noun,
    ];

    /// The part of speech for the word at zero-based `index`, given the
    /// `total` number of words in the sentence.
    ///
    /// Words form clauses of three: `verb adjective noun`. To avoid a dangling
    /// clause, the trailing partial group reads grammatically:
    /// - a 2-word remainder becomes **verb + noun** (skipping the adjective);
    /// - a lone trailing word becomes a **noun**, so the sentence ends on one.
    pub fn for_position(index: usize, total: usize) -> Self {
        let remainder = total % 3;
        let full = total - remainder; // words in complete clauses
        if index < full {
            return Self::CYCLE[index % 3];
        }
        // Trailing partial group (remainder is 1 or 2 here, never 0).
        let offset = index - full;
        match remainder {
            2 => [PartOfSpeech::Verb, PartOfSpeech::Noun][offset],
            _ => PartOfSpeech::Noun,
        }
    }

    /// Stable array index (`Verb` => 0, `Adjective` => 1, `Noun` => 2).
    #[inline]
    pub fn slot(self) -> usize {
        match self {
            PartOfSpeech::Verb => 0,
            PartOfSpeech::Adjective => 1,
            PartOfSpeech::Noun => 2,
        }
    }

    /// Lowercase name, e.g. for diagnostics.
    #[inline]
    pub fn as_str(self) -> &'static str {
        match self {
            PartOfSpeech::Verb => "verb",
            PartOfSpeech::Adjective => "adjective",
            PartOfSpeech::Noun => "noun",
        }
    }
}

impl fmt::Display for PartOfSpeech {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// A digit group that a single word represents.
///
/// Numbers are consumed two digits at a time as [`Code::Pair`]. When the input
/// has an odd number of digits, the final leftover digit becomes a
/// [`Code::Single`] — which may therefore only ever appear as the last word.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Code {
    /// A two-digit group, `00..=99`.
    Pair(u8),
    /// A single trailing digit, `0..=9` (final position only).
    Single(u8),
}

impl Code {
    /// The digit string this code contributes (`Pair(5)` => "05", `Single(7)` => "7").
    pub fn digits(self) -> String {
        match self {
            Code::Pair(n) => format!("{n:02}"),
            Code::Single(d) => format!("{d}"),
        }
    }

    /// Number of digits this code contributes (2 for a pair, 1 for a single).
    pub fn digit_len(self) -> usize {
        match self {
            Code::Pair(_) => 2,
            Code::Single(_) => 1,
        }
    }
}

impl Serialize for Code {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(&self.digits())
    }
}

/// A single emitted word plus enough provenance to reconstruct its digits.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Word {
    /// The word's spelling (lowercase ASCII).
    pub text: String,
    /// The part of speech this word occupies.
    pub pos: PartOfSpeech,
    /// The digit group this word encodes.
    pub code: Code,
}

/// A full sentence encoding a digit string.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Sentence {
    /// The words in order.
    pub words: Vec<Word>,
    /// The number of source digits (lets callers assert round-trip length).
    pub digit_len: usize,
}

impl Sentence {
    /// The exact digit string this sentence encodes (leading zeros preserved).
    pub fn to_digits(&self) -> String {
        self.words.iter().map(|w| w.code.digits()).collect()
    }

    /// Lowercase, space-joined words with no punctuation.
    pub fn to_plain(&self) -> String {
        self.words
            .iter()
            .map(|w| w.text.as_str())
            .collect::<Vec<_>>()
            .join(" ")
    }

    /// Capitalized, clause-grouped rendering: words grouped in threes
    /// (`verb adjective noun`), clauses joined by `", "`, ending in `"."`.
    pub fn to_pretty(&self) -> String {
        if self.words.is_empty() {
            return String::new();
        }
        let clauses: Vec<String> = self
            .words
            .chunks(3)
            .map(|clause| {
                clause
                    .iter()
                    .map(|w| w.text.as_str())
                    .collect::<Vec<_>>()
                    .join(" ")
            })
            .collect();
        let mut s = clauses.join(", ");
        if let Some(first) = s.get_mut(0..1) {
            first.make_ascii_uppercase();
        }
        s.push('.');
        s
    }

    /// Canonical human-facing text (same as [`Sentence::to_pretty`]).
    ///
    /// Decoding is tolerant of case and punctuation, so this round-trips.
    pub fn to_text(&self) -> String {
        self.to_pretty()
    }
}

impl fmt::Display for Sentence {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.to_text())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn full_clauses_cycle_verb_adjective_noun() {
        // total = 6 -> two complete clauses.
        let total = 6;
        let pos: Vec<_> = (0..total)
            .map(|i| PartOfSpeech::for_position(i, total))
            .collect();
        use PartOfSpeech::*;
        assert_eq!(pos, vec![Verb, Adjective, Noun, Verb, Adjective, Noun]);
    }

    #[test]
    fn two_word_remainder_is_verb_noun() {
        use PartOfSpeech::*;
        // total = 2 -> verb, noun (no dangling adjective).
        assert_eq!(PartOfSpeech::for_position(0, 2), Verb);
        assert_eq!(PartOfSpeech::for_position(1, 2), Noun);
        // total = 5 -> clause + verb, noun.
        let pos: Vec<_> = (0..5).map(|i| PartOfSpeech::for_position(i, 5)).collect();
        assert_eq!(pos, vec![Verb, Adjective, Noun, Verb, Noun]);
    }

    #[test]
    fn lone_trailing_word_is_a_noun() {
        use PartOfSpeech::*;
        assert_eq!(PartOfSpeech::for_position(0, 1), Noun);
        // total = 4 -> clause + lone noun.
        let pos: Vec<_> = (0..4).map(|i| PartOfSpeech::for_position(i, 4)).collect();
        assert_eq!(pos, vec![Verb, Adjective, Noun, Noun]);
    }

    #[test]
    fn pos_slot_is_stable() {
        assert_eq!(PartOfSpeech::Verb.slot(), 0);
        assert_eq!(PartOfSpeech::Adjective.slot(), 1);
        assert_eq!(PartOfSpeech::Noun.slot(), 2);
    }

    #[test]
    fn code_renders_digits() {
        assert_eq!(Code::Pair(5).digits(), "05");
        assert_eq!(Code::Pair(0).digits(), "00");
        assert_eq!(Code::Pair(99).digits(), "99");
        assert_eq!(Code::Single(7).digits(), "7");
        assert_eq!(Code::Single(0).digits(), "0");
    }

    #[test]
    fn code_digit_len() {
        assert_eq!(Code::Pair(42).digit_len(), 2);
        assert_eq!(Code::Single(4).digit_len(), 1);
    }

    #[test]
    fn sentence_to_digits_concatenates() {
        let s = Sentence {
            words: vec![
                Word {
                    text: "a".into(),
                    pos: PartOfSpeech::Verb,
                    code: Code::Pair(31),
                },
                Word {
                    text: "b".into(),
                    pos: PartOfSpeech::Adjective,
                    code: Code::Single(4),
                },
            ],
            digit_len: 3,
        };
        assert_eq!(s.to_digits(), "314");
    }

    #[test]
    fn sentence_pretty_groups_and_capitalizes() {
        let s = Sentence {
            words: vec![
                Word {
                    text: "kicks".into(),
                    pos: PartOfSpeech::Verb,
                    code: Code::Pair(70),
                },
                Word {
                    text: "tiny".into(),
                    pos: PartOfSpeech::Adjective,
                    code: Code::Pair(12),
                },
                Word {
                    text: "dog".into(),
                    pos: PartOfSpeech::Noun,
                    code: Code::Pair(17),
                },
                Word {
                    text: "saw".into(),
                    pos: PartOfSpeech::Verb,
                    code: Code::Single(0),
                },
            ],
            digit_len: 7,
        };
        assert_eq!(s.to_pretty(), "Kicks tiny dog, saw.");
        assert_eq!(s.to_plain(), "kicks tiny dog saw");
    }

    #[test]
    fn empty_sentence_pretty_is_empty() {
        let s = Sentence {
            words: vec![],
            digit_len: 0,
        };
        assert_eq!(s.to_pretty(), "");
    }
}
