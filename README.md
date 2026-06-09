# major_mnemonic

A reversible implementation of the [Mnemonic Major System](https://en.wikipedia.org/wiki/Mnemonic_major_system).
Enter a number, get a memorable sentence back — and decode that sentence back into
the exact original number.

```
$ mnemonic 314159265
Meet ready lip, enjoy heal.

$ mnemonic decode "Meet ready lip, enjoy heal."
314159265
```

A bare number is shorthand for `encode` — `mnemonic 314159265` and
`mnemonic encode 314159265` are equivalent.

## Install

```
cargo install mnemonic                  # from crates.io (once published)
```

Or install the latest straight from the repo:

```
cargo install --git https://github.com/jackhale/major_mnemonic mnemonic
```

Either way you get a `mnemonic` binary in `~/.cargo/bin`. To install a local
checkout, run `cargo install --path crates/cli`.

## How it works

**Encoding** splits the number into two-digit pairs (`00`–`99`). Each pair maps to
one word, and words are assigned a part of speech by their position, cycling
**verb → adjective → noun**, so three pairs read as a clause like *"meet ready lip"*.
An **odd-length** number ends with a leftover single digit, which is rendered as a
single-consonant **verb** as the final word (e.g. `007` → "Assess hook.") — this
tends to read more naturally than a trailing lone noun.

**Decoding** is **position-independent**: every word is globally unique across the
peg list, so each word is looked up on its own and the codes are concatenated in
reading order. You can hand-build a sentence from any peg words, in any grammatical
arrangement, and it decodes. Reading order still determines the *digit* order,
though — `meet row` is `314` while `row meet` is `431`.

The round trip is exact, including **leading zeros** (`007` → "Assess hook." →
`007`) and odd-length numbers.

Major System digit ↔ sound: `0`=s/z · `1`=t/d · `2`=n · `3`=m · `4`=r · `5`=l ·
`6`=ch/sh/j · `7`=k/hard-c/g/q · `8`=f/v · `9`=p/b. Vowels and w/h/y are fillers.

## Architecture

A Cargo workspace built so the logic can be reused across every target platform:

```
crates/
  core/   major_mnemonic_core — pure library: encode/decode/validate + embedded
          peg data. NO UI/CLI/FFI dependencies, so it drops straight into the
          GUI and mobile (UniFFI) front ends.
  cli/    mnemonic            — the `mnemonic` binary (package name `mnemonic`).
  gui/    mnemonic-gui        — small native desktop GUI (egui/eframe) over core;
          encodes live as you type, plus a timed training drill.
data:     crates/core/data/pegs.v1.json — versioned word data (the source of truth
          for the bijection). Each word stores its `digits`, validated against the
          array index and global word uniqueness (hard gates) and a phonetic
          encoder (soft typo-catcher). Global uniqueness is what lets decoding be
          position-independent.
tools/    Python generator for the peg data + a port of the phonetic encoder.
```

## Usage

```
mnemonic <DIGITS>          [--format pretty|plain|json]   # shorthand for `encode`
mnemonic encode <DIGITS>   [--format pretty|plain|json]   # omit DIGITS to read stdin
mnemonic decode <SENTENCE>                                # omit SENTENCE to read stdin
mnemonic train             [--digits N] [--rounds N] [--mode decode|encode]
mnemonic validate                                         # check the embedded data
```

Exit codes: `0` ok · `2` bad/empty encode input · `3` undecodable sentence ·
`4` data validation failure.

### Training

`mnemonic train` runs an interactive drill: each round shows a prompt, you type
your answer and press Enter, and a score plus timing print at the end.

- `--digits N` — how many digits each round's number has (default 4; use `2` to
  drill the individual pegs).
- `--rounds N` — how many rounds to play (default 10).
- `--mode decode` (default) shows the **words** and you recall the **number**;
  `--mode encode` shows the **number** and you recall the **words** (any words
  that encode it are accepted). The encode direction is for once you know the
  pegs well.

The same drill is available in the desktop GUI's **Train** tab, with a live
timer.

## Development

```
cargo test --workspace        # unit + integration + round-trip property tests
cargo clippy --workspace      # lints
cargo run -p mnemonic -- encode 1234567890
cargo run -p mnemonic-gui     # launch the desktop GUI
```

### Regenerating the peg data

The word lists in `tools/generate_pegs.py` are the authoritative published pegs
(nouns, verbs, adjectives for `00`–`99`, plus single-digit pegs `0`–`9`). Words
are normalized to single ASCII tokens; those whose spelling diverges from the
heuristic encoder (silent letters, soft/hard `g`) are flagged
`phonetic_exception` rather than altered. After editing the lists, regenerate
and re-test:

```
python tools/major.py            # self-check the encoder port
python tools/generate_pegs.py    # rewrites crates/core/data/pegs.v1.json
cargo test -p major_mnemonic_core
```

## Roadmap

- [x] Reversible core library (encode/decode/validate) with full test coverage
- [x] CLI front end
- [x] Authoritative published peg lists (incl. single-digit pegs)
- [x] Desktop GUI prototype (egui/eframe) over `core` — live encode as you type
- [x] Training mode (CLI + GUI) — timed, scored peg drills both directions
- [ ] Mobile bindings (UniFFI) + iOS/Android shells

### Known limitation

Strict reversibility means no inserted glue words ("the", "a"), so output is
clause-structured (`verb adjective noun, …`) rather than fully natural prose. A
future version could add a guaranteed-disjoint stopword set that decode skips.
