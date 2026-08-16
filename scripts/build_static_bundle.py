#!/usr/bin/env python3
"""Build a lean static SCIP 10.0.2 + SoPlex release bundle."""

from __future__ import annotations

import argparse
import hashlib
import json
import os
import platform
from pathlib import Path
import shutil
import subprocess
import sys
import tarfile
import tempfile
import urllib.request
import zipfile


SCIP_VERSION = "10.0.2"
SOURCE_URL = (
    "https://github.com/scipopt/scip/releases/download/"
    f"v{SCIP_VERSION}/scipoptsuite-{SCIP_VERSION}.tgz"
)
SOURCE_SHA256 = "eecc29f31e8c8a3089c95ef99dd310d05e1546ba40f4ff36551d75a5f5c47073"
TARGETS = ("linux", "linux-arm", "macos-intel", "macos-arm", "windows")

CMAKE_OPTIONS = {
    "AMPL": "OFF",
    "AUTOBUILD": "OFF",
    "BOOST": "OFF",
    "BUILD_SHARED_LIBS": "OFF",
    "BUILD_TESTING": "OFF",
    "CMAKE_BUILD_TYPE": "Release",
    "CMAKE_INSTALL_LIBDIR": "lib",
    "CMAKE_POSITION_INDEPENDENT_CODE": "ON",
    "CONOPT": "OFF",
    "EXACTSOLVE": "OFF",
    "GCG": "OFF",
    "GMP": "OFF",
    "IPOPT": "OFF",
    "LAPACK": "OFF",
    "LPS": "spx",
    "LPSEXACT": "none",
    "LTO": "OFF",
    "MPFR": "OFF",
    "PAPILO": "OFF",
    "QUADMATH": "OFF",
    "READLINE": "OFF",
    "SHARED": "OFF",
    "SOPLEX": "ON",
    "SYM": "snauty",
    "TPI": "tny",
    "UG": "OFF",
    "WORHP": "OFF",
    "ZIMPL": "OFF",
    "ZLIB": "OFF",
}


def run(command: list[str], *, cwd: Path | None = None) -> None:
    print("+", " ".join(command), flush=True)
    subprocess.run(command, cwd=cwd, check=True)


