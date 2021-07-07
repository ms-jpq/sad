#!/usr/bin/env python3

from argparse import ArgumentParser, Namespace
from datetime import datetime
from json import dumps
from os import chdir
from os.path import abspath, dirname

from toml import load


def _cwd() -> None:
    root = dirname(dirname(abspath(__file__)))
    chdir(root)


def _read(name: str) -> str:
    with open(name, "r") as fd:
        return fd.read()


def _set_output(name: str, value: str) -> None:
    print(f"::set-output name={name}::{value}")


def _set_release_env() -> None:
    cargo = load("Cargo.toml")
    version = cargo["package"]["version"]
    time = datetime.now()
    tag_name = f"ci_{version}_{time.strftime('%Y-%m-%d_%H-%M')}"
    release_name = f"CI - {version} | {time.strftime('%Y-%m-%d %H:%M')}"
    release_notes = _read("release_notes.md")
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
    _cwd()
    args = _parse_args()
    if args.release:
        _set_release_env()


main()
