# Ревью сеанса 2: G006/G007/G009–G013

Дата: 2026-11-02. Ветка `codex/night-vanilla-03`.
Входящий диапазон `cfc95d0a3..4afc44fd7`, target `0.16.1+mc26.3`.
Local manifest соответствует 26.3, 5037 Java, server SHA1
`33680f5f2ac32864d6d7cf5e56a705fdb3e05f4c`.

## P1 — неверный план переделки persistence (B-06-04), снят

Входящий отчёт утверждал, что Java не сохраняет имя/lock в корневых NBT-полях,
и предлагал заменить это применением item components на каждом reload.
Local `BaseContainerBlockEntity` явно читает/пишет эти поля в
`loadAdditional`/`saveAdditional`. `BlockEntity.loadWithComponents` читает
специализированное состояние и stored components без `applyComponents`.

`collectComponents` и `removeComponentsFromTag` относятся к item/pick-пути;
они не означают, что chunk save обязан собирать все implicit fields в карту.
Применение отсутствующих item components на reload могло бы очистить имя/lock
и inventory. Production persistence не менялся; добавлен regression на
`save_without_metadata` → binary NBT → `load_with_components`, сохраняющий
identity и пять diamonds в седьмом слоте бочки. Это не живой restart/unload.
Blocker и checkpoint исправлены; G021 начинается с проверки фактов, без нового hook.

## P1 — лава могла заменить waterloggable slab, исправлено

`BucketItem.empty_fluid_contents` проверял `can_place_liquid_with_player` с WATER
независимо от содержимого ведра. Для пустой oak slab проверка проходила, после
чего lava consumer заменял плиту лавой. В dispenser это новый путь; общий helper
также используется рукой. Теперь передаётся фактический fluid ID из typed
fluid block state. Regression сохраняет slab и выбрасывает неиспользованное
lava bucket. Java reference: `BucketItem.emptyContents`, `LiquidBlockContainer`.

## P1 — неудачная стрижка выбрасывала инструмент, исправлено

Java `ShearsDispenseItemBehavior.execute` всегда возвращает переданный stack,
повреждая его только при success. Vecta ошибочно вызывала default drop при
пустой/неподходящей цели. Более того, два новых теста закрепляли этот баг;
`get_damage_value() == 0` на пустом stack скрывал потерю инструмента.

Удалён fallback drop. Тесты проверяют настоящий stack в слоте и отсутствие
item entities, отдельно для пустой цели и уже остриженной овцы.

## P2 — отсутствовал vanilla leash-first path, исправлено

Java выбирает non-spectator entities и сначала вызывает
`shearOffAllLeashConnections(null)`, даже для сущности без `Shearable`.
Vecta заранее фильтровала выборку до готовых shearables, пропуская поводки.
Существующая leash foundation уже реализована; новый fake hook не нужен.

Теперь выборка исключает spectators, затем по каждой entity идёт leash-first
и только потом alive/ready shearable. Два regressions проверяют приоритет
поводка перед шерстью у овцы и перерезание поводка несостригаемой свиньи.
Одна успешная операция повреждает инструмент один раз.

## P2 — неверная категория звука стрижки, исправлено

Новая capability не передавала `SoundSource`; обе реализации жёстко использовали
Players, включая dispenser. Java `Shearable.shear` принимает категорию от caller.
Параметр добавлен в существующую capability/реальные sheep/mooshroom consumers.
Рука передаёт Players, dispenser — Blocks. Packet test проверяет категорию звука
при реальной стрижке овцы из раздатчика.

## P2 — ведра пропускали final sound/animation, исправлено

Rust bucket behaviors переносили Java `execute`, но забыли финальную фазу
`DefaultDispenseItemBehavior.dispense`. Успех размещения/подбора не отправлял
1000/2000. Rejected filled bucket в Java вызывает отдельный default `dispense`
внутри execute, затем внешнюю фазу: две пары 1000/2000. У пустого ведра отказ
идёт через `super.execute`, поэтому остаётся одна пара.