def sha256(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as source:
        for chunk in iter(lambda: source.read(1024 * 1024), b""):
            digest.update(chunk)
    return digest.hexdigest()


def download_source(destination: Path) -> None:
    print(f"Downloading {SOURCE_URL}", flush=True)
    with (
        urllib.request.urlopen(SOURCE_URL, timeout=300) as response,
        destination.open("wb") as output,
    ):
        shutil.copyfileobj(response, output)
    actual = sha256(destination)
    if actual != SOURCE_SHA256:
        raise RuntimeError(
            f"SCIP source SHA-256 mismatch: expected {SOURCE_SHA256}, got {actual}"
        )


def safe_extract(archive_path: Path, destination: Path) -> None:
    destination = destination.resolve()
    with tarfile.open(archive_path, "r:gz") as archive:
        for member in archive.getmembers():
            extracted = (destination / member.name).resolve()
            if destination != extracted and destination not in extracted.parents:
                raise RuntimeError(f"unsafe path in SCIP source archive: {member.name}")
        archive.extractall(destination)


def find_archive(lib_dir: Path, names: tuple[str, ...]) -> Path:
    matches = [lib_dir / name for name in names if (lib_dir / name).is_file()]
    if len(matches) != 1:
        expected = ", ".join(names)
        found = ", ".join(path.name for path in lib_dir.iterdir())
        raise RuntimeError(
            f"expected exactly one of [{expected}] in {lib_dir}; found [{found}]"
        )
    return matches[0]


def copy_tree(source: Path, destination: Path) -> None:
    if destination.exists():
        shutil.rmtree(destination)
    shutil.copytree(source, destination)


def stage_bundle(install_dir: Path, stage_root: Path, target: str) -> None:
    source_lib = install_dir / "lib"
    if not source_lib.is_dir() and (install_dir / "lib64").is_dir():
        source_lib = install_dir / "lib64"
    if not source_lib.is_dir():
        raise RuntimeError(f"SCIP install has neither lib nor lib64: {install_dir}")

    bundle = stage_root / "scip_install"
    bundle_lib = bundle / "lib"
    bundle_lib.mkdir(parents=True)

    if target == "windows":
        scip = find_archive(source_lib, ("libscip.lib",))
        soplex = find_archive(source_lib, ("libsoplex.lib",))
    else:
        scip = find_archive(source_lib, ("libscip.a",))
        soplex = find_archive(source_lib, ("libsoplex.a",))
    shutil.copy2(scip, bundle_lib / scip.name)
    shutil.copy2(soplex, bundle_lib / soplex.name)

    include_dir = install_dir / "include"
    if not include_dir.is_dir():
        raise RuntimeError(f"SCIP install has no include directory: {install_dir}")
    copy_tree(include_dir, bundle / "include")

    installed_licenses = install_dir / "share" / "licenses"
    if installed_licenses.is_dir():
        copy_tree(installed_licenses, bundle / "licenses")
    else:
        raise RuntimeError(f"SCIP install has no licenses: {installed_licenses}")

    metadata = {
        "build_type": "Release",
        "cmake_options": CMAKE_OPTIONS,
        "scip_version": SCIP_VERSION,
        "source_sha256": SOURCE_SHA256,
        "target_tag": target,
    }
    workflow_commit = os.environ.get("GITHUB_SHA")
    if workflow_commit:
        metadata["workflow_commit"] = workflow_commit
    (bundle / "bundle-info.json").write_text(
        json.dumps(metadata, indent=2, sort_keys=True) + "\n", encoding="utf-8"
    )


def audit_bundle(stage_root: Path, target: str) -> None:
    bundle = stage_root / "scip_install"
    lib_dir = bundle / "lib"
    if target == "windows":
        find_archive(lib_dir, ("libscip.lib",))
        find_archive(lib_dir, ("libsoplex.lib",))
    else:
        find_archive(lib_dir, ("libscip.a",))
        find_archive(lib_dir, ("libsoplex.a",))

    forbidden_suffixes = (".dylib", ".dll", ".so")
    forbidden = []
    for path in bundle.rglob("*"):
        name = path.name.lower()
        if path.is_file() and (
            name.endswith(forbidden_suffixes) or ".so." in name
        ):
            forbidden.append(path.relative_to(stage_root).as_posix())
    if forbidden:
        raise RuntimeError(
            "dynamic libraries found in static SCIP bundle: " + ", ".join(forbidden)
        )


def write_reproducible_zip(stage_root: Path, output: Path) -> None:
    output.parent.mkdir(parents=True, exist_ok=True)
    with zipfile.ZipFile(
        output, "w", compression=zipfile.ZIP_DEFLATED, compresslevel=9
    ) as archive:
        for path in sorted(stage_root.rglob("*")):
            if not path.is_file():
                continue
            relative = path.relative_to(stage_root).as_posix()
            info = zipfile.ZipInfo(relative, date_time=(1980, 1, 1, 0, 0, 0))
            info.create_system = 3
            info.compress_type = zipfile.ZIP_DEFLATED
            info.external_attr = 0o644 << 16
            archive.writestr(info, path.read_bytes(), compresslevel=9)


def build(target: str, output_dir: Path) -> Path:
    host = (platform.system().lower(), platform.machine().lower())
    host_targets = {
        ("darwin", "arm64"): "macos-arm",
        ("darwin", "x86_64"): "macos-intel",
        ("linux", "aarch64"): "linux-arm",
        ("linux", "arm64"): "linux-arm",
        ("linux", "x86_64"): "linux",
        ("windows", "amd64"): "windows",
        ("windows", "x86_64"): "windows",
    }
    detected = host_targets.get(host)
    if detected != target:
        raise RuntimeError(
            f"target tag {target!r} does not match native host {host[0]}-{host[1]}"
        )

    with tempfile.TemporaryDirectory(prefix="scip-static-bundle-") as temporary:
        work = Path(temporary)
        archive = work / f"scipoptsuite-{SCIP_VERSION}.tgz"
        source_parent = work / "source"
        source_parent.mkdir()
        download_source(archive)
        safe_extract(archive, source_parent)
        source = source_parent / f"scipoptsuite-{SCIP_VERSION}"
        build_dir = work / "build"
        install_dir = work / "install"

        configure = [
            "cmake",
            "-S",
            str(source),
            "-B",
            str(build_dir),
            "-G",
            "Ninja",
            f"-DCMAKE_INSTALL_PREFIX={install_dir}",
        ]
        configure.extend(f"-D{name}={value}" for name, value in CMAKE_OPTIONS.items())
        run(configure)
        run(
            [
                "cmake",
                "--build",
                str(build_dir),
                "--config",
                "Release",
                "--target",
                "install",
            ]
        )

        stage_root = work / "stage"
        stage_bundle(install_dir, stage_root, target)
        audit_bundle(stage_root, target)
        output = output_dir.resolve() / f"libscip-{target}.zip"
        write_reproducible_zip(stage_root, output)
        print(f"Created {output} ({output.stat().st_size} bytes)", flush=True)
        print(f"SHA-256 {sha256(output)}", flush=True)
        return output


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("target", choices=TARGETS, help="bundle target tag")
    parser.add_argument(
        "--output-dir",
        type=Path,
        default=Path("dist"),
        help="directory for libscip-<target>.zip (default: dist)",
    )
    return parser.parse_args()


def main() -> int:
    arguments = parse_args()
    try:
        build(arguments.target, arguments.output_dir)
    except (OSError, RuntimeError, subprocess.CalledProcessError) as error:
        print(f"error: {error}", file=sys.stderr)
        return 1
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
