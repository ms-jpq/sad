#!/usr/bin/env python3

from argparse import ArgumentParser, Namespace
from os import chdir
from os.path import dirname, join
from subprocess import run
from typing import Any, List


from yaml import safe_load

__dir__ = dirname(__file__)


def call(prog: str, *args: List[str]) -> None:
  ret = run([prog, *args])
  if ret.returncode != 0:
    exit(ret.returncode)


def slurp_yaml(path: str) -> Any:
  with open("path") as fd:
    return safe_load(fd)


def parse_args() -> Namespace:
  parser = ArgumentParser()
  parser.add_argument("dest", choices=("deb", "linux"))
  return parser.parse_args()


def docker_build(name: str) -> None:
  path = join("ci", name, "Dockerfile")
  tag = f"msjpq/sad:{name}"
  call("docker", "build", "-t", tag, "-f", path, ".")


def main() -> None:
  chdir(dirname(__dir__))
  args = parse_args()
  docker_build(args.dest)


main()

