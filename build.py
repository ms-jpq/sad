#!/usr/bin/env python3

from argparse import ArgumentParser, Namespace
from contextlib import nullcontext, suppress
from locale import strxfrm
from pathlib import Path
from platform import uname
from shutil import copy2, rmtree, which
from subprocess import check_call
from zipfile import ZipFile

from jinja2 import Environment, FileSystemLoader, StrictUndefined
from toml import loads

_TOP_LEVEL = Path(__file__).resolve().parent
_ARTS = _TOP_LEVEL / "artifacts"

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

_DPKG_ARCH = {
    "x86_64": "amd64",
    "aarch64": "aarch64",
}

UNAME = uname()


def _deps() -> None:
    if UNAME.system == "Linux" and which("apt"):
        check_call(("sudo", "apt", "update"), cwd=_TOP_LEVEL)
        check_call(
            (
                "sudo",
                "apt",
                "install",
                "--yes",
                "--",
                "gcc-mingw-w64",  # windows
            ),
            cwd=_TOP_LEVEL,
        )
    for toolchain in sorted(_TOOL_CHAINS, key=strxfrm):
        check_call(("rustup", "target", "add", "--", toolchain), cwd=_TOP_LEVEL)


def _compile(triple: str) -> None:
    check_call(("cargo", "test", "--locked"), cwd=_TOP_LEVEL)
    check_call(
        ("cargo", "build", "--locked", "--release", "--target", triple),
        cwd=_TOP_LEVEL,
    )


def _bin_path(triple: str) -> Path:
    suffix = ".exe" if "windows" in triple else ""
    release = _TOP_LEVEL / "target" / triple / "release" / "sad"
    return release.with_suffix(suffix)


def _archive(triple: str) -> None:
    release = _bin_path(triple)
    archive = (_ARTS / triple).with_suffix(".zip")
    with ZipFile(archive, mode="w") as fd:
        fd.write(release, arcname=release.name)


def _deb(triple: str) -> None:
    arch, _, _ = triple.partition("-")
    cargo = _TOP_LEVEL / "Cargo.toml"
    templates = _TOP_LEVEL / "templates"

    release = _bin_path(triple)
    tmp = _TOP_LEVEL / "temp" / triple

    sad = tmp / "usr" / "local" / "bin" / "sad"
    control = tmp / "DEBIAN" / "control"
    deb = (_ARTS / triple).with_suffix(".deb")

    j2 = Environment(
        enable_async=True,
        trim_blocks=True,
        lstrip_blocks=True,
        undefined=StrictUndefined,
        loader=FileSystemLoader(templates),
    )

    env = {**loads(cargo.read_text())["package"], "arch": _DPKG_ARCH[arch]}
    ctrl = j2.get_template("control").render(env)

    with suppress(FileNotFoundError):
        rmtree(tmp)
    for path in (sad, control):
        path.parent.mkdir(parents=True, exist_ok=True)
    control.write_text(ctrl)
    copy2(release, sad)

    if which("dpkg-deb"):
        check_call(
            ("dpkg-deb", "--root-owner-group", "--build", tmp, deb),
            cwd=_TOP_LEVEL,
        )


def _build(triple: str) -> None:
    assert triple in _TOOL_CHAINS
    _compile(triple)
    _archive(triple)
    _deb(triple)


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
        _build(triple)

    elif args.action == "buildr":
        _build(args.triple)

    else:
        assert False


main()
