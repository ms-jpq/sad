#!/usr/bin/env python3

import argparse
import json
import os
from argparse import Namespace
from datetime import datetime
from os import path
from typing import Iterator, List

import toml


def cwd() -> None:
  root = path.dirname(path.dirname(path.abspath(__file__)))
  os.chdir(root)


def read(name: str) -> str:
  with open(name, "r") as fd:
    return fd.read()


def set_output(name: str, value: str) -> None:
  print(f"::set-output name={name}::{value}")


def set_release_env() -> None:
  cargo = toml.load("Cargo.toml")
  version = cargo["package"]["version"]
  time = datetime.now()
  tag_name = f"ci_{version}_{time.strftime('%Y-%m-%d_%H-%M')}"
  release_name = f"CI - {version} | {time.strftime('%Y-%m-%d %H:%M')}"
  release_notes = read("release_notes.md")
  release_info = {"tag_name": tag_name,
                  "release_name": release_name,
                  "release_notes": release_notes}

  dump = json.dumps(release_info)
  set_output("RELEASE_INFO", dump)


def parse_args() -> Namespace:
  parser = argparse.ArgumentParser()
  parser.add_argument("--release", action="store_true")
  return parser.parse_args()


def main() -> None:
  cwd()
  args = parse_args()
  if args.release:
    set_release_env()


main()
