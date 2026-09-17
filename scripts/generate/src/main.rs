//! Writes `src/Tables.mw` and `src/Cases.mw` from `unicode-normalization`.
//!
//! ```text
//! cargo run --release -- <package root> <NormalizationTest.txt>
//! ```
//!
//! * **Tables** -- every per-character fact the crate's algorithms consult,
//!   read off its public functions for every code point: combining classes,
//!   full canonical, compatibility and CJK-variant decompositions, the
//!   primary composites, the quick-check properties, and the leading and
//!   trailing non-starter counts for stream-safe text.
//! * **Cases** -- strings with what the crate makes of them, from the official
//!   `NormalizationTest.txt` and at random, and characters with what the
//!   crate's `char` functions say of them.
//!
//! The algorithms are ported by hand into `src/`, and their source is
//! fingerprinted.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Write as _;
use std::path::PathBuf;
use unicode_normalization::char::{
    canonical_combining_class, compose, decompose_canonical, decompose_cjk_compat_variants,
    decompose_compatible, is_combining_mark, is_public_assigned,
};
use unicode_normalization::{IsNormalized, UnicodeNormalization};

const SCALARS: u32 = 0x11_0000;

/// The crate version pinned in `Cargo.toml`.
const UPSTREAM_VERSION: &str = "0.1.25";

/// The fingerprint of the crate's algorithms, which `src/` ports.
const SOURCES: u64 = 0xe118_0940_55f5_a8b2;

fn main() {
    let mut args = std::env::args().skip(1);
    let root = PathBuf::from(args.next().unwrap_or_else(|| "../..".into()));
    let test_file = PathBuf::from(
        args.next()
            .unwrap_or_else(|| "target/NormalizationTest-17.0.0.txt".into()),
    );

    let print = fingerprint(include_str!(concat!(env!("OUT_DIR"), "/sources.rs.txt")));
    if print != SOURCES {
        eprintln!(
            "error: unicode-normalization is not the version src/ ports.\n\
             Compare its source in {} with the previous version, carry any change\n\
             into src/, then set SOURCES in scripts/generate/src/main.rs to\n\
             {print:#x}",
            env!("UPSTREAM_DIR")
        );
        std::process::exit(1);
    }

    let tables = tables();
    let tables_path = root.join("src/Tables.mw");
    std::fs::write(&tables_path, &tables).unwrap();
    eprintln!("wrote {} ({} bytes)", tables_path.display(), tables.len());

    let official = std::fs::read_to_string(&test_file)
        .unwrap_or_else(|e| panic!("could not read {}: {e}", test_file.display()));
    let cases = cases(&official);
    let cases_path = root.join("src/Cases.mw");
    std::fs::write(&cases_path, &cases).unwrap();
    eprintln!("wrote {} ({} bytes)", cases_path.display(), cases.len());
}

/// FNV-1a: stable across builds, which `DefaultHasher` does not promise.
fn fingerprint(text: &str) -> u64 {
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    for b in text.bytes() {
        h ^= u64::from(b);
        h = h.wrapping_mul(0x0100_0000_01b3);
    }
    h
}

// --- encoding -----------------------------------------------------------------------

/// A number as `digits` base-64 digits, most significant first, each digit the
/// character `'0' + d`: `'0'` to `'o'`, one contiguous run of ASCII.
fn digits(out: &mut String, value: u64, digits: u32) {
    assert!(
        value < 1 << (6 * digits),
        "{value} does not fit in {digits} digits"
    );
    for k in (0..digits).rev() {
        out.push(char::from(b'0' + ((value >> (6 * k)) & 63) as u8));
    }
}

/// A string, as its length in bytes (3 digits) and then its bytes.
fn text(out: &mut String, s: &str) {
    digits(out, s.len() as u64, 3);
    out.push_str(s);
}

/// `text` as one Meadow string literal, broken with `\`-newline every `width`
/// characters. Only printable ASCII is written raw; a space that would start a
/// line is `\x20`, since a continuation drops leading whitespace.
fn long_literal(text: &str, width: usize) -> String {
    let mut out = String::with_capacity(text.len() + text.len() / width * 4 + 2);
    out.push('"');
    for (i, c) in text.chars().enumerate() {
        let line_start = i > 0 && i % width == 0;
        if line_start {
            out.push_str("\\\n    ");
        }
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '$' => out.push_str("\\$"),
            ' ' if line_start => out.push_str("\\x20"),
            ' '..='~' => out.push(c),
            _ => {
                let _ = write!(out, "\\u{{{:X}}}", u32::from(c));
            }
        }
    }
    out.push('"');
    out
}

