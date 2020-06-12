#!/usr/bin/env bash

set -eu
set -o pipefail

RELEASE="$1"


context="$(dirname "$(dirname "$0")")"
cd "$context" || exit 1
printf '%s\n' "$PWD"



IMAGE="msjpq/sad:$RELEASE"

docker build -t "$IMAGE" . -f "$RELEASE/Dockerfile"

