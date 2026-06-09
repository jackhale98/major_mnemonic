//! Decoding a sentence back into its digit string.

use crate::error::DecodeError;
use crate::tables::tables;

/// Decode a sentence back into the exact digit string it encodes.
///
/// Decoding is *position-independent*: each token is looked up in the peg list
/// regardless of its grammatical slot, and the matched codes are concatenated
/// in reading order. Any combination of peg words therefore decodes, in
/// whatever order they appear — but reading order still determines the digit
/// order (`meet row` is `314`, `row meet` is `431`). Words may be pairs (two
/// digits) or singles (one digit), and either may appear at any position.
/// Input is tolerant of capitalization and punctuation.
pub fn decode(text: &str) -> Result<String, DecodeError> {
    let tokens: Vec<String> = text
        .split(|c: char| !c.is_ascii_alphabetic())
        .filter(|s| !s.is_empty())
        .map(|s| s.to_ascii_lowercase())
        .collect();

    if tokens.is_empty() {
        return Err(DecodeError::Empty);
    }

    let t = tables();
    let mut out = String::with_capacity(tokens.len() * 2);

    for (index, token) in tokens.iter().enumerate() {
        match t.lookup_word(token) {
            Some(code) => out.push_str(&code.digits()),
            None => {
                return Err(DecodeError::UnknownWord {
                    word: token.clone(),
                    index,
                })
            }
        }
    }

    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::encode::encode;

    #[test]
    fn rejects_empty() {
        assert_eq!(decode("   ... "), Err(DecodeError::Empty));
    }

    #[test]
    fn unknown_word_reports_position() {
        let err = decode("zzqzz").unwrap_err();
        match err {
            DecodeError::UnknownWord { index, word } => {
                assert_eq!(index, 0);
                assert_eq!(word, "zzqzz");
            }
            other => panic!("expected UnknownWord, got {other:?}"),
        }
    }

    #[test]
    fn decodes_words_in_any_grammatical_order() {
        // Encode picks a verb, adjective, noun in that order...
        let canonical = encode("314159").unwrap().to_plain();
        let words: Vec<&str> = canonical.split(' ').collect();
        assert_eq!(words.len(), 3);
        // ...but decode accepts the same words rearranged, reading digits in
        // whatever order the words appear.
        let reversed = format!("{} {} {}", words[2], words[1], words[0]);
        assert_eq!(decode(&canonical).unwrap(), "314159");
        // verb(31) adj(41) noun(59) reversed -> noun(59) adj(41) verb(31).
        assert_eq!(decode(&reversed).unwrap(), "594131");
    }

    #[test]
    fn single_word_decodes_at_any_position() {
        // A single (one-digit) word used to be valid only as the final word.
        // Now it decodes anywhere, mixed freely with pair words.
        let single = encode("7").unwrap().to_plain(); // one-digit word -> "7"
        let pair = encode("31").unwrap().to_plain(); // two-digit word -> "31"
        assert_eq!(decode(&format!("{single} {pair}")).unwrap(), "731");
        assert_eq!(decode(&format!("{pair} {single}")).unwrap(), "317");
    }

    #[test]
    fn tolerates_case_and_punctuation() {
        let sentence = encode("314159").unwrap();
        let pretty = sentence.to_pretty(); // capitalized + commas + period
        assert_eq!(decode(&pretty).unwrap(), "314159");
        assert_eq!(decode(&pretty.to_uppercase()).unwrap(), "314159");
    }

    #[test]
    fn roundtrips_odd_length_with_single() {
        let sentence = encode("314").unwrap();
        assert_eq!(decode(&sentence.to_text()).unwrap(), "314");
    }
}
