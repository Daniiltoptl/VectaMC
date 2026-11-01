# Общие RandomSequences и последовательные тики миров

Дата: 2026-11-01. База: `12a196296`, ветка `codex/night-vanilla-03`.
Эталон: Java 26.3, локальный manifest соответствует workspace `0.16.1+mc26.3`.

## Согласованное поведение

Владелец выбрал один серверный owner для named RNG, как у MinecraftServer,
а затем явно разрешил последовательные игровые тики миров. Обе архитектурные
развилки закрыты. Копии банка на каждый domain не создаются.

- `RandomSequences` принадлежит `Server`; все hosted worlds получают общий Arc
  до старта gameplay. Seed берётся у server default world; live Level.random
  остаётся отдельным у каждого мира. Standalone test worlds имеют свой owner
  до server binding.
- `WorldTickWorkers.tick_all` отправляет следующий request только после ответа
  предыдущего worker. Ошибка текущего мира останавливает дальнейший dispatch.
  Coordinator продолжает использовать существующие worker threads; игровые
  world ticks не перекрываются. Фоновая генерация, chunk encoding и I/O не менялись.
- `WorldMap` сохраняет порядок вставки независимо от перехода SmallMap на heap.
  Lookup остаётся в карте; замена по ключу сохраняет место. Значения, ключи,
  пары и выборка worlds одного domain используют один порядок.
- Продолжение RNG хранится в `data/random_sequences.toml` server default world
  через существующий `SavedDataManager`. Сохраняются signed seed words,
  salt и два seed-default flags. Это формат Vecta, не импорт/экспорт Mojang NBT.
  RAM-only storage сохраняет прежнюю ephemeral семантику SavedDataManager.
- Autosave работает в существующей фоновой задаче, shutdown дожидается её
  завершения и сохраняет актуальный state. Snapshot не держит RNG mutex во время
  I/O. Draw после snapshot остаётся dirty; неудачная запись не подтверждается.

Сверено с `MinecraftServer` (LinkedHashMap levels, tickChildren, getRandomSequence),
`RandomSequences`, `RandomSequence`, `XoroshiroRandomSource` и его codec.

## Проверки

Ранее шесть RandomSequences tests прошли в рабочем дереве ревью:
disk save/reload, snapshot race, I/O failure, signed seed words и defaults,
невалидный key, общий поток двух настоящих worlds из разных domains.

Новые regressions проверяют dispatch через реальные request/response channels
с ручным завершением настоящего `World.tick_game`, без sleeps и зависимости от
планировщика ОС: второй request отсутствует до ответа первого, включая frozen
итерацию; missing response не запускает последующие worlds. Отдельный тест
проверяет порядок десяти worlds после heap spill и замены существующего world.
Сохранён runtime test с настоящими worker threads на нескольких тиках.

Первый compile нового WorldMap test выявил пропущенный test-only импорт Arc;
импорт исправлен. После этого:

- `cargo test --locked -p vecta-core --lib server::world -j 1` — **7 passed**,
  exit 0, включая настоящие workers, ordered dispatch и десять worlds с заменой.
- `cargo test --locked -p vecta-core --lib server::random_sequences -j 1` —
  **6 passed**, exit 0, повторно после изменения WorldMap и dispatch.
- `cargo fmt --all --check` — exit 0.
- `python -m unittest discover -s scripts` — **4 passed**, exit 0.
- `cargo test --locked --workspace -j 1` — **4105 passed / 0 failed / 18 ignored**,
  exit 0; сборка 4m39s. Это общий прогон всех изменений, включая RNG и tick ordering.

- `cargo clippy --locked --workspace --all-targets --all-features -j 1 -- -D warnings`
  — exit 0, 1m36s. Сохранилось прежнее Cargo manifest warning об unused mimalloc
  при dhat-heap; lint checks не подавлялись.
- `python scripts/audit_vanilla.py` — 3105 objects, 49 BE types, 232 source markers.
  Audit provenance ссылается на игровой коммит `54243b963`.

Build env: `CARGO_TARGET_DIR=D:\CodexBuilds\VectaMC`, `CARGO_INCREMENTAL=0`,
`TMP`/`TEMP=D:\CodexBuilds\tmp`; toolchain, rustflags, profiles и dependencies
не менялись. Логи: `D:\CodexBuilds\logs\codex-rng-*`.

## Коммит

`54243b963` — общий owner и persistence RNG, последовательный dispatch,
стабильный порядок WorldMap и regressions. Чужой `planning/night-01/report.md`
не изменён и не включён в коммиты; его исходный SHA256 сохранён.

## Границы

Не проверялись настоящий клиент, Linux, abrupt process termination и полная
Java loot-table golden extraction. TOML использует существующий write path
SavedDataManager; гарантии crash-atomic сохранения этим изменением не добавлены.
Остальные loot functions/contexts и специальные dispenser consumers остаются
в очереди, как перечислено в [ревью](W04-019-034-codex.md).
