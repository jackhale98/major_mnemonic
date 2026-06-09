//! Encoding a digit string into a [`Sentence`].

use crate::error::EncodeError;
use crate::sentence::{Code, PartOfSpeech, Sentence, Word};
use crate::tables::tables;

/// Encode a digit string (e.g. `"314159"`) into a memorable sentence.
///
/// The input is treated as a *digit string*, so leading zeros are significant
/// and preserved on the round trip. Digits are consumed two at a time; a final
/// leftover digit (odd-length input) becomes a single-digit **verb** as the
/// last word, which tends to read more naturally than a trailing lone noun.
pub fn encode(digits: &str) -> Result<Sentence, EncodeError> {
    if digits.is_empty() {
        return Err(EncodeError::Empty);
    }
    for (pos, ch) in digits.char_indices() {
        if !ch.is_ascii_digit() {
            return Err(EncodeError::NonDigit { ch, pos });
        }
    }

    let ds: Vec<u8> = digits.bytes().map(|b| b - b'0').collect();
    let t = tables();

    let total_words = ds.len().div_ceil(2);
    let mut words = Vec::with_capacity(total_words);
    let mut i = 0;
    let mut word_index = 0;
    while i < ds.len() {
        let pos = PartOfSpeech::for_position(word_index, total_words);
        let word = if i + 1 < ds.len() {
            let code = ds[i] * 10 + ds[i + 1];
            i += 2;
            Word {
                text: t.pair_word(pos, code).to_string(),
                pos,
                code: Code::Pair(code),
            }
        } else {
            // A trailing odd digit is rendered as a single-digit verb rather
            // than taking its positional part of speech (always a noun). This
            // keeps the bijection: the last word's slot is always a noun, so
            // decode can try a noun pair first and fall back to a verb single.
            let d = ds[i];
            i += 1;
            let pos = PartOfSpeech::Verb;
            Word {
                text: t.single_word(pos, d).to_string(),
                pos,
                code: Code::Single(d),
            }
        };
        words.push(word);
        word_index += 1;
    }

    Ok(Sentence {
        words,
        digit_len: digits.len(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_empty() {
        assert_eq!(encode(""), Err(EncodeError::Empty));
    }

    #[test]
    fn rejects_non_digit() {
        assert_eq!(
            encode("12a3"),
            Err(EncodeError::NonDigit { ch: 'a', pos: 2 })
        );
    }

    #[test]
    fn even_length_is_all_pairs() {
        let s = encode("3141").unwrap();
        assert_eq!(s.words.len(), 2);
        assert!(matches!(s.words[0].code, Code::Pair(31)));
        assert!(matches!(s.words[1].code, Code::Pair(41)));
        assert_eq!(s.digit_len, 4);
    }

    #[test]
    fn odd_length_ends_in_single_verb() {
        let s = encode("314").unwrap();
        assert_eq!(s.words.len(), 2);
        assert!(matches!(s.words[0].code, Code::Pair(31)));
        assert!(matches!(s.words[1].code, Code::Single(4)));
        // The trailing single is always rendered as a verb.
        assert_eq!(s.words[1].pos, PartOfSpeech::Verb);
    }

    #[test]
    fn lone_single_digit_is_a_verb() {
        let s = encode("7").unwrap();
        assert_eq!(s.words.len(), 1);
        assert!(matches!(s.words[0].code, Code::Single(7)));
        assert_eq!(s.words[0].pos, PartOfSpeech::Verb);
    }

    #[test]
    fn assigns_pos_by_clause_structure() {
        use PartOfSpeech::*;
        // 3 pairs -> one full clause.
        let s = encode("000000").unwrap();
        assert_eq!(
            s.words.iter().map(|w| w.pos).collect::<Vec<_>>(),
            vec![Verb, Adjective, Noun]
        );
        // 2 pairs -> verb + noun (no dangling adjective).
        let s = encode("0000").unwrap();
        assert_eq!(
            s.words.iter().map(|w| w.pos).collect::<Vec<_>>(),
            vec![Verb, Noun]
        );
        // 1 pair -> lone noun.
        let s = encode("00").unwrap();
        assert_eq!(s.words[0].pos, Noun);
        // 4 pairs -> full clause + lone noun.
        let s = encode("00000000").unwrap();
        assert_eq!(
            s.words.iter().map(|w| w.pos).collect::<Vec<_>>(),
            vec![Verb, Adjective, Noun, Noun]
        );
    }

    #[test]
    fn preserves_leading_zeros_in_digits() {
        assert_eq!(encode("05").unwrap().to_digits(), "05");
        assert_eq!(encode("007").unwrap().to_digits(), "007");
        assert_eq!(encode("0").unwrap().to_digits(), "0");
    }
}
