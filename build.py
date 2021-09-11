#!/usr/bin/env python3

from argparse import ArgumentParser, Namespace
from contextlib import nullcontext
from locale import strxfrm
from pathlib import Path
from platform import uname
from shutil import which
from subprocess import check_call

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


def _deps() -> None:
    if which("apt"):
        check_call(("apt", "update"), cwd=_TOP_LEVEL)
        check_call(("apt", "install", "--yes", "--", "gcc-mingw-w64"), cwd=_TOP_LEVEL)
    for toolchain in sorted(_TOOL_CHAINS, key=strxfrm):
        check_call(("rustup", "target", "add", "--", toolchain), cwd=_TOP_LEVEL)


def _build(triple: str, release: bool) -> None:
    check_call(
        (
            "cargo",
            "build",
            "--target",
            triple,
            *(("--release",) if release else ()),
        ),
        cwd=_TOP_LEVEL,
    )


def _parse_args() -> Namespace:
    un = uname()
    arch_choices = {"x86_64", "aarch64"}
    os_choices = {os for (os, _) in _DEFAULTS.values()}
    compiler_choices = {"musl", "gnu", "darwin"}
    os, compiler = _DEFAULTS.get(un.system) or (None, None)

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
            default=un.machine,
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

    return parser.parse_args()


def main() -> None:
    args = _parse_args()
    if args.action == "deps":
        _deps()

    elif args.action == "build":
        triple = "-".join((args.arch, args.os, args.compiler))
        assert triple in _TOOL_CHAINS
        _build(triple, release=args.release)

    else:
        assert False


main()
