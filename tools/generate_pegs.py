"""Generate crates/core/data/pegs.v1.json from the published Major System pegs.

Word lists are the authoritative pegs from the Mnemonic Major System (Wikipedia),
including the single-digit pegs used for odd-length numbers. Words are normalized
to single lowercase ASCII tokens. Because the source is authoritative, any word
whose *spelling* disagrees with our heuristic phonetic encoder (silent letters,
`tch`, soft/hard g, etc.) is flagged `phonetic_exception` rather than altered.

Run:  python tools/generate_pegs.py
"""
from __future__ import annotations

import json
import os
import re
import unicodedata

from major import major_encode

CONS = {0: "s", 1: "t", 2: "n", 3: "m", 4: "r", 5: "l", 6: "j", 7: "k", 8: "f", 9: "p"}
PAIR_VOWELS = {
    "verb": [("a", "e"), ("o", "e"), ("a", "o"), ("e", "o"), ("u", "e")],
    "adjective": [("o", "a"), ("i", "a"), ("u", "a"), ("e", "a"), ("o", "i")],
    "noun": [("i", "o"), ("e", "o"), ("a", "i"), ("u", "o"), ("i", "e")],
}

# --- Authoritative pegs (00-99), row order = noun, verb, adjective. -----------

NOUNS = [
    "sauce", "seed", "sun", "sumo", "sierra", "soil", "sewage", "sky", "sofa", "soap",
    "daisy", "tattoo", "tuna", "dome", "diary", "tail", "dish", "dog", "dove", "tuba",
    "nose", "net", "onion", "enemy", "winery", "nail", "nacho", "neck", "knife", "honeybee",
    "mouse", "meadow", "moon", "mummy", "emery", "mole", "match", "mug", "movie", "map",
    "rice", "road", "rain", "rum", "aurora", "railway", "roach", "rag", "roof", "rope",
    "louse", "lady", "lion", "lime", "lorry", "lily", "leech", "leg", "lava", "lip",
    "cheese", "cheetah", "chin", "gem", "shrew", "chilli", "cha-cha", "chick", "chef", "jeep",
    "goose", "cat", "coin", "game", "crow", "clay", "cage", "cake", "cave", "cube",
    "vase", "video", "fan", "ovum", "fairy", "fool", "veggie", "fig", "fife", "vibe",
    "boss", "bead", "pony", "puma", "berry", "bell", "pouch", "bike", "beef", "pipe",
]

VERBS = [
    "assess", "swat", "assign", "assume", "sorrow", "sell", "switch", "soak", "save", "sob",
    "tease", "edit", "widen", "time", "draw", "tell", "teach", "take", "defy", "type",
    "ionize", "unite", "nanny", "name", "honour", "inhale", "enjoy", "knock", "envy", "nab",
    "amuse", "meet", "mine", "mime", "marry", "mail", "mash", "mock", "move", "mop",
    "erase", "read", "ruin", "ram", "rear", "rule", "reach", "rake", "arrive", "wrap",
    "lose", "let", "align", "loom", "lure", "lull", "latch", "lick", "love", "help",
    "chase", "cheat", "chain", "jam", "jury", "chill", "judge", "check", "achieve", "chop",
    "kiss", "quote", "weaken", "comb", "carry", "kill", "coach", "cook", "give", "copy",
    "fuse", "fight", "fine", "fume", "fry", "fly", "fetch", "fake", "viva", "fob",
    "oppose", "bite", "ban", "bomb", "bury", "peel", "patch", "poke", "pave", "pop",
]

ADJECTIVES = [
    "sissy", "sad", "snowy", "awesome", "sorry", "slow", "swishy", "sick", "savvy", "sappy",
    "dizzy", "tight", "wooden", "tame", "dry", "tall", "whitish", "thick", "deaf", "deep",
    "noisy", "neat", "neon", "numb", "narrow", "annual", "nudgy", "naggy", "naïve", "wannabe",
    "messy", "mute", "mean", "mum", "merry", "male", "mushy", "mucky", "mauve", "wimpy",
    "rosy", "ready", "runny", "haram", "rare", "royal", "rich", "rocky", "rough", "ripe",
    "lazy", "elite", "alien", "lame", "leery", "loyal", "lush", "lucky", "leafy", "loopy",
    "choosy", "chatty", "shiny", "sham", "cherry", "jolly", "jewish", "shaky", "chief", "cheap",
    "cozy", "good", "keen", "gummy", "grey", "cool", "catchy", "quick", "goofy", "agape",
    "fussy", "fat", "funny", "foamy", "furry", "foul", "fishy", "foggy", "fave", "fab",
    "busy", "bad", "bony", "balmy", "pro", "blue", "bushy", "back", "puffy", "baby",
]