const HEADER: &str = "\
-- Copyright 2012-2025 The Rust Project Developers, and the Meadow port's
-- authors. Dual-licensed under Apache-2.0 or MIT: see COPYRIGHT.";

// --- tables -------------------------------------------------------------------------

fn chars() -> impl Iterator<Item = char> {
    (0..SCALARS).filter_map(char::from_u32)
}

fn is_hangul_syllable(c: char) -> bool {
    ('\u{AC00}'..='\u{D7A3}').contains(&c)
}

fn collect(c: char, f: impl Fn(char, &mut dyn FnMut(char))) -> Vec<char> {
    let mut out = Vec::new();
    f(c, &mut |d| out.push(d));
    out
}

fn canonical(c: char) -> Vec<char> {
    collect(c, |c, emit| decompose_canonical(c, emit))
}

fn compatible(c: char) -> Vec<char> {
    collect(c, |c, emit| decompose_compatible(c, emit))
}

fn cjk_variant(c: char) -> Vec<char> {
    collect(c, |c, emit| decompose_cjk_compat_variants(c, emit))
}

/// Maximal runs of code points with the same value, leaving out `skip`.
fn runs<T: PartialEq + Copy>(f: impl Fn(char) -> T, skip: T) -> Vec<(u32, u32, T)> {
    let mut out: Vec<(u32, u32, T)> = Vec::new();
    for c in chars() {
        let cp = u32::from(c);
        let v = f(c);
        if v == skip {
            continue;
        }
        match out.last_mut() {
            Some((_, hi, lv)) if *lv == v && (*hi + 1 == cp || (*hi == 0xD7FF && cp == 0xE000)) => {
                *hi = cp
            }
            _ => out.push((cp, cp, v)),
        }
    }
    out
}

/// A decomposition table: an index of `cp start length` records (4, 3 and 1
/// digits), sorted, into a string of code points (4 digits each).
fn decomposition_table(map: &BTreeMap<char, Vec<char>>) -> (String, String) {
    let mut index = String::new();
    let mut data = String::new();
    let mut at = 0u64;
    for (c, d) in map {
        digits(&mut index, u32::from(*c).into(), 4);
        digits(&mut index, at, 3);
        digits(&mut index, d.len() as u64, 1);
        for x in d {
            digits(&mut data, u32::from(*x).into(), 4);
        }
        at += 4 * d.len() as u64;
    }
    (index, data)
}

fn qc(r: IsNormalized) -> u8 {
    match r {
        IsNormalized::Yes => 0,
        IsNormalized::No => 1,
        IsNormalized::Maybe => 2,
    }
}

