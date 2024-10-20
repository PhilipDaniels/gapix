#!/bin/bash
cd "$(dirname "$0")"

# Build separately so that globbing works in the next command.
cargo build --release

RUST_BACKTRACE=1 RUST_LOG=DEBUG target/release/gapix -f --metres=5 --analyse /home/phil/OneDrive/Documents/Cycling/Testing\ GPXs/*.gpx
