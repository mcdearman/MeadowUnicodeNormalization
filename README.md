# unicodeNormalization

The Unicode Normalization Forms of
[UAX #15](https://www.unicode.org/reports/tr15/) (NFD, NFC, NFKD and NFKC) for
[Meadow](https://github.com/mcdearman/meadow). Also includes quick checks,
stream-safe text, and CJK compatibility variants.

This package is a port of Rust's
[`unicode-normalization`](https://github.com/unicode-rs/unicode-normalization)
0.1.25, covering Unicode 17.0.0.

Normalize text before comparing, searching or storing it. `"é"` can be written
as one code point (U+00E9) or as two (`e` followed by U+0301), and the two
spellings only compare equal once both are in the same form.

## Install

```sh
meadow add mcdearman/meadow-unicode-normalization
```

## Use

```meadow
use unicodeNormalization (nfc, nfd, nfkc, isNfc)

def main =
  ( nfc "e\u{301}" == "\u{e9}",        -- True: composed
    nfd "\u{e9}" == "e\u{301}",        -- True: decomposed
    nfkc "\u{fb01}ne \u{2460}",       -- "fine 1": compatibility forms folded
    isNfc "e\u{301}"                   -- False
  )
```

| function | |
|---|---|
| `nfd s`, `nfc s` | canonical decomposition, and decomposition followed by composition |
| `nfkd s`, `nfkc s` | the same with compatibility decomposition, which folds ligatures, circled digits, full-width forms and the like |
| `isNfc s`, `isNfd s`, `isNfkc s`, `isNfkd s` | whether `s` is already in that form |
| `isNfcQuick s` … | `Yes`, `No` or `Maybe`, without normalizing `s` |
| `streamSafe s` | inserts U+034F wherever more than 30 non-starters would otherwise occur in a row (UAX #15's Stream-Safe Text Format) |
| `isNfcStreamSafe s`, `isNfdStreamSafe s` | whether `s` is both normalized and stream-safe (the matching `…Quick` checks also exist) |
| `cjkCompatVariants s` | replaces CJK compatibility ideographs with their standardized variation sequences, which keep them distinct under normalization |
| `canonicalCombiningClass c` | the canonical combining class of a character (0 for a starter) |
| `compose a b` | the primary composite of two characters, if they have one |
| `decomposeCanonical c`, `decomposeCompatible c`, `decomposeCjkCompatVariants c` | a character's full decomposition |
| `isCombiningMark c`, `isPublicAssigned c` | character properties |
| `unicodeVersion` | `(17, 0, 0)` |

The crate's normalizers are iterators, so they can run on text as it streams
in. These functions take and return whole strings.

## How it's made

- **`src/Tables.mw`** is generated from the crate's public functions, called on
  every code point. It holds combining classes, decompositions, composites,
  quick-check properties and non-starter counts. The generator checks that the
  composites it finds are the crate's entire composition table.
- **`src/Normalize.mw`** translates the crate's algorithms by hand.
- **`src/Cases.mw`** is generated test data:
  - 7,626 strings, 4,011 of them from every tenth line of the official
    `NormalizationTest.txt` and the rest random mixtures of every kind of
    character the algorithms treat differently;
  - 4,704 characters;
  - 3,400 pairs of characters to compose, including every pair of Hangul edge
    cases.

  For each string, `meadow test` checks all six transformations, all six quick
  checks and all six `is…` functions against the crate's own answers.

To regenerate, run `scripts/generate.sh`. It needs a Rust toolchain, and
downloads `NormalizationTest.txt` from unicode.org the first time. If the
crate's algorithms changed, the generator stops so that `src/Normalize.mw` can
be updated first.

## Licence

Dual-licensed under [Apache-2.0](LICENSE-APACHE) or [MIT](LICENSE-MIT), at your
option, like the crate. See [COPYRIGHT](COPYRIGHT).
