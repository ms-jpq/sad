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


def cwd() -> None:
  cwd = path.dirname(path.abspath(__file__))
  os.chdir(cwd)


def parse_args() -> Namespace:
  parser = argparse.ArgumentParser()
  return parser.parse_args()


def main() -> None:
  cwd()
  args = parse_args()
  os.makedirs(artifacts_dir, exist_ok=True)


main()
