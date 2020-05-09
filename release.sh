#!/usr/bin/env bash

set -eu
set -o pipefail

cd "$(dirname "$0")"

mkdir -p ./dist ./out


if [[ "$(uname)" = 'Linux' ]]
then
  cargo build --release --target=x86_64-unknown-linux-gnu --target-dir=./dist/x86_64-unknown-linux-gnu
fi

if [[ "$(uname)" = 'Darwin' ]]
then
  docker run -it --rm -w /workdir -v "$PWD":/workdir rust cargo build --release --target-dir=./dist/x86_64-unknown-linux-gnu
  cargo build --release --target-dir=./dist/x86_64-apple-darwin

  cp ./dist/x86_64-apple-darwin/release/sad ./out/x86_64-apple-darwin
fi

cp ./dist/x86_64-unknown-linux-gnu/release/sad ./out/x86_64-unknown-linux-gnu

zip -r release.zip ./out

