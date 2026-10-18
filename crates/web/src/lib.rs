//! `mnemonic-web` — a thin WebAssembly layer over `major_mnemonic_core` for the
//! static web page in `www/`.
//!
//! Every function returns a JSON string (parsed on the JS side). That keeps the
//! boundary tiny and dependency-free, and the core stays the single source of
//! truth: the page never re-implements encoding, decoding or grading.

use major_mnemonic_core::{Challenge, Code, Direction, PartOfSpeech};
use serde::Serialize;
use serde_json::json;
use wasm_bindgen::prelude::wasm_bindgen;

#[derive(Serialize)]
struct DecodedWord {
    text: String,
    /// The word's digits, or `None` if it isn't a peg word.
    code: Option<String>,
}

/// Encode a digit string.
///
/// `{"ok": true, "pretty": "...", "words": [{"text", "pos", "code"}]}` or
/// `{"ok": false, "error": "..."}`.
#[wasm_bindgen]
pub fn encode(digits: &str) -> String {
    match major_mnemonic_core::encode(digits) {
        Ok(s) => json!({ "ok": true, "pretty": s.to_pretty(), "words": s.words }),
        Err(e) => json!({ "ok": false, "error": e.to_string() }),
    }
    .to_string()
}

/// Decode a sentence, with a per-word breakdown (unknown words included, so the
/// page can point at them).
///
/// `{"ok": true, "digits": "...", "words": [...]}` or
/// `{"ok": false, "error": "...", "words": [...]}`.
#[wasm_bindgen]
pub fn decode(text: &str) -> String {
    // Same tokenization as core's decoder; decoding is position-independent,
    // so each token can be decoded on its own.
    let words: Vec<DecodedWord> = text
        .split(|c: char| !c.is_ascii_alphabetic())
        .filter(|t| !t.is_empty())
        .map(|t| DecodedWord {
            text: t.to_string(),
            code: major_mnemonic_core::decode(t).ok(),
        })
        .collect();
    match major_mnemonic_core::decode(text) {
        Ok(digits) => json!({ "ok": true, "digits": digits, "words": words }),
        Err(e) => json!({ "ok": false, "error": e.to_string(), "words": words }),
    }
    .to_string()
}

/// The full peg table: `{"pairs": [{"code", "verb", "adjective", "noun"}; 100],
/// "singles": [{"code", "word"}; 10]}`.
#[wasm_bindgen]
pub fn pegs() -> String {
    let pairs: Vec<_> = (0..100u8)
        .map(|n| {
            // Three copies of a pair encode as one verb-adjective-noun clause.
            let code = Code::Pair(n).digits();
            let s = major_mnemonic_core::encode(&code.repeat(3)).expect("valid digits");
            let word = |pos: PartOfSpeech| s.words[pos.slot()].text.clone();
            json!({
                "code": code,
                "verb": word(PartOfSpeech::Verb),
                "adjective": word(PartOfSpeech::Adjective),
                "noun": word(PartOfSpeech::Noun),
            })
        })
        .collect();
    let singles: Vec<_> = (0..10u8)
        .map(|d| {
            let s = major_mnemonic_core::encode(&d.to_string()).expect("valid digit");
            json!({ "code": d.to_string(), "word": s.words[0].text })
        })
        .collect();
    json!({ "pairs": pairs, "singles": singles }).to_string()
}

/// A training challenge for `number` (random digits chosen by the page):
/// `{"number": "...", "sentence": "..."}`.
#[wasm_bindgen]
pub fn challenge(number: &str) -> String {
    match Challenge::new(number) {
        Ok(c) => json!({ "ok": true, "number": c.number, "sentence": c.sentence }),
        Err(e) => json!({ "ok": false, "error": e.to_string() }),
    }
    .to_string()
}

/// Grade an answer to the challenge for `number`. `to_words` selects the
/// number → words direction (any peg words decoding to `number` are accepted);
/// otherwise the learner was shown words and typed the number.
#[wasm_bindgen]
pub fn grade(number: &str, to_words: bool, answer: &str) -> bool {
    let direction = if to_words {
        Direction::NumberToWords
    } else {
        Direction::WordsToNumber
    };
    Challenge::new(number)
        .map(|c| c.grade(direction, answer))
        .unwrap_or(false)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::Value;

    fn parse(s: String) -> Value {
        serde_json::from_str(&s).unwrap()
    }

    #[test]
    fn encode_and_decode_roundtrip() {
        let e = parse(encode("314159265"));
        assert_eq!(e["ok"], true);
        assert_eq!(e["pretty"], "Meet ready lip, enjoy heal.");
        assert_eq!(e["words"][0]["code"], "31");
        let d = parse(decode(e["pretty"].as_str().unwrap()));
        assert_eq!(d["digits"], "314159265");
        assert_eq!(d["words"].as_array().unwrap().len(), 5);
    }

    #[test]
    fn errors_are_reported_with_breakdown() {
        assert_eq!(parse(encode("12a"))["ok"], false);
        let d = parse(decode("meet zzz"));
        assert_eq!(d["ok"], false);
        assert_eq!(d["words"][0]["code"], "31");
        assert!(d["words"][1]["code"].is_null());
    }

    #[test]
    fn peg_table_is_complete_and_decodes() {
        let p = parse(pegs());
        let pairs = p["pairs"].as_array().unwrap();
        assert_eq!(pairs.len(), 100);
        for row in pairs {
            for pos in ["verb", "adjective", "noun"] {
                let d = parse(decode(row[pos].as_str().unwrap()));
                assert_eq!(d["digits"], row["code"]);
            }
        }
        assert_eq!(p["singles"].as_array().unwrap().len(), 10);
    }

    #[test]
    fn grading_both_directions() {
        let c = parse(challenge("0042"));
        let sentence = c["sentence"].as_str().unwrap();
        assert!(grade("0042", true, sentence));
        assert!(grade("0042", false, "00 42"));
        assert!(!grade("0042", false, "0043"));
    }
}
