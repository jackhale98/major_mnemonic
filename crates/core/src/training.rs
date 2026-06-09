//! Training-mode helpers: build practice challenges and grade answers.
//!
//! This module is pure and RNG-free. Front ends generate the random number to
//! practice and own the timing/scoring loop; the core just turns a number into
//! a challenge and grades a learner's answer, so the logic is shared and tested
//! in one place.

use crate::error::EncodeError;

/// Which way the learner is practicing.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Direction {
    /// Shown a mnemonic sentence, recall the number. The common drill.
    WordsToNumber,
    /// Shown a number, recall a mnemonic sentence. For advanced learners.
    NumberToWords,
}

/// One practice item: a target number paired with its canonical mnemonic.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Challenge {
    /// The target digit string.
    pub number: String,
    /// The canonical mnemonic for `number`.
    pub sentence: String,
}

impl Challenge {
    /// Build a challenge for `number`. Fails only if `number` is not a valid
    /// digit string (same rules as [`crate::encode`]).
    pub fn new(number: impl Into<String>) -> Result<Self, EncodeError> {
        let number = number.into();
        let sentence = crate::encode(&number)?.to_pretty();
        Ok(Self { number, sentence })
    }

    /// The text shown to the learner for `direction`.
    pub fn prompt(&self, direction: Direction) -> &str {
        match direction {
            Direction::WordsToNumber => &self.sentence,
            Direction::NumberToWords => &self.number,
        }
    }

    /// The canonical expected answer, e.g. to reveal after a miss.
    pub fn answer(&self, direction: Direction) -> &str {
        match direction {
            Direction::WordsToNumber => &self.number,
            Direction::NumberToWords => &self.sentence,
        }
    }

    /// Grade a learner's `answer` for `direction`.
    ///
    /// For `WordsToNumber`, any non-digit characters in the answer are ignored
    /// (so spaces or separators are fine). For `NumberToWords`, *any* peg words
    /// that decode to the target count as correct — not just the canonical
    /// sentence — since decoding is position-independent.
    pub fn grade(&self, direction: Direction, answer: &str) -> bool {
        match direction {
            Direction::WordsToNumber => digits_only(answer) == self.number,
            Direction::NumberToWords => {
                crate::decode(answer).map(|d| d == self.number).unwrap_or(false)
            }
        }
    }

    /// Grade `answer` and capture everything needed to review the round later.
    pub fn review(&self, direction: Direction, answer: &str) -> RoundOutcome {
        RoundOutcome {
            prompt: self.prompt(direction).to_string(),
            guess: answer.trim().to_string(),
            expected: self.answer(direction).to_string(),
            correct: self.grade(direction, answer),
        }
    }
}

/// A graded round, kept so the learner can review the whole session at the end.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RoundOutcome {
    /// What the learner was shown (a sentence, or a number).
    pub prompt: String,
    /// What the learner answered, trimmed of surrounding whitespace.
    pub guess: String,
    /// The program's own canonical answer.
    pub expected: String,
    /// Whether the guess was accepted.
    pub correct: bool,
}

fn digits_only(s: &str) -> String {
    s.chars().filter(char::is_ascii_digit).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn challenge_pairs_number_with_its_mnemonic() {
        let c = Challenge::new("31").unwrap();
        assert_eq!(c.number, "31");
        assert_eq!(c.sentence, crate::encode("31").unwrap().to_pretty());
    }

    #[test]
    fn new_rejects_non_digits() {
        assert!(Challenge::new("12x").is_err());
    }

    #[test]
    fn grades_words_to_number() {
        let c = Challenge::new("3142").unwrap();
        assert!(c.grade(Direction::WordsToNumber, "3142"));
        // Separators and spaces in the answer are ignored.
        assert!(c.grade(Direction::WordsToNumber, "31 42"));
        assert!(!c.grade(Direction::WordsToNumber, "3141"));
    }

    #[test]
    fn grades_number_to_words_accepting_any_valid_mnemonic() {
        let c = Challenge::new("314159").unwrap();
        // The canonical sentence is accepted...
        assert!(c.grade(Direction::NumberToWords, &c.sentence));
        // ...and so is the plain rendering (different punctuation/case).
        let plain = crate::encode("314159").unwrap().to_plain();
        assert!(c.grade(Direction::NumberToWords, &plain));
        // Gibberish is not.
        assert!(!c.grade(Direction::NumberToWords, "zzqzz"));
        // A sentence for a different number is not.
        let other = crate::encode("314158").unwrap().to_pretty();
        assert!(!c.grade(Direction::NumberToWords, &other));
    }

    #[test]
    fn review_captures_prompt_guess_and_answer() {
        let c = Challenge::new("3142").unwrap();
        let hit = c.review(Direction::WordsToNumber, " 3142 ");
        assert!(hit.correct);
        assert_eq!(hit.prompt, c.sentence);
        assert_eq!(hit.guess, "3142");
        assert_eq!(hit.expected, "3142");

        let miss = c.review(Direction::WordsToNumber, "9999");
        assert!(!miss.correct);
        assert_eq!(miss.guess, "9999");
        assert_eq!(miss.expected, "3142");
    }

    #[test]
    fn number_to_words_accepts_all_single_digit_words() {
        // "40" canonically encodes to one pair word, but a learner may spell it
        // out as two single-digit words; both must be accepted.
        let c = Challenge::new("40").unwrap();
        let four = crate::encode("4").unwrap().to_plain();
        let zero = crate::encode("0").unwrap().to_plain();
        let spelled_out = format!("{four} {zero}");
        assert_ne!(spelled_out, c.sentence.to_lowercase());
        assert!(c.grade(Direction::NumberToWords, &spelled_out));
    }
}
