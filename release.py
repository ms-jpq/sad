#!/usr/bin/env python3

import argparse
import hashlib
import jinja2
import os
import subprocess
import sys
import shutil
import toml
import yaml
from argparse import Namespace
from os import path
from typing import Iterator, List

artifacts_dir = "artifacts"
build_dir = "target"
prog_name = "sad"


def cwd() -> None:
  cwd = path.dirname(path.abspath(__file__))
  os.chdir(cwd)


def run(args: List[str], cwd=os.getcwd()) -> None:
  ret = subprocess.run(args, cwd=cwd.encode(),
                       stdout=sys.stdout, stderr=sys.stderr)
  if ret.returncode != 0:
    exit(ret.returncode)


def cross_build() -> Iterator[str]:
  targets = ["x86_64-unknown-linux-gnu",
             "x86_64-unknown-linux-musl"]
  for arch in targets:
    args = ["cross", "build", "--release", "--target", arch]
    run(args)
    release = path.join(build_dir, arch, "release", prog_name)
    dest = path.join(artifacts_dir, arch)
    shutil.copy2(release, dest)
    yield dest


def sha256(resource: str) -> str:
  with open(resource, "rb") as fd:
    binary = fd.read()
    sha = hashlib.sha256(binary).hexdigest()
    return sha


def macos_build() -> str:
  if sys.platform != "darwin":
    return
  arch = "x86_64-apple-darwin"
  artifact_dir = path.join(build_dir, arch)
  args = ["cargo", "build", "--release", "--target-dir", artifact_dir]
  run(args)
  release = path.join(build_dir, arch, "release", prog_name)
  dest = path.join(artifacts_dir, arch)
  shutil.copy2(release, dest)
  return dest


def git_repo(name, uri: str) -> None:
  install_target = path.join(artifacts_dir, name)
  if path.isdir(install_target):
    run(["git", "pull"], cwd=install_target)
  else:
    run(["git", "clone", "--depth=1", uri,
         install_target])


def homebrew_release(artifact: str) -> None:
  if sys.platform != "darwin":
    return
  git_repo("homebrew", "https://github.com/ms-jpq/homebrew-sad")
  sha = sha256(artifact)


def parse_args() -> Namespace:
  parser = argparse.ArgumentParser()
  return parser.parse_args()


def main() -> None:
  cwd()
  args = parse_args()
  os.makedirs(artifacts_dir, exist_ok=True)
  # macos_artifact = macos_build()

  # homebrew_release()
  # cross_build()


main()
