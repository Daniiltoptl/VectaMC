"""Inventory generated dispatch and TODO markers; this is not a parity score.

Run cargo check --locked -p vecta-core before collecting the report. Generated
registrations are read as build output, never rewritten by this script.
"""

import argparse
import csv
import hashlib
import json
import pathlib
import re
import subprocess
from collections import defaultdict


def registrations(source, method, registry):
    pattern = (
        rf"registry\s*\.\s*{method}\s*\(\s*&\s*\*?\s*"
        rf"{registry}\s*::\s*(\w+)"
    )
    matches = re.findall(pattern, source)
    calls = re.findall(rf"registry\s*\.\s*{method}\s*\(", source)
    if len(matches) != len(calls) or len(matches) != len(set(matches)):
        raise ValueError(f"Cannot inventory every {registry} {method} call unambiguously")
    return set(matches)


def write_csv(path, fields, rows):
    with path.open("w", encoding="utf-8", newline="") as stream:
        writer = csv.DictWriter(stream, fieldnames=fields, lineterminator="\n")
        writer.writeheader()
        writer.writerows(rows)


def collect(root, output):
    inputs = {}

    def read(relative):
        path = root / relative
        data = path.read_bytes()
        inputs[relative] = hashlib.sha256(data).hexdigest()
        return data.decode("utf-8")

    cargo = read("Cargo.toml")
    package = cargo.split("[workspace.package]", 1)[1].split("\n[", 1)[0]
    version = re.search(r'^version\s*=\s*"([^"\n]+)"', package, re.MULTILINE)
    if not version or "+mc" not in version[1]:
        raise ValueError("workspace.package.version must contain +mc<version>")
    target = version[1].split("+mc", 1)[1]
    classes = json.loads(read("vecta-core/build/classes.json"))
    rows = []
    summaries = []
    missing_classes = {}
    generic = {("items", "Item"), ("blocks", "Block"), ("blocks", "AirBlock"), ("entities", "Player")}
    for kind, generated, method, registry in [
        ("items", "behavior/generated/items.rs", "set_behavior", "vanilla_items"),
        ("blocks", "behavior/generated/blocks.rs", "set_behavior", "vanilla_blocks"),
        ("entities", "entity/generated/entities.rs", "register", "vanilla_entities"),
    ]:
        relative = f"vecta-core/src/{generated}"
        registered = registrations(read(relative), method, registry)
        known = {entry["name"].upper() for entry in classes[kind]}
        if not registered or registered - known:
            raise ValueError(f"Unexpected generated registration format in {relative}; rebuild and inspect")
        missing = defaultdict(list)
        for entry in sorted(classes[kind], key=lambda item: item["name"]):
            present = entry["name"].upper() in registered
            note = "audit_existing_implementation" if present else "check_specialization_or_shared_path"
            if (kind, entry["class"]) in generic:
                note = "generic_or_separate_path"
            rows.append({
                "kind": kind, "name": entry["name"], "vanilla_class": entry["class"],
                "generated_registration": "yes" if present else "no", "review": note,
            })
            if not present:
                missing[entry["class"]].append(entry["name"])
        summaries.append((kind, len(classes[kind]), len(registered)))
        missing_classes[kind] = missing

    block_entities = json.loads(read("vecta-registry/build_assets/block_entities.json"))["block_entity_types"]
    registry_source = read("vecta-core/src/block_entity/registry.rs").split("pub fn init_block_entities()", 1)[1]
    registered = registrations(registry_source, "register", "vanilla_block_entity_types")
    known = {entry["name"].upper() for entry in block_entities}
    if not registered or registered - known:
        raise ValueError("Unexpected block entity registration format")
    block_entity_rows = [{
        "name": entry["name"],
        "registered_factory": "yes" if entry["name"].upper() in registered else "no",
        "valid_blocks": " ".join(entry["valid_blocks"]),
    } for entry in sorted(block_entities, key=lambda item: item["name"])]

    markers = []
    for crate in sorted(root.glob("vecta*")):
        for path in sorted((crate / "src").rglob("*.rs")):
            relative = path.relative_to(root)
            if "generated" in relative.parts or "tests" in relative.parts or "test" in path.stem:
                continue
            for line, content in enumerate(path.read_text(encoding="utf-8").splitlines(), 1):
                if re.search(r"\b(?:TODO|FIXME)\b|\b(?:todo|unimplemented)!\s*\(", content):
                    markers.append({"path": relative.as_posix(), "line": line, "marker": content.strip()})

    output.mkdir(parents=True, exist_ok=True)
    write_csv(output / "objects.csv", ["kind", "name", "vanilla_class", "generated_registration", "review"], rows)
    write_csv(output / "block_entities.csv", ["name", "registered_factory", "valid_blocks"], block_entity_rows)
    write_csv(output / "source_markers.csv", ["path", "line", "marker"], markers)
    revision = subprocess.run(["git", "rev-parse", "HEAD"], cwd=root, check=True, capture_output=True, text=True).stdout.strip()
    metadata = {"minecraft_version": target, "source_revision": revision, "inputs_sha256": inputs}
    (output / "provenance.json").write_text(json.dumps(metadata, indent=2) + "\n", encoding="utf-8")

    lines = [
        "# Каталог для проверки ванильной совместимости", "",
        f"Эталон: Minecraft Java **{target}**. Базовый коммит: `{revision}`.", "",
        "Это инвентаризация регистрации, а не процент готовности и не доказательство отсутствия всей механики.",
        "`yes` означает наличие сгенерированной регистрации; полноту реализации нужно проверять отдельно.",
        "`no` означает необходимость проверить специальный класс, общие компоненты и отдельные пути исполнения.",
        "Обычные `Item`, `Block`, воздух и создаваемый при входе `Player` не требуют отдельного поведения автоматически.",
        "Некоторые классы имеют только клиентские отличия; сверяйте серверные методы с эталоном.", "",
        "| Раздел | Записей в classes.json | Сгенерированных регистраций |", "| --- | ---: | ---: |",
    ]
    lines += [f"| {kind} | {total} | {count} |" for kind, total, count in summaries]
    lines += ["", "Полный поимённый каталог, включая уже зарегистрированные объекты: [objects.csv](objects.csv).", "",
        "Блок-сущности и наличие фабрик: [block_entities.csv](block_entities.csv).",
        "Незарегистрированные типы могут сохранять NBT через `UnimplementedBlockEntity`; это не игровая реализация.", "",
        f"Маркеры TODO/FIXME из исходников: [source_markers.csv](source_markers.csv), строк: {len(markers)}.",
        "Это текстовые указатели для разбора, включая потенциально устаревшие комментарии; один маркер не равен одной задаче.", "",
        "## Классы без сгенерированного назначения", "",
    ]
    for kind, missing in missing_classes.items():
        lines += [f"### {kind}", "", "| Класс | Имена реестра |", "| --- | --- |"]
        lines += [f"| `{name}` | {', '.join(f'`{item}`' for item in entries)} |" for name, entries in sorted(missing.items())]
        lines.append("")
    lines += ["## Обновление", "", "Из корня репозитория:", "", "```sh", "cargo check --locked -p vecta-core", "python scripts/audit_vanilla.py", "```", "",
        "Скрипт читает результаты генераторов. После изменения регистраций сначала пересоберите проект.",
        "Исходные JSON Minecraft и сгенерированные Rust-файлы скрипт не изменяет.",
        "Хеши входных каталогов и результатов генерации сохранены в [provenance.json](provenance.json).", "",
    ]
    (output / "README.md").write_text("\n".join(lines), encoding="utf-8")
    return len(rows), len(block_entity_rows), len(markers)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--output", type=pathlib.Path, default=pathlib.Path("planning/audit"))
    args = parser.parse_args()
    root = pathlib.Path(__file__).resolve().parent.parent
    output = args.output if args.output.is_absolute() else root / args.output
    try:
        counts = collect(root, output)
    except (OSError, ValueError, KeyError, subprocess.CalledProcessError) as error:
        parser.exit(1, f"Audit failed: {error}. Run cargo check --locked -p vecta-core first.\n")
    print(f"Wrote {counts[0]} objects, {counts[1]} block entity types, {counts[2]} source markers to {output}")


if __name__ == "__main__":
    main()
