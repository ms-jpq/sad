#!/usr/bin/env python3

import argparse
import os
import subprocess
import sys
from argparse import Namespace
from datetime import datetime
from os import path
from typing import Iterator, List

import toml


def cwd() -> None:
  root = path.dirname(path.dirname(path.abspath(__file__)))
  os.chdir(root)


def set_output(name: str, value: str) -> None:
  print(f"::set-output name={name}::{value}")


def run(args: List[str], cwd=os.getcwd()) -> None:
  ret = subprocess.run(args, cwd=cwd.encode(),
                       stdout=sys.stdout, stderr=sys.stderr)
  if ret.returncode != 0:
    exit(ret.returncode)


def set_tag() -> None:
  cargo = toml.load("Cargo.toml")
  version = cargo["package"]["version"]
  time = datetime.now().strftime("%Y-%m-%d %H:%M")
  tag = f"{version} | {time}"
  set_output("TAG_NAME", tag)
  set_output("RELEASE_NAME", tag)


def parse_args() -> Namespace:
  parser = argparse.ArgumentParser()
  parser.add_argument("--release", action="store_true")
  return parser.parse_args()


def main() -> None:
  cwd()
  args = parse_args()
  if args.tag:
    set_tag()


main()
