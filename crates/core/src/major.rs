//! Spelling-based Major System encoder.
//!
//! This maps a word's *spelling* to the digit sequence implied by the Major
//! System consonant sounds. It is used **only** for build-time / test-time
//! validation of the peg data — never for the encode/decode bijection, whose
//! source of truth is the explicit `digits` field stored per word.
//!
//! English spelling is irregular (silent letters, soft/hard `c`/`g`, digraphs
//! pronounced two ways), so this is a best-effort typo-catcher. Words whose
//! pronunciation legitimately disagrees with this heuristic are marked with
//! `phonetic_exception` in the data file.
//!
//! Mapping: 0=`s`/`z`, 1=`t`/`d`, 2=`n`, 3=`m`, 4=`r`, 5=`l`, 6=`ch`/`sh`/`j`,
//! 7=`k`/hard `c`/hard `g`/`q`, 8=`f`/`v`, 9=`p`/`b`. Vowels and `w`/`h`/`y` are
//! non-encoding fillers; doubled consonants making one sound count once.

/// The single-letter consonant classes that are context-free.
fn letter_digit(c: char) -> Option<u8> {
    Some(match c {
        's' | 'z' => 0,
        't' | 'd' => 1,
        'n' => 2,
        'm' => 3,
        'r' => 4,
        'l' => 5,
        'j' => 6,
        'k' | 'q' => 7,
        'f' | 'v' => 8,
        'p' | 'b' => 9,
        _ => return None,
    })
}

/// Whether the following letter makes a preceding `c`/`g` "soft".
fn is_front_vowel(c: Option<char>) -> bool {
    matches!(c, Some('e') | Some('i') | Some('y'))
}

/// Encode a word's spelling to the Major System digit sequence it implies.
///
/// Non-letters are ignored; the result is the sequence of encoded consonant
/// sounds in order.
pub fn major_encode(word: &str) -> Vec<u8> {
    let chars: Vec<char> = word
        .chars()
        .filter(|c| c.is_ascii_alphabetic())
        .map(|c| c.to_ascii_lowercase())
        .collect();

    // Push `digit` (sourced from letters spanning `start..=end`), collapsing a
    // doubled consonant / single-sound cluster: an identical digit produced by
    // the immediately adjacent preceding letter is folded into one.
    fn push(
        out: &mut Vec<u8>,
        last: &mut Option<(u8, usize)>,
        digit: u8,
        start: usize,
        end: usize,
    ) {
        if let Some((prev_digit, prev_end)) = *last {
            if prev_digit == digit && start == prev_end + 1 {
                *last = Some((digit, end));
                return;
            }
        }
        out.push(digit);
        *last = Some((digit, end));
    }

    let mut out = Vec::new();
    let mut last: Option<(u8, usize)> = None;
    let mut i = 0;
    while i < chars.len() {
        let c = chars[i];
        let next = chars.get(i + 1).copied();
        match (c, next) {
            // `tch` => /tʃ/ => single 6 (the t is absorbed): match, switch.
            ('t', Some('c')) if chars.get(i + 2) == Some(&'h') => {
                push(&mut out, &mut last, 6, i, i + 2);
                i += 3;
            }
            ('c', Some('h')) | ('s', Some('h')) => {
                push(&mut out, &mut last, 6, i, i + 1);
                i += 2;
            }
            ('t', Some('h')) => {
                push(&mut out, &mut last, 1, i, i + 1);
                i += 2;
            }
            ('p', Some('h')) => {
                push(&mut out, &mut last, 8, i, i + 1);
                i += 2;
            }
            ('c', Some('k')) => {
                push(&mut out, &mut last, 7, i, i + 1);
                i += 2;
            }
            ('x', _) => {
                // x => /ks/ => 7 then 0
                push(&mut out, &mut last, 7, i, i);
                push(&mut out, &mut last, 0, i, i);
                i += 1;
            }
            ('c', _) => {
                let d = if is_front_vowel(next) { 0 } else { 7 };
                push(&mut out, &mut last, d, i, i);
                i += 1;
            }
            ('g', _) => {
                let d = if is_front_vowel(next) { 6 } else { 7 };
                push(&mut out, &mut last, d, i, i);
                i += 1;
            }
            _ => {
                if let Some(d) = letter_digit(c) {
                    push(&mut out, &mut last, d, i, i);
                }
                i += 1;
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::major_encode;

    #[test]
    fn single_consonant_classes() {
        assert_eq!(major_encode("sea"), vec![0]); // s
        assert_eq!(major_encode("zoo"), vec![0]); // z
        assert_eq!(major_encode("tea"), vec![1]); // t
        assert_eq!(major_encode("day"), vec![1]); // d
        assert_eq!(major_encode("hen"), vec![2]); // n (h filler)
        assert_eq!(major_encode("ma"), vec![3]); // m
        assert_eq!(major_encode("row"), vec![4]); // r (w filler)
        assert_eq!(major_encode("law"), vec![5]); // l
        assert_eq!(major_encode("jaw"), vec![6]); // j
        assert_eq!(major_encode("key"), vec![7]); // k (y filler)
        assert_eq!(major_encode("ivy"), vec![8]); // v
        assert_eq!(major_encode("bee"), vec![9]); // b
    }

    #[test]
    fn digraphs() {
        assert_eq!(major_encode("ship"), vec![6, 9]); // sh, p
        assert_eq!(major_encode("chip"), vec![6, 9]); // ch, p
        assert_eq!(major_encode("thaw"), vec![1]); // th
        assert_eq!(major_encode("phone"), vec![8, 2]); // ph, n
    }

    #[test]
    fn hard_and_soft_c() {
        assert_eq!(major_encode("cat"), vec![7, 1]); // hard c
        assert_eq!(major_encode("cell"), vec![0, 5]); // soft c, ll -> one 5
        assert_eq!(major_encode("ace"), vec![0]); // soft c
    }

    #[test]
    fn hard_and_soft_g() {
        assert_eq!(major_encode("gum"), vec![7, 3]); // hard g
        assert_eq!(major_encode("gel"), vec![6, 5]); // soft g
    }

    #[test]
    fn ck_and_x_clusters() {
        assert_eq!(major_encode("rock"), vec![4, 7]); // ck -> single 7
        assert_eq!(major_encode("fox"), vec![8, 7, 0]); // x -> 7,0
    }

    #[test]
    fn tch_cluster_is_single_six() {
        assert_eq!(major_encode("match"), vec![3, 6]); // m, tch
        assert_eq!(major_encode("switch"), vec![0, 6]); // s, w, i, tch
        assert_eq!(major_encode("fetch"), vec![8, 6]);
    }

    #[test]
    fn doubled_letters_collapse_but_split_by_vowel() {
        assert_eq!(major_encode("butter"), vec![9, 1, 4]); // tt -> one 1
        assert_eq!(major_encode("mom"), vec![3, 3]); // m..m split by vowel -> two
    }

    #[test]
    fn fillers_only_is_empty() {
        assert_eq!(major_encode("hwy"), Vec::<u8>::new());
        assert_eq!(major_encode("aeiou"), Vec::<u8>::new());
    }
}
