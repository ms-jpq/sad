#!/usr/bin/env bash

set -eu
set -o pipefail

cd "$(dirname "$0")"

rm release.zip || true
rm -r "$PWD/sad" || true
mkdir -p "$PWD/target" "$PWD/sad"


builds=(
  x86_64-unknown-linux-gnu
  x86_64-unknown-linux-musl
  x86_64-pc-windows-gnu
)


cross_build() {
  local ARCH="$1"
  cross build --release --target="$ARCH"
  cp "$PWD/target/$ARCH/release/sad" "./sad/$ARCH" || cp "$PWD/target/$ARCH/release/sad.exe" "./sad/$ARCH"
}


macos_build() {
  if [[ "$(uname)" = 'Darwin' ]]
  then
    local ARCH="x86_64-apple-darwin"
    local DIST="$PWD/target/$ARCH"
    cargo build --release --target-dir="$DIST"
    cp "$DIST/release/sad" "$PWD/sad/$ARCH"
  fi
}


macos_build

for build in "${builds[@]}"
do
  cross_build "$build"
done


zip -r release.zip ./sad