fn tables() -> String {
    let (maj, min, pat) = unicode_normalization::UNICODE_VERSION;

    // Hangul syllables and ASCII decompose by rule, not by table.
    let tabled = |c: char| c > '\x7f' && !is_hangul_syllable(c);
    let mut canon = BTreeMap::new();
    let mut compat = BTreeMap::new();
    let mut variants = BTreeMap::new();
    for c in chars().filter(|c| tabled(*c)) {
        let d = canonical(c);
        let k = compatible(c);
        let v = cjk_variant(c);
        if d != [c] {
            canon.insert(c, d.clone());
        }
        // Compatibility falls back to canonical, so only differences are kept.
        if k != d {
            compat.insert(c, k);
        }
        if v != [c] {
            variants.insert(c, v);
        }
    }

    // The primary composites: every pair that composes, found by trying every
    // character that takes part in a decomposition against every other.
    let mut parts: BTreeSet<char> = BTreeSet::new();
    for (c, d) in &canon {
        parts.insert(*c);
        parts.extend(d.iter().copied());
    }
    let mut composites: Vec<(char, char, char)> = Vec::new();
    for &a in &parts {
        for &b in &parts {
            if let Some(c) = compose(a, b) {
                composites.push((a, b, c));
            }
        }
    }
    composites.sort();
    let expected = include_str!(concat!(env!("OUT_DIR"), "/composition_count.txt"))
        .trim()
        .parse::<usize>()
        .unwrap();
    assert_eq!(
        composites.len(),
        expected,
        "the composites found are not the crate's whole table"
    );

    // Leading and trailing non-starters in a decomposition, as the crate's
    // generator counts them.
    let mut nonstarters = String::new();
    for (c, d) in canon
        .iter()
        .chain(compat.iter())
        .collect::<BTreeMap<_, _>>()
    {
        let full = compat.get(c).unwrap_or(d);
        let lead = full
            .iter()
            .take_while(|x| canonical_combining_class(**x) != 0)
            .count();
        let trail = full
            .iter()
            .rev()
            .take_while(|x| canonical_combining_class(**x) != 0)
            .count();
        if lead > 0 || trail > 0 {
            digits(&mut nonstarters, u32::from(*c).into(), 4);
            digits(&mut nonstarters, lead as u64, 1);
            digits(&mut nonstarters, trail as u64, 1);
        }
    }

    let value_table = |runs: Vec<(u32, u32, u8)>, width: u32| {
        let mut s = String::new();
        for (lo, hi, v) in runs {
            digits(&mut s, lo.into(), 4);
            digits(&mut s, hi.into(), 4);
            digits(&mut s, v.into(), width);
        }
        s
    };
    let range_table = |runs: Vec<(u32, u32, bool)>| {
        let mut s = String::new();
        for (lo, hi, _) in runs {
            digits(&mut s, lo.into(), 4);
            digits(&mut s, hi.into(), 4);
        }
        s
    };
    let quick = |f: fn(std::iter::Once<char>) -> IsNormalized| {
        value_table(runs(|c| qc(f(std::iter::once(c))), 0), 1)
    };

    let (canon_index, canon_data) = decomposition_table(&canon);
    let (compat_index, compat_data) = decomposition_table(&compat);
    let (variant_index, variant_data) = decomposition_table(&variants);
    let mut pairs = String::new();
    for (a, b, c) in &composites {
        for x in [a, b, c] {
            digits(&mut pairs, u32::from(*x).into(), 4);
        }
    }

    let mut out = String::new();
    let _ = writeln!(
        out,
        "-- GENERATED by scripts/generate.sh from unicode-normalization {UPSTREAM_VERSION}.
-- Do not edit: run the script again instead.
--
-- Every table is a string of fixed-width records, each field written in the
-- base-64 digits '0' ('0' + 0) to 'o' ('0' + 63), most significant first, and
-- is read in place by `Lookup.mw`. Tables of ranges hold `lo hi`, 4 digits
-- each, and sometimes a value after.
--
{HEADER}

-- The Unicode version the tables describe.
@pub(pkg) def unicodeVersion = ({maj}, {min}, {pat})
"
    );
    let mut def = |name: &str, doc: &str, body: &str| {
        let _ = writeln!(
            out,
            "{doc}\n@pub(pkg) def {name} =\n  {}\n",
            long_literal(body, 96)
        );
    };
    def(
        "combiningClasses",
        "-- The canonical combining class of each code point that has one: ranges and\n-- the class (2 digits).",
        &value_table(runs(canonical_combining_class, 0), 2),
    );
    let decompositions: [(&str, &str, &str, &str, &str); 3] = [
        (
            "canonicalIndex",
            "canonicalData",
            &canon_index,
            &canon_data,
            "full canonical decomposition, for all but ASCII and Hangul syllables",
        ),
        (
            "compatibilityIndex",
            "compatibilityData",
            &compat_index,
            &compat_data,
            "full compatibility decomposition, where it differs from the canonical one",
        ),
        (
            "variantIndex",
            "variantData",
            &variant_index,
            &variant_data,
            "CJK compatibility variant (standardized variation sequence)",
        ),
    ];
    for (index_name, data_name, index, data, what) in decompositions {
        def(
            index_name,
            &format!(
                "-- Each code point's {what}: `cp start length` (4, 3, 1 digits),\n-- where `start` is an offset into `{data_name}`."
            ),
            index,
        );
        def(data_name, "-- Code points, 4 digits each.", data);
    }
    def(
        "composites",
        "-- The primary composites, as `first second composite` (4 digits each), sorted.\n-- Hangul syllables compose by rule and are not here.",
        &pairs,
    );
    def(
        "nonstarters",
        "-- For each decomposable code point whose decomposition starts or ends with\n-- non-starters: `cp leading trailing` (4, 1, 1 digits).",
        &nonstarters,
    );
    def(
        "combiningMarks",
        "-- The code points with General_Category=Mark.",
        &range_table(runs(is_combining_mark, false)),
    );
    def(
        "publicAssigned",
        "-- The code points assigned for public use.",
        &range_table(runs(is_public_assigned, false)),
    );
    for (name, f) in [
        (
            "quickNfc",
            unicode_normalization::is_nfc_quick::<std::iter::Once<char>> as fn(_) -> _,
        ),
        ("quickNfkc", unicode_normalization::is_nfkc_quick),
        ("quickNfd", unicode_normalization::is_nfd_quick),
        ("quickNfkd", unicode_normalization::is_nfkd_quick),
    ] {
        def(
            name,
            &format!(
                "-- The {name} quick-check property, where it is not Yes: ranges and 1 for\n-- No, 2 for Maybe."
            ),
            &quick(f),
        );
    }
    out.truncate(out.trim_end().len());
    out.push('\n');
    out
}

