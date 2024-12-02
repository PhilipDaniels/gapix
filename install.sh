#!/bin/bash
cd "$(dirname "$0")"

cargo build --release && cp target/release/gapix /home/phil/bin
