//! CLI integration tests: behavior, stdin, and the exit-code contract.

use assert_cmd::Command;
use predicates::prelude::*;

fn mnemonic() -> Command {
    Command::cargo_bin("mnemonic").unwrap()
}

#[test]
fn encode_then_decode_roundtrips() {
    let out = mnemonic()
        .args(["encode", "314159"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let sentence = String::from_utf8(out).unwrap();

    mnemonic()
        .args(["decode", sentence.trim()])
        .assert()
        .success()
        .stdout("314159\n");
}

#[test]
fn encode_reads_stdin() {
    mnemonic()
        .arg("encode")
        .write_stdin("42\n")
        .assert()
        .success()
        .stdout(predicate::str::is_empty().not());
}

#[test]
fn decode_reads_stdin() {
    let sentence = mnemonic()
        .args(["encode", "--format", "plain", "42"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let sentence = String::from_utf8(sentence).unwrap();

    mnemonic()
        .arg("decode")
        .write_stdin(sentence)
        .assert()
        .success()
        .stdout("42\n");
}

#[test]
fn encode_plain_format_is_lowercase_words() {
    mnemonic()
        .args(["encode", "--format", "plain", "3141"])
        .assert()
        .success()
        .stdout(predicate::str::is_match(r"^[a-z]+ [a-z]+\n$").unwrap());
}

#[test]
fn encode_json_format_emits_words_array() {
    mnemonic()
        .args(["encode", "--format", "json", "31"])
        .assert()
        .success()
        .stdout(predicate::str::contains("\"words\""))
        .stdout(predicate::str::contains("\"code\": \"31\""));
}

#[test]
fn validate_succeeds_on_embedded_data() {
    mnemonic().arg("validate").assert().success();
}

#[test]
fn encode_rejects_non_digit_with_exit_2() {
    mnemonic()
        .args(["encode", "12x3"])
        .assert()
        .code(2)
        .stderr(predicate::str::contains("invalid character"));
}

#[test]
fn train_scores_correct_answers() {
    // Seed 7 with 2-digit rounds deterministically yields 40, 13, 01.
    mnemonic()
        .args(["train", "--digits", "2", "--rounds", "3", "--seed", "7"])
        .write_stdin("40\n13\n01\n")
        .assert()
        .success()
        .stdout(predicate::str::contains("Score: 3/3 (100%)"));
}

#[test]
fn train_scores_wrong_answers() {
    mnemonic()
        .args(["train", "--digits", "2", "--rounds", "3", "--seed", "7"])
        .write_stdin("00\n00\n00\n")
        .assert()
        .success()
        .stdout(predicate::str::contains("Score: 0/3 (0%)"));
}

#[test]
fn train_encode_accepts_alternative_words() {
    // Seed 7 yields 40, 13, 01. Answer with all single-digit words instead of
    // the canonical pair words; they encode the same numbers, so they count.
    mnemonic()
        .args([
            "train", "--digits", "2", "--rounds", "3", "--seed", "7", "--mode", "encode",
        ])
        .write_stdin("row sew\nhate aim\nsew hate\n")
        .assert()
        .success()
        .stdout(predicate::str::contains("Score: 3/3 (100%)"));
}

#[test]
fn train_rejects_zero_rounds_with_exit_2() {
    mnemonic()
        .args(["train", "--rounds", "0"])
        .assert()
        .code(2)
        .stderr(predicate::str::contains("at least 1"));
}

#[test]
fn decode_rejects_unknown_word_with_exit_3() {
    mnemonic()
        .args(["decode", "zzqzz"])
        .assert()
        .code(3)
        .stderr(predicate::str::contains("unknown word"));
}
