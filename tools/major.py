"""Python port of crates/core/src/major.rs `major_encode`.

MUST stay byte-for-byte equivalent in behavior to the Rust encoder so that the
generated data passes the Rust soft-validation gate.
"""
from __future__ import annotations

_LETTER_DIGIT = {
    "s": 0, "z": 0,
    "t": 1, "d": 1,
    "n": 2,
    "m": 3,
    "r": 4,
    "l": 5,
    "j": 6,
    "k": 7, "q": 7,
    "f": 8, "v": 8,
    "p": 9, "b": 9,
}


def _is_front_vowel(c: str | None) -> bool:
    return c in ("e", "i", "y")


def major_encode(word: str) -> list[int]:
    chars = [c.lower() for c in word if c.isascii() and c.isalpha()]
    out: list[int] = []
    last: tuple[int, int] | None = None  # (digit, end_index)

    def push(digit: int, start: int, end: int) -> None:
        nonlocal last
        if last is not None:
            prev_digit, prev_end = last
            if prev_digit == digit and start == prev_end + 1:
                last = (digit, end)
                return
        out.append(digit)
        last = (digit, end)

    i = 0
    n = len(chars)
    while i < n:
        c = chars[i]
        nxt = chars[i + 1] if i + 1 < n else None
        nxt2 = chars[i + 2] if i + 2 < n else None
        if c == "t" and nxt == "c" and nxt2 == "h":
            push(6, i, i + 2); i += 3  # tch -> /tʃ/ -> single 6
        elif (c == "c" and nxt == "h") or (c == "s" and nxt == "h"):
            push(6, i, i + 1); i += 2
        elif c == "t" and nxt == "h":
            push(1, i, i + 1); i += 2
        elif c == "p" and nxt == "h":
            push(8, i, i + 1); i += 2
        elif c == "c" and nxt == "k":
            push(7, i, i + 1); i += 2
        elif c == "x":
            push(7, i, i); push(0, i, i); i += 1
        elif c == "c":
            push(0 if _is_front_vowel(nxt) else 7, i, i); i += 1
        elif c == "g":
            push(6 if _is_front_vowel(nxt) else 7, i, i); i += 1
        else:
            d = _LETTER_DIGIT.get(c)
            if d is not None:
                push(d, i, i)
            i += 1
    return out


if __name__ == "__main__":
    # Self-check mirroring the Rust unit tests.
    checks = {
        "sea": [0], "zoo": [0], "tea": [1], "day": [1], "hen": [2], "ma": [3],
        "row": [4], "law": [5], "jaw": [6], "key": [7], "ivy": [8], "bee": [9],
        "ship": [6, 9], "chip": [6, 9], "thaw": [1], "phone": [8, 2],
        "cat": [7, 1], "cell": [0, 5], "ace": [0], "gum": [7, 3], "gel": [6, 5],
        "rock": [4, 7], "fox": [8, 7, 0], "butter": [9, 1, 4], "mom": [3, 3],
        "match": [3, 6], "switch": [0, 6], "fetch": [8, 6],
        "hwy": [], "aeiou": [],
    }
    bad = {w: (major_encode(w), exp) for w, exp in checks.items() if major_encode(w) != exp}
    if bad:
        raise SystemExit(f"encoder self-check FAILED: {bad}")
    print("major.py self-check OK")
