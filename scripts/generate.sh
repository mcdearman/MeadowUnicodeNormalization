#!/bin/sh
# Regenerate src/Tables.mw and src/Cases.mw from the unicode-normalization crate.
#
#   scripts/generate.sh
#
# The crate version is pinned in scripts/generate/Cargo.toml. To move to a new
# one, change the pin (and UNICODE below, to the crate's Unicode version) and run
# this: if the crate's algorithms have changed, the generator stops and says so,
# and the Meadow sources have to be brought into line with them before the
# fingerprint in scripts/generate/src/main.rs is moved.
#
# Needs a Rust toolchain, curl the first time (for the official test file),
# and `meadow` to format and test the result.

set -eu

UNICODE="17.0.0"
root="$(cd "$(dirname "$0")/.." && pwd)"
tests="$root/scripts/generate/target/NormalizationTest-$UNICODE.txt"

if [ ! -f "$tests" ]; then
    mkdir -p "$(dirname "$tests")"
    curl -fsSL "https://www.unicode.org/Public/$UNICODE/ucd/NormalizationTest.txt" -o "$tests"
fi

cargo run --quiet --release --manifest-path "$root/scripts/generate/Cargo.toml" -- "$root" "$tests"

if command -v meadow >/dev/null 2>&1; then
    (cd "$root" && meadow fmt src && meadow test)
else
    echo "note: meadow is not on PATH, so the result was not formatted or tested" >&2
fi
