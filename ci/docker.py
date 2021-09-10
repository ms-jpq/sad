#!/usr/bin/env python3

from argparse import ArgumentParser, Namespace
from datetime import datetime
from pathlib import Path
from subprocess import check_call

from yaml import safe_load

_TOP_LEVEL = Path(__file__).resolve().parent.parent
_PREFIX = "msjpq/sad"


def _parse_args() -> Namespace:
    parser = ArgumentParser()
    parser.add_argument("dest", choices=("deb", "linux"))
    return parser.parse_args()


def _docker_build(name: str) -> None:
    image = f"{_PREFIX}:{name}"
    path = _TOP_LEVEL / "ci" / name / "Dockerfile"
    check_call(
        (
            "docker",
            "build",
            "-t",
            image,
            "-f",
            path,
            "--",
            ".",
        ),
        cwd=_TOP_LEVEL,
    )


def _docker_cp(name: str) -> None:
    time = datetime.now().strftime("%H-%M-%S")
    image = f"{_PREFIX}:{name}"
    path = _TOP_LEVEL / "ci" / name / "artifacts.yml"
    with path.open() as fd:
        spec = safe_load(fd)

    container = f"{name}-{time}"

    check_call(
        (
            "docker",
            "create",
            "--name",
            container,
            image,
        )
    )
    for target in spec["targets"]:
        src = f"{container}:{target['src']}"
        dest = _TOP_LEVEL / "artifacts" / target["dest"]
        check_call(
            (
                "docker",
                "cp",
                src,
                dest,
            )
        )

    check_call(
        (
            "docker",
            "rm",
            container,
        )
    )


def main() -> None:
    args = _parse_args()
    name = args.dest

    _docker_build(name)
    _docker_cp(name)


main()
