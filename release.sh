#!/usr/bin/env bash

set -eu
set -o pipefail

cd "$(dirname "$0")"

mkdir -p ./dist

cargo build --release --target=x86_64-unknown-linux-gnu
cargo build --release --target=x86_64-apple-darwin

