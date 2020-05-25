#!/usr/bin/env python3

import argparse
import jinja2
import os
import subprocess
import sys
import shutil
from argparse import Namespace
from os import path
from typing import List


release_dir = "release"
build_dir = "target"
prog_name = "sad"


def cwd() -> None:
  cwd = path.dirname(path.abspath(__file__))
  os.chdir(cwd)


def run(args: List[str]) -> None:
  ret = subprocess.run(args, stdout=sys.stdout, stderr=sys.stderr)
  if ret.returncode != 0:
    exit(ret.returncode)


def cross_build() -> None:
  targets = ["x86_64-unknown-linux-gnu",
             "x86_64-unknown-linux-musl"]
  for arch in targets:
    args = ["cross", "build", "--release", "--target", arch]
    run(args)
    release = path.join(build_dir, arch, release_dir, prog_name)
    dest = path.join(release_dir, arch)
    shutil.copy2(release, dest)


def macos_build() -> None:
  if sys.platform != "darwin":
    return
  arch = "x86_64-apple-darwin"
  artifact_dir = path.join(build_dir, arch)
  args = ["cargo", "build", "--release", "--target-dir", artifact_dir]
  run(args)
  release = path.join(build_dir, arch, release_dir, prog_name)
  dest = path.join(release_dir, arch)
  shutil.copy2(release, dest)


def parse_args() -> Namespace:
  parser = argparse.ArgumentParser()
  return parser.parse_args()


def main() -> None:
  cwd()
  args = parse_args()
  os.makedirs(release_dir, exist_ok=True)
  macos_build()
  cross_build()


main()
