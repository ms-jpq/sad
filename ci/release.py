#!/usr/bin/env python3

from contextlib import contextmanager
from dataclasses import asdict, dataclass
from datetime import datetime
from hashlib import sha256
from itertools import chain, repeat
from os import environ, linesep, scandir
from os.path import normcase
from pathlib import Path
from subprocess import check_call, check_output
from typing import Iterator
from urllib.request import build_opener

from jinja2 import Environment, FileSystemLoader, StrictUndefined
from toml import load as load_toml
from yaml import safe_load


@dataclass(frozen=True)
class _Project:
    repo: str
    version: str
    desc: str
    long_desc: str


_TOP_LEVEL = Path(__file__).resolve().parent.parent


def _walk(path: Path) -> Iterator[Path]:
    for s in scandir(path):
        p = Path(s)
        if s.is_dir():
            yield from _walk(p)
        else:
            yield p


def _load_values() -> _Project:
    cargo = load_toml(_TOP_LEVEL / "Cargo.toml")
    vals = safe_load((_TOP_LEVEL / "ci" / "vars.yml").read_text())
    project = _Project(**{**vals, "version": cargo["package"]["version"]})
    return project


def _release(project: _Project) -> str:
    tag = environ["GITHUB_REF"].removeprefix("refs/tags/")

    time = datetime.now().strftime("%Y-%m-%d_%H-%M")
    title = f"ci_{project.version}_{time}"
    body = (_TOP_LEVEL / "RELEASE_NOTES.md").read_text()
    message = f"{title}{linesep}{body}"

    arts = (f"{normcase(p)}#{p.name}" for p in _walk(_TOP_LEVEL / "arts"))
    attachments = chain.from_iterable(zip(repeat("--attach"), arts))

    check_call(
        ("hub", "release", "create", "--message", message, *attachments, "--", tag)
    )
    uris = check_output(
        ("hub", "release", "show", "--format", "%as", "--", tag), text=True
    )
    return uris


def _build_j2() -> Environment:
    j2 = Environment(
        enable_async=True,
        trim_blocks=True,
        lstrip_blocks=True,
        undefined=StrictUndefined,
        loader=FileSystemLoader(_TOP_LEVEL / "ci" / "templates"),
    )
    return j2


@contextmanager
def _git_ops() -> Iterator[Path]:
    pkgs = _TOP_LEVEL / "packages"
    token = environ["CI_TOKEN"]
    uri = f"https://ms-jpq:{token}@github.com/ms-jpq/homebrew-sad"
    email = "ci@ci.ci"
    username = "ci-bot"
    time = datetime.now().strftime("%Y-%m-%d %H:%M")
    msg = f"CI - {time}"

    check_call(("git", "clone", "--depth=1", uri, pkgs), cwd=_TOP_LEVEL)
    check_call(("git", "config", "user.email", email), cwd=pkgs)
    check_call(("git", "config", "user.name", username), cwd=pkgs)
    yield pkgs
    check_call(("git", "add", "-A"), cwd=pkgs)
    check_call(("git", "commit", "-m", msg), cwd=pkgs)
    check_call(("git", "push", "--force"), cwd=pkgs)


def _template(brew_uri: str, project: _Project) -> None:
    j2 = _build_j2()
    with build_opener().open(brew_uri) as resp:
        brew_artifact = resp.read()

    sha = sha256(brew_artifact).hexdigest()
    vals = {**asdict(project), "sha256": sha, "release_uri": brew_uri}
    brew_rend = j2.get_template("homebrew.rb.j2").render(**vals)
    snap_rend = j2.get_template("snapcraft.yml.j2").render(**vals)

    with _git_ops() as cwd:
        (cwd / "sad.rb").write_text(brew_rend)
        (cwd / "snapcraft.yaml").write_text(snap_rend)


def main() -> None:
    project = _load_values()
    brew_uri = _release(project)
    print(brew_uri)
    # _template(brew_uri, project=project)


main()
