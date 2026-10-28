"""Prepare a version-pinned local vanilla reference without Gradle.

Requires Python 3.11+ and Java suitable for the target. Downloads and sources
stay in ignored directories. Does not start Minecraft or accept its EULA.
"""

import hashlib
import json
import os
from pathlib import Path
import re
import shutil
import subprocess
import sys
import tempfile
import urllib.request
import zipfile

ROOT = Path(__file__).resolve().parent.parent
MANIFEST = "https://piston-meta.mojang.com/mc/game/version_manifest_v2.json"
VINEFLOWER_VERSION = "1.12.0"
VINEFLOWER_URL = (
    "https://repo.maven.apache.org/maven2/org/vineflower/vineflower/"
    f"{VINEFLOWER_VERSION}/vineflower-{VINEFLOWER_VERSION}.jar"
)


def read_url(url):
    with urllib.request.urlopen(url, timeout=120) as response:
        return response.read()


def download(url, destination, expected_hash, algorithm, expected_size=None):
    """Reuse verified cache entries and never publish partial downloads."""
    def valid(path):
        if not path.is_file():
            return False
        if expected_size is not None and path.stat().st_size != expected_size:
            return False
        with path.open("rb") as source:
            return hashlib.file_digest(source, algorithm).hexdigest() == expected_hash

    if valid(destination):
        return
    destination.parent.mkdir(parents=True, exist_ok=True)
    temporary = destination.with_suffix(destination.suffix + ".part")
    try:
        with urllib.request.urlopen(url, timeout=120) as response, temporary.open("wb") as output:
            shutil.copyfileobj(response, output)
        if not valid(temporary):
            raise ValueError(f"Checksum or size mismatch: {url}")
        temporary.replace(destination)
    finally:
        temporary.unlink(missing_ok=True)


def extract_sources(archive_path, destination):
    """Extract Java sources only, rejecting paths outside the output."""
    root = destination.resolve()
    count = 0
    with zipfile.ZipFile(archive_path) as archive:
        for entry in archive.infolist():
            if not entry.filename.endswith(".java"):
                continue
            target = (root / entry.filename).resolve()
            if not target.is_relative_to(root):
                raise ValueError(f"Unsafe archive entry: {entry.filename}")
            target.parent.mkdir(parents=True, exist_ok=True)
            with archive.open(entry) as source, target.open("wb") as output:
                shutil.copyfileobj(source, output)
            count += 1
    return count


def main():
    # Python 3.11/3.12's TOML 1.0 parser rejects Cargo's TOML 1.1 inline tables.
    manifest_text = (ROOT / "Cargo.toml").read_text(encoding="utf-8")
    package = re.search(r"(?ms)^\[workspace\.package\]\s*\n(.*?)(?=^\[|\Z)", manifest_text)
    version_match = re.search(r'^version\s*=\s*"([^"]+)"', package[1], re.M) if package else None
    if version_match is None:
        raise ValueError("Cannot read workspace.package.version from Cargo.toml")
    version = version_match[1]
    _, separator, target = version.partition("+mc")
    if not separator or not target:
        raise ValueError("Cargo.toml workspace version must include +mc<VERSION>")
    reference = ROOT / "minecraft-src"
    provenance_path = reference / ".vecta-reference.json"
    if reference.exists():
        if provenance_path.is_file():
            provenance = json.loads(provenance_path.read_text(encoding="utf-8"))
            if provenance["minecraft_version"] == target and (
                reference / "minecraft/src/net/minecraft/world/item/Item.java"
            ).is_file():
                print(f"Vanilla reference is already prepared for Minecraft {target}.")
                return
        raise ValueError(
            f"{reference} exists without a matching reference manifest. "
            "Move it aside before generating a new reference."
        )

    cache = ROOT / ".vecta-cache" / "vanilla" / target
    cache.mkdir(parents=True, exist_ok=True)
    manifest = json.loads(read_url(MANIFEST))
    entry = next((entry for entry in manifest["versions"] if entry["id"] == target), None)
    if entry is None:
        raise ValueError(f"Minecraft {target} is absent from the official manifest")
    details_path = cache / "version-details.json"
    download(entry["url"], details_path, entry["sha1"], "sha1")
    details = json.loads(details_path.read_text(encoding="utf-8"))
    server = details["downloads"]["server"]
    server_path = cache / "server-bundled.jar"
    print(f"Preparing official Minecraft {target} (Java {details['javaVersion']['majorVersion']}).", flush=True)
    download(server["url"], server_path, server["sha1"], "sha1", server["size"])

    with zipfile.ZipFile(server_path) as bundled:
        nested = [name for name in bundled.namelist()
                  if name.startswith("META-INF/versions/") and name.endswith(".jar")]
        if len(nested) != 1:
            raise ValueError("Expected exactly one server jar in the official bundle")
        input_path = cache / "minecraft-server.jar"
        input_path.write_bytes(bundled.read(nested[0]))
        libraries = []
        library_dir = cache / "libraries"
        library_dir.mkdir(exist_ok=True)
        for index, name in enumerate(bundled.namelist()):
            if name.startswith("META-INF/libraries/") and name.endswith(".jar"):
                library = library_dir / f"{index}.jar"
                library.write_bytes(bundled.read(name))
                libraries.append(f"-e={library}")

    with zipfile.ZipFile(input_path) as server_archive:
        version_info = json.loads(server_archive.read("version.json"))
        if version_info["id"] != target:
            raise ValueError("Official server version does not match Cargo.toml")
        if "net/minecraft/world/item/Item.class" not in server_archive.namelist():
            raise ValueError("This script requires an unobfuscated Minecraft server")

    decompiler = cache.parent / f"vineflower-{VINEFLOWER_VERSION}.jar"
    checksum = read_url(VINEFLOWER_URL + ".sha256").decode("ascii").strip().split()[0]
    download(VINEFLOWER_URL, decompiler, checksum, "sha256")
    java_home = os.environ.get("JAVA_HOME")
    java = str(Path(java_home) / "bin" / "java") if java_home else "java"
    with tempfile.TemporaryDirectory(prefix="reference-", dir=cache) as temporary:
        stage = Path(temporary)
        output = stage / "decompiled.jar"
        log_path = cache / "decompile.log"
        print(f"Decompiling the server; log: {log_path}", flush=True)
        with log_path.open("w", encoding="utf-8") as log:
            subprocess.run(
                [java, "-Xmx4G", "-jar", str(decompiler), "--file", "--log-level=WARN",
                 "--thread-count=4", *libraries, str(input_path), str(output)],
                stdout=log, stderr=subprocess.STDOUT, check=True,
            )
        staged_reference = stage / "minecraft-src"
        sources = staged_reference / "minecraft" / "src"
        count = extract_sources(output, sources)
        if not (sources / "net/minecraft/world/item/Item.java").is_file():
            raise ValueError(f"Expected Item.java is missing; inspect {log_path}")
        provenance = {
            "minecraft_version": target,
            "server_url": server["url"],
            "server_sha1": server["sha1"],
            "decompiler_version": VINEFLOWER_VERSION,
            "decompiler_sha256": checksum,
            "java_source_files": count,
        }
        (staged_reference / ".vecta-reference.json").write_text(
            json.dumps(provenance, indent=2) + "\n", encoding="utf-8"
        )
        shutil.move(str(staged_reference), reference)
    print(f"Prepared {count} Java source files in {reference}")


if __name__ == "__main__":
    try:
        main()
    except (OSError, ValueError, KeyError, subprocess.CalledProcessError, zipfile.BadZipFile) as error:
        sys.exit(f"Cannot prepare vanilla reference: {error}")
