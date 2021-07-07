#!/usr/bin/env python3

from argparse import ArgumentParser, Namespace
from datetime import datetime
from os import chdir
from os.path import dirname, join
from subprocess import run
from typing import Any

from yaml import safe_load

__dir__ = dirname(__file__)
__prefix__ = "msjpq/sad"


def _call(prog: str, *args: str) -> None:
    ret = run([prog, *args])
    if ret.returncode != 0:
        exit(ret.returncode)


def _slurp_yaml(path: str) -> Any:
    with open(path) as fd:
        return safe_load(fd)


def _parse_args() -> Namespace:
    parser = ArgumentParser()
    parser.add_argument("dest", choices=("deb", "linux"))
    return parser.parse_args()


def _docker_build(name: str) -> None:
    image = f"{__prefix__}:{name}"
    path = join("ci", name, "Dockerfile")
    _call("docker", "build", "-t", image, "-f", path, ".")


def _docker_cp(name: str) -> None:
    time = datetime.now().strftime("%H-%M-%S")
    image = f"{__prefix__}:{name}"
    path = join("ci", name, "artifacts.yml")
    spec = _slurp_yaml(path)
    container = f"{name}-{time}"

    _call("docker", "create", "--name", container, image)
    for target in spec["targets"]:
        src = f"{container}:{target['src']}"
        dest = join("artifacts", target["dest"])
        _call("docker", "cp", src, dest)
    _call("docker", "rm", container)


def main() -> None:
    chdir(dirname(__dir__))
    args = _parse_args()
    name = args.dest
    _docker_build(name)
    _docker_cp(name)


main()
