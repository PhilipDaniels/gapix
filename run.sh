#!/bin/bash
cd "$(dirname "$0")"

# Build separately so that globbing works in the next command.
cargo build

#RUST_BACKTRACE=1 RUST_LOG=DEBUG target/release/gapix -f --metres=5 --analyse /home/phil/OneDrive/Documents/Cycling/Testing\ GPXs/*.gpx
#RUST_BACKTRACE=1 RUST_LOG=DEBUG target/release/gapix -f --metres=5 --analyse --countries=GB /home/phil/OneDrive/Documents/Cycling/Testing\ GPXs/*.gpx 2>&1
#RUST_BACKTRACE=1 RUST_LOG=DEBUG target/release/gapix -f --metres=5 --analyse --force-geonames-download --countries=GB,FR,US /home/phil/OneDrive/Documents/Cycling/Testing\ GPXs/*.gpx 2>&1

RUST_BACKTRACE=1 RUST_LOG=DEBUG target/debug/gapix -f --metres=5 --analyse --countries=GB /home/phil/OneDrive/Documents/Cycling/Testing\ FITs/*.fit 2>&1