// --- cases --------------------------------------------------------------------------

/// A small deterministic generator, so that the cases are the same on every run.
struct Rng(u64);

impl Rng {
    fn next(&mut self) -> u64 {
        // xorshift64*
        self.0 ^= self.0 >> 12;
        self.0 ^= self.0 << 25;
        self.0 ^= self.0 >> 27;
        self.0.wrapping_mul(0x2545_f491_4f6c_dd1d)
    }

    fn below(&mut self, n: usize) -> usize {
        (self.next() % n as u64) as usize
    }
}

/// Characters of every kind the algorithms treat differently.
fn interesting() -> Vec<char> {
    let mut picks: BTreeSet<char> = "aAeoz.1 \u{7f}".chars().collect();
    let mut by_class: BTreeMap<u8, usize> = BTreeMap::new();
    let (mut canon, mut compat, mut variant) = (0, 0, 0);
    for c in chars() {
        let class = canonical_combining_class(c);
        let seen = by_class.entry(class).or_default();
        if *seen < 3 {
            *seen += 1;
            picks.insert(c);
        }
        let d = canonical(c);
        if d != [c] && canon < 60 && u32::from(c) % 7 == 0 {
            canon += 1;
            picks.insert(c);
            picks.extend(d.iter().copied());
        }
        if compatible(c) != d && compat < 40 && u32::from(c) % 11 == 0 {
            compat += 1;
            picks.insert(c);
        }
        if cjk_variant(c) != [c] && variant < 10 {
            variant += 1;
            picks.insert(c);
        }
    }
    // Hangul: jamo of each kind, and syllables with and without a trailing
    // consonant.
    picks.extend(
        "\u{1100}\u{1112}\u{1161}\u{1175}\u{11A7}\u{11A8}\u{11C2}\u{11C3}\u{AC00}\u{AC01}\u{D7A3}"
            .chars(),
    );
    // Characters with leading or trailing non-starters in their decompositions.
    picks.extend(
        "\u{0340}\u{0344}\u{0F73}\u{0F75}\u{0F81}\u{FF9E}\u{1E0B}\u{1E0D}\u{0323}\u{0307}\u{0301}"
            .chars(),
    );
    picks.into_iter().collect()
}

/// The code points at the edges of the Hangul jamo and syllable ranges.
const HANGUL_EDGES: [u32; 20] = [
    0x10FF, 0x1100, 0x1112, 0x1113, 0x1160, 0x1161, 0x1175, 0x1176, 0x11A6, 0x11A7, 0x11A8, 0x11C2,
    0x11C3, 0xABFF, 0xAC00, 0xAC01, 0xAC1B, 0xAC1C, 0xD7A3, 0xD7A4,
];

fn quick_index(r: IsNormalized) -> u64 {
    qc(r).into()
}

