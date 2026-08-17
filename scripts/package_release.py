#!/usr/bin/env python3
"""Create portable Eagle Editor folders and GitHub-ready archives."""

from __future__ import annotations

import argparse
import hashlib
import os
from pathlib import Path
import shutil
import subprocess
import tarfile
import zipfile


ROOT = Path(__file__).resolve().parents[1]
DEFAULT_OUTPUT = ROOT / "release" / "github-upload"


def version_from_cargo() -> str:
    for line in (ROOT / "Cargo.toml").read_text(encoding="utf-8").splitlines():
        if line.startswith("version = "):
            return line.split('"', 2)[1]
    raise RuntimeError("Could not read the package version from Cargo.toml")


def copy_runtime_assets(destination: Path) -> None:
    source = ROOT / "assets"
    assets = destination / "assets"
    shutil.copytree(source / "icons", assets / "icons", dirs_exist_ok=True)
    shutil.copytree(source / "player_vehicle", assets / "player_vehicle", dirs_exist_ok=True)


def package_blender_addon(destination: Path) -> None:
    source = ROOT / "blender_addon" / "eagle_mta_toolkit"
    archive = destination / "eagle_mta_toolkit.zip"
    with zipfile.ZipFile(archive, "w", zipfile.ZIP_DEFLATED) as zipped:
        for path in sorted(source.rglob("*")):
            if path.is_file() and "__pycache__" not in path.parts:
                zipped.write(path, Path("eagle_mta_toolkit") / path.relative_to(source))


def copy_mingw_runtime(destination: Path) -> None:
    """Bundle DLLs required by the GNU Windows cross-build, when applicable."""
    compiler = shutil.which("x86_64-w64-mingw32-g++")
    if not compiler:
        return
    for name in ("libstdc++-6.dll", "libgcc_s_seh-1.dll"):
        result = subprocess.run(
            [compiler, f"-print-file-name={name}"],
            check=True,
            capture_output=True,
            text=True,
        )
        path = Path(result.stdout.strip())
        if path.is_file():
            shutil.copy2(path, destination / name)


def stage(platform: str, binary: Path, output: Path, version: str) -> Path:
    folder = output / f"EagleEditor-v{version}-{platform}"
    if folder.exists():
        shutil.rmtree(folder)
    folder.mkdir(parents=True)
    executable_name = "EagleEditor.exe" if platform.startswith("windows") else "EagleEditor"
    target = folder / executable_name
    shutil.copy2(binary, target)
    if not platform.startswith("windows"):
        target.chmod(target.stat().st_mode | 0o111)
    copy_runtime_assets(folder)
    package_blender_addon(folder)
    shutil.copy2(ROOT / "RELEASE_README.md", folder / "README.md")
    shutil.copy2(ROOT / "LICENSE", folder / "LICENSE")
    shutil.copy2(ROOT / "THIRD_PARTY_NOTICES.md", folder / "THIRD_PARTY_NOTICES.md")
    licenses = folder / "LICENSES"
    licenses.mkdir()
    shutil.copy2(ROOT / "LICENSES" / "OFL-1.1.txt", licenses / "OFL-1.1.txt")
    shutil.copy2(
        ROOT / "vendor" / "miniquad" / "LICENSE-MIT",
        licenses / "MINIQUAD-LICENSE-MIT",
    )
    shutil.copy2(
        ROOT / "vendor" / "miniquad" / "LICENSE-APACHE",
        licenses / "MINIQUAD-LICENSE-APACHE",
    )
    if platform.startswith("windows") and os.name != "nt":
        copy_mingw_runtime(folder)
    (folder / "RUNNING.txt").write_text(
        "Eagle Editor\n\n"
        + ("Run EagleEditor.exe.\n" if platform.startswith("windows") else "Run ./EagleEditor.\n")
        + "Choose an Eagle resource in the Project Manager, or pass its folder on the command line.\n",
        encoding="utf-8",
    )
    return folder


def archive_folder(folder: Path) -> Path:
    if "windows" in folder.name.lower():
        archive = folder.parent / f"{folder.name}.zip"
        if archive.exists():
            archive.unlink()
        with zipfile.ZipFile(archive, "w", zipfile.ZIP_DEFLATED) as zipped:
            for path in sorted(folder.rglob("*")):
                if path.is_file():
                    zipped.write(path, path.relative_to(folder.parent))
    else:
        archive = folder.parent / f"{folder.name}.tar.gz"
        if archive.exists():
            archive.unlink()
        with tarfile.open(archive, "w:gz") as tar:
            tar.add(folder, arcname=folder.name)
    return archive


def bundle(output: Path, version: str) -> None:
    for pattern in ("EagleEditor-v*.zip", "EagleEditor-v*.tar.gz", "SHA256SUMS.txt"):
        for stale in output.glob(pattern):
            stale.unlink()
    archives = [
        archive_folder(folder)
        for folder in sorted(output.glob(f"EagleEditor-v{version}-*"))
        if folder.is_dir()
    ]
    checksums = []
    for archive in archives:
        digest = hashlib.sha256(archive.read_bytes()).hexdigest()
        checksums.append(f"{digest}  {archive.name}")
    (output / "SHA256SUMS.txt").write_text("\n".join(checksums) + "\n", encoding="utf-8")


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("--platform", choices=("linux-x86_64", "windows-x86_64"))
    parser.add_argument("--binary", type=Path)
    parser.add_argument("--output", type=Path, default=DEFAULT_OUTPUT)
    parser.add_argument("--version", default=version_from_cargo())
    parser.add_argument("--bundle-only", action="store_true")
    args = parser.parse_args()
    args.output.mkdir(parents=True, exist_ok=True)
    if not args.bundle_only:
        if not args.platform or not args.binary:
            parser.error("--platform and --binary are required unless --bundle-only is used")
        if not args.binary.is_file():
            parser.error(f"binary does not exist: {args.binary}")
        stage(args.platform, args.binary.resolve(), args.output, args.version)
    else:
        bundle(args.output, args.version)


if __name__ == "__main__":
    main()
