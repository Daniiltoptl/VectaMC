# Блокеры агента B, сеанс 12

Статус: **останов до игрового кода** по `planning/night-06b/BRIEF.md`,
раздел «Java-исходники (первый шаг)». EULA не принималась, ванильный сервер не
запускался.

Текущая ветка Arena: `arena/a8ed48ae-vectamc`. Исходный коммит сессии —
`a2db838b68ac4e21d9626aa1f8b146126911f2b3`; текущая ветка переведена вперёд
`git merge --ff-only` до `d92de08551ae2913a27798c1b871d07d97790fef`
(`origin/codex/night-vanilla-03-b`), чтобы получить код пакета 06 и бриф без
`checkout` другой ветки. Пуш выполняется только в текущую ветку Arena.

## B-06b-01 · Первичный файл `AGENT_B_06_12.md` отсутствовал — снят

Проверено: `/home/user/VectaMC`, все удалённые ветки `Daniiltoptl/VectaMC` и
GitHub code search. Пути `C:\Users\My comp\Desktop\Vecta-b` и
`C:\Users\My comp\Desktop\Vecta\minecraft-src\minecraft\` в облачной среде не
существуют. Пользователь указал вместо отсутствующего файла прочитать полностью
`planning/night-06b/BRIEF.md` и `planning/night-06b/CHECKLIST.md` из
`origin/codex/night-vanilla-03-b`. Оба файла прочитаны полностью.

## B-06b-02 · Не удалось подготовить `minecraft-src` — главный блокер

Команда из брифа:

```sh
./update-minecraft-src.sh
```

Фактический вывод:

```text
Python 3.11.2
/bin/bash: line 3: java: command not found
exit=1
Cannot prepare vanilla reference: <urlopen error TLS/SSL connection has been closed (EOF) (_ssl.c:992)>
```

Дополнительная проверка доступности из Python `urllib`:

```text
https://piston-meta.mojang.com/mc/game/version_manifest_v2.json URLError <urlopen error TLS/SSL connection has been closed (EOF) (_ssl.c:992)>
https://repo.maven.apache.org/maven2/org/vineflower/vineflower/1.12.0/vineflower-1.12.0.jar URLError <urlopen error TLS/SSL connection has been closed (EOF) (_ssl.c:992)>
https://github.com 200 None
```

Причины:

1. В песочнице исходящая сеть разрешена только для `github.com`,
   `codeload.github.com`, `api.github.com`, `registry.npmjs.org`, `pypi.org` и
   `files.pythonhosted.org`. Хосты официального manifest и Maven с Vineflower
   закрыты, поэтому `scripts/prepare_vanilla.py` не может скачать version
   manifest, server bundle и декомпилятор.
2. Исполняемый `java` отсутствует (`java: command not found`), поэтому даже при
   наличии кэша декомпиляция Vineflower невозможна.
3. `.vecta-cache/vanilla/26.3/` пуст; `minecraft-src/` и
   `minecraft-src/.vecta-reference.json` не созданы. Полных Java-исходников
   26.3 в репозитории нет: найдены только три вспомогательных файла
   `scripts/steel-extractor/*.java`, они не являются эталоном.

Следствие: `AGENTS.md` («VANILLA TARGET VERSION», «VERIFY BEFORE ASKING») и
`BRIEF.md` запрещают писать игровой код без локальных исходников 26.3. Все
области агента B — G101–G104, G105–G107, G116, G117, интерактивные блоки,
G095/G119, команды и protocol coverage — остановлены. Не начата даже первая
карточка `W06-501` (`RedstoneWireBlock.java` / connections), потому что её
контракт надо читать в Java.

Что нужно для снятия: доступ к `piston-meta.mojang.com` и
`repo.maven.apache.org` плюс Java нужной мажорной версии, либо уже
подготовленный `minecraft-src/` с корректным `.vecta-reference.json` для
`0.16.1+mc26.3`, скопированный в корень репозитория. Ручная замена Java
исходников, догадки по методам и правки extracted данных не допускаются.

## B-06b-03 · Отсутствует Rust toolchain

```text
rustc: command not found
cargo: command not found
rustup: command not found
cargo fmt --all --check: exit 127
cargo metadata --locked --no-deps: exit 127
```

Поэтому полный `cargo test --locked --workspace`, Clippy и генерация
`vecta-core/src/behavior/generated/items.rs` не запускались. Итог всего
workspace в этой среде **не получен**; прежний `4402 passed / 0 failed /
18 ignored` из брифа не подтверждён новым прогоном и не выдаётся за результат
сеанса. `python scripts/audit_vanilla.py` завершился так:

```text
exit=1
Audit failed: [Errno 2] No such file or directory: '/home/user/VectaMC/vecta-core/src/behavior/generated/items.rs'. Run cargo check --locked -p vecta-core first.
```

## B-06b-04 · SteelExtractor и protocol coverage

Локального checkout SteelExtractor в облаке нет; `run/steel_extractor_output/`
отсутствует. Файл `planning/audit/protocol-coverage.md` в актуальном дереве
тоже отсутствует. Новые extracted JSON и protocol ID вручную не вписывались.
Покрытие протокола остаётся незакрытым пунктом брифа.

## B-06b-05 · Ограничение ветки Arena

`BRIEF.md` и пользователь просили `git checkout codex/night-vanilla-03-b` и пуш
только в неё. Arena закрепляет за сессией `arena/a8ed48ae-vectamc` и запрещает
переключаться/пушить в другую ветку. Поэтому объекты запрошенной ветки были
получены через `git fetch`, а работа ведётся и пушится на текущей ветке.
В `codex/night-vanilla-03`, `codex/night-vanilla-03-b` и `main` пушей,
merge-коммитов, rebase и force-push не делалось.
