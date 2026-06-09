//! Embedded peg data: loading, inverse maps, and integrity validation.
//!
//! The data file is the single source of truth for the encode/decode bijection.
//! Each entry stores an explicit `digits` field whose value is validated against
//! the entry's array index (a hard, arithmetic gate) and, separately, against
//! the spelling-based [`crate::major::major_encode`] (a soft typo-catcher).

use std::collections::HashMap;
use std::sync::LazyLock;

use serde::Deserialize;

use crate::error::ValidationReport;
use crate::major::major_encode;
use crate::sentence::{Code, PartOfSpeech};

/// The embedded peg data, compiled into the binary.
const PEG_JSON: &str = include_str!("../data/pegs.v1.json");

/// Number of two-digit pair codes per part of speech (`00..=99`).
const PAIR_COUNT: usize = 100;
/// Number of single-digit codes per part of speech (`0..=9`).
const SINGLE_COUNT: usize = 10;

#[derive(Debug, Deserialize)]
struct RawEntry {
    word: String,
    digits: Vec<u8>,
    #[serde(default)]
    phonetic_exception: bool,
}

#[derive(Debug, Deserialize)]
struct RawByPos {
    verb: Vec<RawEntry>,
    adjective: Vec<RawEntry>,
    noun: Vec<RawEntry>,
}

impl RawByPos {
    fn get(&self, pos: PartOfSpeech) -> &[RawEntry] {
        match pos {
            PartOfSpeech::Verb => &self.verb,
            PartOfSpeech::Adjective => &self.adjective,
            PartOfSpeech::Noun => &self.noun,
        }
    }
}

#[derive(Debug, Deserialize)]
struct RawData {
    schema_version: u32,
    #[serde(default)]
    #[allow(dead_code)]
    source: String,
    pairs: RawByPos,
    singles: RawByPos,
}

/// Indexed, ready-to-use peg tables with forward and inverse maps.
pub struct Tables {
    /// `pairs[pos][code]` => word, for `code` in `0..=99`.
    pairs: [Vec<String>; 3],
    /// `singles[pos][digit]` => word, for `digit` in `0..=9`.
    singles: [Vec<String>; 3],
    /// Position-independent inverse map: `word` => [`Code`], spanning every
    /// part of speech and both pairs and singles. Decoding uses this so a word
    /// is recognized regardless of which grammatical slot it appears in. Its
    /// unambiguity relies on global word uniqueness (a hard validation gate).
    inv_word: HashMap<String, Code>,
}

impl Tables {
    /// Forward lookup: the pair word for `code` (`0..=99`) at part of speech `pos`.
    pub fn pair_word(&self, pos: PartOfSpeech, code: u8) -> &str {
        &self.pairs[pos.slot()][code as usize]
    }

    /// Forward lookup: the single word for `digit` (`0..=9`) at part of speech `pos`.
    pub fn single_word(&self, pos: PartOfSpeech, digit: u8) -> &str {
        &self.singles[pos.slot()][digit as usize]
    }

    /// Position-independent inverse lookup: the [`Code`] for `word`, regardless
    /// of part of speech or pair/single kind. Returns `None` if the word is not
    /// in the peg list.
    pub fn lookup_word(&self, word: &str) -> Option<Code> {
        self.inv_word.get(word).copied()
    }

    fn build(raw: &RawData) -> Tables {
        let mut pairs = [Vec::new(), Vec::new(), Vec::new()];
        let mut singles = [Vec::new(), Vec::new(), Vec::new()];
        let mut inv_word = HashMap::new();

        for pos in PartOfSpeech::CYCLE {
            let slot = pos.slot();
            for (i, e) in raw.pairs.get(pos).iter().enumerate() {
                pairs[slot].push(e.word.clone());
                inv_word.insert(e.word.clone(), Code::Pair(i as u8));
            }
            for (d, e) in raw.singles.get(pos).iter().enumerate() {
                singles[slot].push(e.word.clone());
                inv_word.insert(e.word.clone(), Code::Single(d as u8));
            }
        }

        Tables {
            pairs,
            singles,
            inv_word,
        }
    }
}

/// Parse the embedded JSON. Panics only if the embedded file is malformed,
/// which is prevented by `build.rs` and the validation tests.
fn parse_embedded() -> RawData {
    serde_json::from_str(PEG_JSON).expect("embedded peg data must be valid JSON")
}

/// Lazily-initialized global tables.
static TABLES: LazyLock<Tables> = LazyLock::new(|| {
    let raw = parse_embedded();
    let errors = hard_validate(&raw);
    assert!(
        errors.is_empty(),
        "embedded peg data failed hard validation: {errors:?}"
    );
    Tables::build(&raw)
});

/// Access the global peg tables.
pub fn tables() -> &'static Tables {
    &TABLES
}

fn is_valid_token(word: &str) -> bool {
    !word.is_empty() && word.chars().all(|c| c.is_ascii_lowercase())
}