Выделены реальные `execute` (spawn без feedback) и общая final feedback фаза.
Filled/empty consumers теперь сохраняют этот порядок; bottle failure использует
общий execute. Проверены success воды/лавы/снега, rejected snow, source/flowing
pickup и две пары при eject заполненной бутылки из полного контейнера.
Empty bucket, как Java, возвращает новый stack типа pickup без дополнительных
компонентов результата `pickupBlock`.

## Остальной проверенный объём и ограничения

Сверены Java `DispenseItemBehavior`, `DefaultDispenseItemBehavior`,
`ShearsDispenseItemBehavior`, `FlintAndSteelDispenseItemBehavior`, `BucketItem`,
`SolidBucketItem`, `BoneMealItem`, `Shearable`, `BeehiveBlock`,
`BlockEntity` и `BaseContainerBlockEntity`; просмотрены новые registrations,
capabilities, consumers и тесты. Огниво действительно изнашивается об обычную
неподходящую цель: success в Java остаётся true. Это не исправлялось как баг.

Входящий объём — 32 файла, большая часть новых строк относится к tests/docs.
4240 tests у исполнителя — его заявленный результат, не свежий прогон Codex.

Остаются частичными:
- body equipment для отсутствующих wolf/horse;
- SmallFireball/fire charge, SulfurCube ignition;
- TreeGrower/saplings и biome/tag/coral ветки underwater bonemeal;
- позиционный WATER_EVAPORATES и полные fizz sound/large-smoke эффекты;
- hive shear, bee release и связанная приоритетная ветка перед entity shear.

Для honeycomb Java использует **HARVEST_BEEHIVE**, а не `blocks/beehive`.
Typed table и interaction-loot helper уже существуют; проверить полноту context
при реализации. Отсутствующий consumer — зависимость разработки, не внешний
blocker без конкретного отсутствующего решения/data.

Ни один из этих пунктов не закрыт одним dispenser registration. Generated Rust,
extracted JSON, toolchain, dependencies, build profiles и архитектура сервера
при ревью не менялись. Нового unsafe/Any/TypeId/production unwrap не добавлено.

## Свежие проверки

| Команда | Результат |
| --- | --- |
| `cargo test --locked -p vecta-core --lib dispenser -j 1` | 83 passed, exit 0 |
| `cargo test --locked -p vecta-core --lib normal_nbt_reload -j 1` | 1 passed, exit 0 |
| `cargo test --locked --workspace -j 1` | **4245 passed / 0 failed / 18 ignored**, exit 0; сборка 3m21s |
| `cargo clippy --locked --workspace --all-targets --all-features -j 1 -- -D warnings` | exit 0, 1m17s |
| `cargo fmt --all --check` | exit 0 |
| `python -m unittest discover -s scripts -p 'test_*.py'` | 4 tests OK, exit 0 |
| `python scripts/audit_vanilla.py` | 3105 objects, 49 BE types, 232 source markers, exit 0 |

Build env: target `D:\CodexBuilds\VectaMC`, TMP/TEMP `D:\CodexBuilds\tmp`,
`CARGO_INCREMENTAL=0`, Cargo последовательно с `-j 1`.
Logs: `D:\CodexBuilds\logs\codex-review06b-*.log`.
Сохранилось прежнее Cargo manifest warning про optional mimalloc при all-features;
оно не подавлялось. Свежий Clippy прошёл с первого прогона ревью.
Клиент 26.3, Linux, Mojang comparison, два сетевых игрока и живой restart/unload
не выполнялись. EULA не принималась.

Следующий ID: G021/W06-101…105 по исправленному checkpoint.
Новый запрос — `WORKBUDDY_CONTINUE_06_03.md`. Foreign dirty report Night 01
и START_06 сохранены, в commits ревью не входят.

Код исправлений и regressions: **`df47b146e`**, 13 Rust paths.
Audit provenance ссылается на этот commit. Документы и запрос — отдельный docs
commit той же ветки; продолжать actual tip, не откатываться к этому SHA.
