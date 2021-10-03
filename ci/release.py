#!/usr/bin/env python3

from argparse import ArgumentParser, Namespace
from datetime import datetime
from hashlib import sha256
from os import environ
from pathlib import Path, PurePath
from subprocess import check_call
from sys import exit
from typing import Callable, Dict

from jinja2 import Environment, FileSystemLoader, StrictUndefined
from toml import load as load_toml
from yaml import safe_load

_TOP_LEVEL = Path(__file__).resolve().parent.parent

artifacts_dir = _TOP_LEVEL / "artifacts"
packages_dir = _TOP_LEVEL / "packages"


def _load_values() -> Dict[str, str]:
    cargo = load_toml(_TOP_LEVEL / "Cargo.toml")
    vals = safe_load((_TOP_LEVEL / "ci" / "vars.yml").read_text())
    values = {
        "project_repo": "https://github.com/ms-jpq/sad",
        "version": cargo["package"]["version"],
        "desc": vals["desc"],
        "long_desc": vals["long_desc"],
    }
    return values


def _build_j2(src: PurePath, filters: Dict[str, Callable] = {}) -> Environment:
    j2 = Environment(
        enable_async=True,
        trim_blocks=True,
        lstrip_blocks=True,
        undefined=StrictUndefined,
        loader=FileSystemLoader(src),
    )
    j2.filters = {**j2.filters, **filters}
    return j2


def _git_clone(name: PurePath) -> None:
    token = environ["CI_TOKEN"]
    email = "ci@ci.ci"
    username = "ci-bot"
    uri = f"https://ms-jpq:{token}@github.com/ms-jpq/homebrew-sad.git"
    check_call(("git", "clone", "--depth=1", uri, name), cwd=_TOP_LEVEL)
    check_call(("git", "config", "user.email", email), cwd=name)
    check_call(("git", "config", "user.name", username), cwd=name)


def _git_commit(repo: PurePath) -> None:
    time = datetime.now().strftime("%Y-%m-%d %H:%M")
    msg = f"CI - {time}"
    check_call(("git", "add", "-A"), cwd=repo)
    check_call(("git", "commit", "-m", msg), cwd=repo)
    check_call(("git", "push", "--force"), cwd=repo)


def _homebrew_release(
    j2: Environment, values: Dict[str, str], artifact: Path, uri: str
) -> None:
    sha = sha256(artifact.read_bytes()).hexdigest()
    vals = {**values, "sha256": sha, "release_uri": uri}
    render = j2.get_template("homebrew.rb.j2").render(**vals)
    (packages_dir / "sad.rb").write_text(render)
    _git_commit(packages_dir)


def _snap_release(j2: Environment, values: Dict[str, str]) -> None:
    vals = {**values}
    render = j2.get_template("snapcraft.yml.j2").render(**vals)
    (packages_dir / "snapcraft.yaml").write_text(render)
    _git_commit(packages_dir)


def parse_args() -> Namespace:
    parser = ArgumentParser()
    parser.add_argument("--brew-artifact")
    parser.add_argument("--brew-uri")
    parser.add_argument("--snapcraft", action="store_true")
    return parser.parse_args()


def main() -> None:
    args = parse_args()
    _git_clone(packages_dir)
    j2 = _build_j2(_TOP_LEVEL / "ci" / "templates")
    values = _load_values()
    if args.brew_artifact and args.brew_uri:
        _homebrew_release(
            j2=j2,
            values=values,
            artifact=artifacts_dir / args.brew_artifact,
            uri=args.brew_uri,
        )
    elif args.snapcraft:
        _snap_release(j2=j2, values=values)
    else:
        exit(1)


main()