/// Hard gates: structural invariants the round-trip bijection depends on.
/// These are pure and must always pass for shipped data.
fn hard_validate(raw: &RawData) -> Vec<String> {
    let mut errors = Vec::new();

    if raw.schema_version != 1 {
        errors.push(format!("unsupported schema_version {}", raw.schema_version));
    }

    for pos in PartOfSpeech::CYCLE {
        let pairs = raw.pairs.get(pos);
        let singles = raw.singles.get(pos);

        if pairs.len() != PAIR_COUNT {
            errors.push(format!(
                "{pos}: expected {PAIR_COUNT} pair entries, found {}",
                pairs.len()
            ));
        }
        if singles.len() != SINGLE_COUNT {
            errors.push(format!(
                "{pos}: expected {SINGLE_COUNT} single entries, found {}",
                singles.len()
            ));
        }

        // Index consistency: array position IS the code.
        for (i, e) in pairs.iter().enumerate() {
            let expected = vec![(i / 10) as u8, (i % 10) as u8];
            if e.digits != expected {
                errors.push(format!(
                    "{pos} pair[{i}] ({:?}): digits {:?} != expected {:?}",
                    e.word, e.digits, expected
                ));
            }
            if !is_valid_token(&e.word) {
                errors.push(format!(
                    "{pos} pair[{i}]: {:?} is not a single lowercase ASCII token",
                    e.word
                ));
            }
        }
        for (d, e) in singles.iter().enumerate() {
            let expected = vec![d as u8];
            if e.digits != expected {
                errors.push(format!(
                    "{pos} single[{d}] ({:?}): digits {:?} != expected {:?}",
                    e.word, e.digits, expected
                ));
            }
            if !is_valid_token(&e.word) {
                errors.push(format!(
                    "{pos} single[{d}]: {:?} is not a single lowercase ASCII token",
                    e.word
                ));
            }
        }

        // Injectivity within a part of speech, and pair/single disjointness.
        let mut seen: HashMap<&str, String> = HashMap::new();
        for (i, e) in pairs.iter().enumerate() {
            if let Some(prev) = seen.insert(e.word.as_str(), format!("pair[{i}]")) {
                errors.push(format!(
                    "{pos}: duplicate word {:?} (pair[{i}] and {prev})",
                    e.word
                ));
            }
        }
        for (d, e) in singles.iter().enumerate() {
            if let Some(prev) = seen.insert(e.word.as_str(), format!("single[{d}]")) {
                errors.push(format!(
                    "{pos}: word {:?} appears in both single[{d}] and {prev}",
                    e.word
                ));
            }
        }
    }

    // Global uniqueness gate: decoding is position-independent, so it maps each
    // word to a code without regard to part of speech. That bijection is only
    // well-defined if every word is unique across all six tables (3 parts of
    // speech x {pairs, singles}).
    {
        let mut seen: HashMap<&str, String> = HashMap::new();
        for pos in PartOfSpeech::CYCLE {
            for (kind, entries) in [
                ("pair", raw.pairs.get(pos)),
                ("single", raw.singles.get(pos)),
            ] {
                for (i, e) in entries.iter().enumerate() {
                    let here = format!("{pos} {kind}[{i}]");
                    if let Some(prev) = seen.insert(e.word.as_str(), here.clone()) {
                        errors.push(format!(
                            "word {:?} appears in both {here} and {prev}; \
                             words must be globally unique for position-independent decoding",
                            e.word
                        ));
                    }
                }
            }
        }
    }

    errors
}

/// Soft gate: spelling-based phonetic cross-check. Mismatches that are not
/// flagged with `phonetic_exception` are reported (likely data-entry typos).
fn soft_validate(raw: &RawData) -> Vec<String> {
    let mut errors = Vec::new();
    for pos in PartOfSpeech::CYCLE {
        for (kind, entries) in [
            ("pair", raw.pairs.get(pos)),
            ("single", raw.singles.get(pos)),
        ] {
            for (i, e) in entries.iter().enumerate() {
                if e.phonetic_exception {
                    continue;
                }
                let encoded = major_encode(&e.word);
                if encoded != e.digits {
                    errors.push(format!(
                        "{pos} {kind}[{i}]: {:?} phonetically encodes to {:?} but is mapped to {:?} (add \"phonetic_exception\": true if intentional)",
                        e.word, encoded, e.digits
                    ));
                }
            }
        }
    }
    errors
}

/// Run all data-integrity checks (hard + soft) over the embedded data.
pub fn validate() -> Result<(), ValidationReport> {
    let raw = parse_embedded();
    let mut errors = hard_validate(&raw);
    errors.extend(soft_validate(&raw));
    if errors.is_empty() {
        Ok(())
    } else {
        Err(ValidationReport { errors })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn embedded_data_passes_all_gates() {
        if let Err(report) = validate() {
            panic!(
                "embedded peg data is invalid:\n{}",
                report.errors.join("\n")
            );
        }
    }

    #[test]
    fn tables_have_full_coverage() {
        let t = tables();
        for pos in PartOfSpeech::CYCLE {
            for code in 0u8..=99 {
                assert!(!t.pair_word(pos, code).is_empty());
            }
            for d in 0u8..=9 {
                assert!(!t.single_word(pos, d).is_empty());
            }
        }
    }

    #[test]
    fn forward_inverse_pair_roundtrip() {
        let t = tables();
        for pos in PartOfSpeech::CYCLE {
            for code in 0u8..=99 {
                let w = t.pair_word(pos, code);
                assert_eq!(t.lookup_word(w), Some(Code::Pair(code)));
            }
        }
    }

    #[test]
    fn forward_inverse_single_roundtrip() {
        let t = tables();
        for pos in PartOfSpeech::CYCLE {
            for d in 0u8..=9 {
                let w = t.single_word(pos, d);
                assert_eq!(t.lookup_word(w), Some(Code::Single(d)));
            }
        }
    }

    #[test]
    fn hard_validate_catches_bad_index() {
        let raw: RawData = serde_json::from_str(
            r#"{"schema_version":1,
                "pairs":{"verb":[{"word":"sun","digits":[0,2]}],"adjective":[],"noun":[]},
                "singles":{"verb":[],"adjective":[],"noun":[]}}"#,
        )
        .unwrap();
        let errors = hard_validate(&raw);
        // "sun" encodes 0,2 but sits at pair index 0 which expects [0,0].
        assert!(errors.iter().any(|e| e.contains("pair[0]")));
    }
}
