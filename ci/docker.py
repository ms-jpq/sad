#!/usr/bin/env python3

from argparse import ArgumentParser, Namespace
from datetime import datetime
from os import chdir
from os.path import dirname, join
from subprocess import run
from typing import Any, List

from yaml import safe_load

__dir__ = dirname(__file__)
__prefix__ = "msjpq/sad"


def call(prog: str, *args: str) -> None:
    ret = run([prog, *args])
    if ret.returncode != 0:
        exit(ret.returncode)


def slurp_yaml(path: str) -> Any:
    with open(path) as fd:
        return safe_load(fd)


def parse_args() -> Namespace:
    parser = ArgumentParser()
    parser.add_argument("dest", choices=("deb", "linux"))
    return parser.parse_args()


def docker_build(name: str) -> None:
    image = f"{__prefix__}:{name}"
    path = join("ci", name, "Dockerfile")
    call("docker", "build", "-t", image, "-f", path, ".")


def docker_cp(name: str) -> None:
    time = datetime.now().strftime("%H-%M-%S")
    image = f"{__prefix__}:{name}"
    path = join("ci", name, "artifacts.yml")
    spec = slurp_yaml(path)
    container = f"{name}-{time}"

    call("docker", "create", "--name", container, image)
    for target in spec["targets"]:
        src = f"{container}:{target['src']}"
        dest = join("artifacts", target["dest"])
        call("docker", "cp", src, dest)
    call("docker", "rm", container)


def main() -> None:
    chdir(dirname(__dir__))
    args = parse_args()
    name = args.dest
    docker_build(name)
    docker_cp(name)


main()
