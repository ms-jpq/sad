#!/usr/bin/env python3

from argparse import ArgumentParser, Namespace
from datetime import datetime
from json import dumps
from pathlib import Path

from toml import load

_TOP_LEVEL = Path(__file__).resolve().parent.parent


def _set_output(name: str, value: str) -> None:
    print(f"::set-output name={name}::{value}")


def _set_release_env() -> None:
    cargo = load(_TOP_LEVEL / "Cargo.toml")

    version = cargo["package"]["version"]
    time = datetime.now()
    tag_name = f"ci_{version}_{time.strftime('%Y-%m-%d_%H-%M')}"
    release_name = f"CI - {version} | {time.strftime('%Y-%m-%d %H:%M')}"
    release_notes = (_TOP_LEVEL / "release_notes.md").read_text()
    release_info = {
        "tag_name": tag_name,
        "release_name": release_name,
        "release_notes": release_notes,
    }

    dump = dumps(release_info)
    _set_output("RELEASE_INFO", dump)


def _parse_args() -> Namespace:
    parser = ArgumentParser()
    parser.add_argument("--release", action="store_true")
    return parser.parse_args()


def main() -> None:
    args = _parse_args()
    if args.release:
        _set_release_env()


main()
