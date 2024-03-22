#!/usr/bin/env python3

from contextlib import contextmanager
from dataclasses import asdict, dataclass
from datetime import datetime
from hashlib import sha256
from itertools import chain, repeat
from os import environ, linesep, scandir
from os.path import normcase
from pathlib import Path
from subprocess import check_call
from sys import stderr
from time import sleep
from tomllib import loads
from typing import Iterator
from urllib.error import HTTPError
from urllib.request import build_opener

from jinja2 import Environment, FileSystemLoader, StrictUndefined
from yaml import safe_load


@dataclass(frozen=True)
class _Project:
    repo: str
    version: str
    tag: str
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
    tag = environ["GITHUB_REF"].removeprefix("refs/tags/")
    repo = environ["GITHUB_REPOSITORY"]
    repo_uri = f"https://github.com/{repo}"
    cargo = loads((_TOP_LEVEL / "Cargo.toml").read_text())
    vals = safe_load((_TOP_LEVEL / "ci" / "vars.yml").read_text())
    project = _Project(
        **{
            **vals,
            "repo": repo_uri,
            "version": cargo["package"]["version"],
            "tag": tag,
        }
    )
    return project


def _release(project: _Project) -> None:
    time = datetime.now().strftime("%Y-%m-%d_%H-%M")
    title = f"ci_{project.version}_{time}"
    body = (_TOP_LEVEL / "RELEASE_NOTES.md").read_text()
    message = f"{title}{linesep}{linesep}{body}"
    attachments = _walk(_TOP_LEVEL / "arts")

    check_call(
        (
            "gh",
            "release",
            "create",
            "--notes",
            message,
            "--",
            project.tag,
            *attachments,
        )
    )


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
    check_call(("git", "config", "--", "user.email", email), cwd=pkgs)
    check_call(("git", "config", "--", "user.name", username), cwd=pkgs)
    yield pkgs
    check_call(("git", "add", "-A"), cwd=pkgs)
    check_call(("git", "commit", "-m", msg), cwd=pkgs)
    check_call(("git", "push", "--force"), cwd=pkgs)


def _sha(uri: str) -> str:
    opener = build_opener()
    for _ in range(9):
        try:
            with opener.open(uri) as resp:
                body = resp.read()
        except HTTPError as e:
            print(uri, e, sep=linesep, file=stderr)
            sleep(9)
        else:
            sha = sha256(body).hexdigest()
            return sha
    else:
        raise TimeoutError()


def _template(project: _Project) -> None:
    prefix = f"{project.repo}/releases/download/{project.tag}"
    aarch64_uri = f"{prefix}/aarch64-apple-darwin.zip"
    x86_uri = f"{prefix}/x86_64-apple-darwin.zip"
    aarch64_sha = _sha(aarch64_uri)
    x86_sha = _sha(x86_uri)

    vals = {
        **asdict(project),
        "aarch64_uri": aarch64_uri,
        "aarch64_sha": aarch64_sha,
        "x86_sha": x86_sha,
        "x86_uri": x86_uri,
    }
    j2 = _build_j2()
    brew_rend = j2.get_template("homebrew.rb").render(**vals)
    snap_rend = j2.get_template("snapcraft.yml").render(**vals)

    with _git_ops() as cwd:
        (cwd / "sad.rb").write_text(brew_rend)
        (cwd / "snapcraft.yaml").write_text(snap_rend)


def main() -> None:
    project = _load_values()
    _release(project)
    _template(project)


main()