SINGLES = {
    "noun": ["hose", "hat", "hen", "home", "arrow", "whale", "shoe", "cow", "hoof", "pie"],
    "verb": ["sew", "hate", "know", "aim", "row", "heal", "chew", "hook", "view", "buy"],
    "adjective": ["easy", "hot", "new", "yummy", "hairy", "oily", "itchy", "gay", "heavy", "happy"],
}

PAIRS = {"noun": NOUNS, "verb": VERBS, "adjective": ADJECTIVES}


def normalize(word: str) -> str:
    """Strip footnote markers, fold accents, drop non-letters, lowercase."""
    word = re.sub(r"\[[a-z]\]", "", word)  # footnote markers like [a]
    word = unicodedata.normalize("NFKD", word).encode("ascii", "ignore").decode()
    word = re.sub(r"[^A-Za-z]", "", word)  # hyphens, spaces, etc.
    return word.lower()


def synth_pair(pos: str, code: int, used: set[str]) -> str:
    a, b = code // 10, code % 10
    for v1, v2 in PAIR_VOWELS[pos]:
        w = f"{CONS[a]}{v1}{CONS[b]}{v2}"
        if w not in used and major_encode(w) == [a, b]:
            return w
    base, w, k = f"{CONS[a]}a{CONS[b]}o", f"{CONS[a]}a{CONS[b]}o", 0
    while w in used:
        k += 1
        w = base + "o" * k
    return w


def make_entry(pos: str, word: str, digits: list[int], used: set[str],
               notes: list[str], where: str) -> dict:
    norm = normalize(word)
    if not norm or norm in used:
        reason = "empty after normalize" if not norm else f"duplicate {norm!r}"
        notes.append(f"{where}: {word!r} {reason}; SYNTHESIZED fallback")
        a, b = (digits + [0])[0], (digits + [0])[1] if len(digits) == 2 else digits[0]
        norm = synth_pair(pos, digits[0] * 10 + digits[1], used) if len(digits) == 2 else \
            next(w for w in (CONS[digits[0]] + v for v in "eou") if w not in used)
    used.add(norm)
    entry = {"word": norm, "digits": digits}
    if major_encode(norm) != digits:
        entry["phonetic_exception"] = True
        notes.append(f"{where}: {norm!r} encodes {major_encode(norm)} != {digits} "
                     f"(flagged phonetic_exception)")
    return entry


def build_pos(pos: str, notes: list[str]) -> tuple[list[dict], list[dict]]:
    used: set[str] = set()
    singles = [
        make_entry(pos, SINGLES[pos][d], [d], used, notes, f"{pos} single[{d}]")
        for d in range(10)
    ]
    pairs = [
        make_entry(pos, PAIRS[pos][code], [code // 10, code % 10], used, notes,
                   f"{pos} pair[{code:02d}]")
        for code in range(100)
    ]
    return pairs, singles


def main() -> None:
    here = os.path.dirname(os.path.abspath(__file__))
    out_path = os.path.normpath(
        os.path.join(here, "..", "crates", "core", "data", "pegs.v1.json")
    )
    os.makedirs(os.path.dirname(out_path), exist_ok=True)

    data = {
        "schema_version": 1,
        "source": ("Authoritative pegs from the Mnemonic Major System (Wikipedia), "
                   "incl. single-digit pegs. Normalized to single ASCII tokens. "
                   "phonetic_exception marks words whose spelling diverges from the "
                   "heuristic encoder (silent letters, tch, soft/hard g)."),
        "pairs": {},
        "singles": {},
    }
    notes: list[str] = []
    exceptions = 0
    for pos in ("verb", "adjective", "noun"):
        pairs, singles = build_pos(pos, notes)
        data["pairs"][pos] = pairs
        data["singles"][pos] = singles
        exceptions += sum(1 for e in pairs + singles if e.get("phonetic_exception"))

    with open(out_path, "w", encoding="utf-8") as f:
        json.dump(data, f, indent=2, ensure_ascii=True)
        f.write("\n")

    print(f"wrote {out_path}")
    print(f"{exceptions} phonetic_exception flags across {3 * 110} words")
    for n in notes:
        print("  -", n)


if __name__ == "__main__":
    main()
