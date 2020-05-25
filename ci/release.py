#!/usr/bin/env python3

import argparse
import hashlib
import os
import subprocess
import sys
from argparse import Namespace
from datetime import datetime
from os import path
from typing import Any, Callable, Dict, List

import jinja2
import toml
import yaml
from jinja2 import Environment

artifacts_dir = "artifacts"
packages_dir = "packages"


def cwd() -> None:
  root = path.dirname(path.dirname(path.abspath(__file__)))
  os.chdir(root)


def run(args: List[str], cwd=os.getcwd()) -> None:
  ret = subprocess.run(args, cwd=cwd.encode(),
                       stdout=sys.stdout, stderr=sys.stderr)
  if ret.returncode != 0:
    exit(ret.returncode)


def load_yaml(src: str) -> Any:
  with open(src) as fd:
    return yaml.safe_load(fd)


def load_values() -> Dict[str, str]:
  cargo = toml.load("Cargo.toml")
  vals = load_yaml(path.join("ci", "vars.yml"))
  values = {"project_repo": "https://github.com/ms-jpq/sad",
            "version": cargo["package"]["version"],
            "desc": vals["desc"], }
  return values


def build_j2(src: str, filters: Dict[str, Callable] = {}) -> Environment:
  j2 = jinja2.Environment(
      enable_async=True,
      trim_blocks=True,
      lstrip_blocks=True,
      undefined=jinja2.StrictUndefined,
      loader=jinja2.FileSystemLoader(src))
  j2.filters = {**j2.filters, **filters}
  return j2


def git_commit(repo: str) -> None:
  token = os.environ["CI_TOKEN"]
  uri = f"https://ms-jpq:{token}@github.com/ms-jpq/homebrew-sad"
  time = datetime.now().strftime("%Y-%m-%d %H:%M")
  msg = f"CI - {time}"
  run(["git", "remote", "set-url", "origin", uri])
  run(["git", "add", "-A"], cwd=repo)
  run(["git", "commit", "-m", msg], cwd=repo)
  run(["git", "push", "--force"], cwd=repo)


def write(filename: str, text: str) -> None:
  with open(filename, "w") as fd:
    fd.write(text)


def sha256(resource: str) -> str:
  with open(resource, "rb") as fd:
    binary = fd.read()
    sha = hashlib.sha256(binary).hexdigest()
    return sha


def homebrew_release(j2: Environment, values: Dict[str, str], artifact: str, uri: str) -> None:
  sha = sha256(artifact)
  vals = {**values, "sha256": sha, "release_uri": uri}
  render = j2.get_template("homebrew.rb.j2").render(**vals)
  dest = path.join(packages_dir, "sad.rb")
  write(dest, render)
  git_commit(packages_dir)


def parse_args() -> Namespace:
  parser = argparse.ArgumentParser()
  parser.add_argument("--brew-artifact")
  parser.add_argument("--brew-uri")
  return parser.parse_args()


def main() -> None:
  cwd()
  args = parse_args()
  j2 = build_j2(path.join("ci", "templates"))
  values = load_values()
  if args.brew_artifact and args.brew_uri:
    homebrew_release(
        j2=j2,
        values=values,
        artifact=path.join(artifacts_dir, args.brew_artifact),
        uri=args.brew_uri)
  else:
    exit(1)


main()
