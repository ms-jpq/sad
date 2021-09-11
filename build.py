#!/usr/bin/env python3

from argparse import ArgumentParser, Namespace
from contextlib import nullcontext
from locale import strxfrm
from pathlib import Path, PurePath
from platform import uname
from shutil import which
from subprocess import check_call
from zipfile import ZipFile

from jinja2 import Environment, FileSystemLoader, StrictUndefined
from toml import loads

_TOP_LEVEL = Path(__file__).resolve().parent

_DEFAULTS = {
    "Linux": ("unknown-linux", "musl" if which("apk") else "gnu"),
    "Darwin": ("apple", "darwin"),
    "Windows": ("pc-windows", "gnu"),
}

_TOOL_CHAINS = {
    "aarch64-apple-darwin",
    "aarch64-unknown-linux-gnu",
    "aarch64-unknown-linux-musl",
    "x86_64-apple-darwin",
    "x86_64-pc-windows-gnu",
    "x86_64-unknown-linux-gnu",
    "x86_64-unknown-linux-musl",
}

UNAME = uname()


def _j2(src: PurePath) -> Environment:
    j2 = Environment(
        enable_async=True,
        trim_blocks=True,
        lstrip_blocks=True,
        undefined=StrictUndefined,
        loader=FileSystemLoader(src),
    )
    return j2


def _deps() -> None:
    if UNAME.system == "Linux" and which("apt"):
        check_call(("sudo", "apt", "update"), cwd=_TOP_LEVEL)
        check_call(
            ("sudo", "apt", "install", "--yes", "--", "gcc-mingw-w64"), cwd=_TOP_LEVEL
        )
    for toolchain in sorted(_TOOL_CHAINS, key=strxfrm):
        check_call(("rustup", "target", "add", "--", toolchain), cwd=_TOP_LEVEL)


def _build(triple: str) -> None:
    check_call(("cargo", "test"), cwd=_TOP_LEVEL)
    check_call(
        ("cargo", "build", "--locked", "--release", "--target", triple),
        cwd=_TOP_LEVEL,
    )


def _archive(triple: str) -> None:
    suffix = ".exe" if "windows" in triple else ""
    raw = _TOP_LEVEL / "target" / triple / "release" / "sad"
    release = raw.with_suffix(suffix)
    archive = (_TOP_LEVEL / "artifacts" / triple).with_suffix(".zip")
    with ZipFile(archive, mode="w") as fd:
        fd.write(release)


def _parse_args() -> Namespace:
    arch_choices = {"x86_64", "aarch64"}
    os_choices = {os for (os, _) in _DEFAULTS.values()}
    compiler_choices = {"musl", "gnu", "darwin"}
    os, compiler = _DEFAULTS.get(UNAME.system) or (None, None)

    parser = ArgumentParser()
    sub_parser = parser.add_subparsers(dest="action", required=True)

    with nullcontext(sub_parser.add_parser("deps")) as p:
        pass

    with nullcontext(sub_parser.add_parser("build")) as p:
        p.add_argument(
            "-r",
            "--release",
            action="store_true",
        )

        p.add_argument(
            "--arch",
            choices=sorted(arch_choices, key=strxfrm),
            default=UNAME.machine,
        )
        p.add_argument(
            "--os",
            required=not bool(os),
            choices=sorted(os_choices, key=strxfrm),
            default=os,
        )
        p.add_argument(
            "--compiler",
            required=not bool(compiler),
            choices=sorted(compiler_choices, key=strxfrm),
            default=compiler,
        )

    with nullcontext(sub_parser.add_parser("buildr")) as p:
        p.add_argument("triple", choices=sorted(_TOOL_CHAINS, key=strxfrm))

    return parser.parse_args()


def main() -> None:
    args = _parse_args()
    if args.action == "deps":
        _deps()

    elif args.action == "build":
        triple = "-".join((args.arch, args.os, args.compiler))
        assert triple in _TOOL_CHAINS
        _build(triple)
        _archive(triple)

    elif args.action == "buildr":
        _build(args.triple)
        _archive(args.triple)

    else:
        assert False


main()