fn cases(official: &str) -> String {
    let mut strings: Vec<String> = Vec::new();
    // Every tenth line of the official test, each of its five columns.
    for line in official
        .lines()
        .filter(|l| !l.starts_with('#') && !l.starts_with('@'))
        .step_by(10)
    {
        for column in line.split(';').take(5) {
            let s: Option<String> = column
                .split_whitespace()
                .map(|h| u32::from_str_radix(h, 16).ok().and_then(char::from_u32))
                .collect();
            if let Some(s) = s {
                strings.push(s);
            }
        }
    }
    let official_count = strings.iter().collect::<BTreeSet<_>>().len();

    let pool = interesting();
    let mut rng = Rng(0x0f0c_7a4a_c0ff_ee42);
    for _ in 0..3000 {
        let len = 1 + rng.below(8);
        strings.push((0..len).map(|_| pool[rng.below(pool.len())]).collect());
    }
    // Long runs of non-starters, for stream safety.
    for n in [29, 30, 31, 32, 45, 61] {
        let marks: String = (0..n).map(|k| pool[(k * 7) % pool.len()]).collect();
        strings.push(format!("a{}", "\u{0301}".repeat(n)));
        strings.push(format!("a{marks}b"));
        strings.push(format!("\u{0344}{}\u{0F73}", "\u{0308}".repeat(n)));
    }
    // Every pair of Hangul edge characters, as text.
    for a in HANGUL_EDGES.iter().filter_map(|c| char::from_u32(*c)) {
        for b in HANGUL_EDGES.iter().filter_map(|c| char::from_u32(*c)) {
            strings.push(format!("{a}{b}"));
            strings.push(format!("{a}{b}\u{11A8}"));
        }
    }
    strings.sort();
    strings.dedup();

    let mut body = String::new();
    for s in &strings {
        digits(&mut body, 0, 1);
        text(&mut body, s);
        for normalized in [
            s.nfd().collect::<String>(),
            s.nfkd().collect(),
            s.nfc().collect(),
            s.nfkc().collect(),
            s.cjk_compat_variants().collect(),
            s.stream_safe().collect(),
        ] {
            text(&mut body, &normalized);
        }
        use unicode_normalization as un;
        for q in [
            un::is_nfc_quick(s.chars()),
            un::is_nfkc_quick(s.chars()),
            un::is_nfd_quick(s.chars()),
            un::is_nfkd_quick(s.chars()),
            un::is_nfc_stream_safe_quick(s.chars()),
            un::is_nfd_stream_safe_quick(s.chars()),
        ] {
            digits(&mut body, quick_index(q), 1);
        }
        for b in [
            un::is_nfc(s),
            un::is_nfkc(s),
            un::is_nfd(s),
            un::is_nfkd(s),
            un::is_nfc_stream_safe(s),
            un::is_nfd_stream_safe(s),
        ] {
            digits(&mut body, u64::from(b), 1);
        }
    }

    // Characters: either side of every change in what the `char` functions
    // say, and the pool.
    let facts = |c: char| {
        (
            canonical_combining_class(c),
            canonical(c),
            compatible(c),
            cjk_variant(c),
            is_combining_mark(c),
            is_public_assigned(c),
        )
    };
    let mut cps: BTreeSet<u32> = pool.iter().map(|c| u32::from(*c)).collect();
    let mut last = None;
    for c in chars() {
        let now = facts(c);
        // Decompositions change at almost every character; only the shape of
        // the answer is compared here, to keep the list short.
        let shape = (
            now.0,
            now.1.len() > 1 || now.1 != [c],
            now.2 != now.1,
            now.3 != [c],
            now.4,
            now.5,
        );
        if last != Some(shape) {
            cps.insert(u32::from(c).saturating_sub(1));
            cps.insert(u32::from(c));
        }
        last = Some(shape);
    }
    let mut char_count = 0;
    for cp in cps {
        let Some(c) = char::from_u32(cp) else {
            continue;
        };
        let (class, canon, compat, variant, mark, assigned) = facts(c);
        char_count += 1;
        digits(&mut body, 1, 1);
        digits(&mut body, cp.into(), 4);
        digits(&mut body, class.into(), 2);
        for d in [canon, compat, variant] {
            text(&mut body, &d.into_iter().collect::<String>());
        }
        digits(&mut body, u64::from(mark), 1);
        digits(&mut body, u64::from(assigned), 1);
    }

    // Pairs composed: every pair of the characters at the edges of the Hangul
    // ranges, and then random ones.
    let hangul: Vec<char> = HANGUL_EDGES
        .iter()
        .filter_map(|c| char::from_u32(*c))
        .collect();
    let mut pairs: Vec<(char, char)> = Vec::new();
    for &a in &hangul {
        for &b in &hangul {
            pairs.push((a, b));
        }
    }
    for _ in 0..3000 {
        pairs.push((pool[rng.below(pool.len())], pool[rng.below(pool.len())]));
    }
    let mut pair_count = 0;
    for (a, b) in pairs {
        pair_count += 1;
        digits(&mut body, 2, 1);
        digits(&mut body, u32::from(a).into(), 4);
        digits(&mut body, u32::from(b).into(), 4);
        digits(&mut body, compose(a, b).map_or(0, u32::from).into(), 4);
    }

    let mut out = String::new();
    let _ = writeln!(
        out,
        "-- GENERATED by scripts/generate.sh from unicode-normalization {UPSTREAM_VERSION}.
-- Do not edit: run the script again instead.
--
-- Inputs, with what the crate makes of them, for `Tests.mw`: {} strings, {} of
-- them from the official NormalizationTest.txt; {char_count} characters; and
-- {pair_count} pairs composed.
--
{HEADER}

-- Each case starts with its kind (1 base-64 digit), and its fields follow in
-- the order `Tests.mw` reads them. A string is its length in bytes (3 digits)
-- and then its bytes; a quick check is 0, 1 or 2 for Yes, No or Maybe.
@cfg(test)
@pub(pkg) def cases =
  {}",
        strings.len(),
        official_count,
        long_literal(&body, 96)
    );
    out
}
