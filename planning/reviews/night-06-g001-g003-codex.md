# Ревью G001–G003 и продолжение пакета 06

Дата: 2026-11-02. Ветка: `codex/night-vanilla-03`.
Диапазон исполнителя: `3a79ec57f..78da455e5` (пять commits).
Target: `Cargo.toml` — `0.16.1+mc26.3`; local source manifest — 26.3,
5037 Java files, server SHA1 `33680f5f2ac32864d6d7cf5e56a705fdb3e05f4c`.

## Вывод

Подтверждённых ошибок в новых игровых регистрациях при этом ревью не найдено.
Пять новых item registrations используют существующие projectile consumers:
experience bottle, три яйца, splash potion. Вместе со snowball подключено
6 из 13 projectile items из `DispenseItemBehavior.bootStrap`. Это не шесть
полностью новых entity implementations и не оценка процента готовности vanilla.

G001–G003 реализуют серверные пути; имеющиеся тесты доказывают перечисленные
ниже сценарии. Пометка всех 15 карточек completed не означает, что проверено
каждое сочетание из общей матрицы или реальный клиентский lifecycle.

Для контекста прочитаны предыдущие upstream, W04-019-034 и server-random-sequences
reviews. Это не повторное построчное ревью всех старых commits. Согласованные
shared RNG, последовательные world ticks и другие прежние foundations сохранены.

## Находки и изменения

### P2 — прежние save/load тесты обходили общий persistence loader

В отчёте совместимость flight/save/load обосновывалась преимущественно
`save_additional`/`load_additional`. Такой тест не обнаружит потерю UUID,
position, velocity, rotation или неверную регистрацию factory.

Добавлен параметризованный сценарий для всех пяти новых items:
настоящий dispenser pulse → `entity_tree_to_persistent` → wincode binary record
→ `persistent_to_entity_tree_at_level` → настоящая keyed entity factory.
Проверяются UUID/position/velocity/rotation, отсутствие owner, item identity/count,
custom name, egg variant defaults и potion contents/duration scale.
Тест использует существующие `ChunkStorage` API; shared интерфейсы и формат
сохранения не менялись. Это проверка record boundary, не disk restart/unload
живого сервера и не продолжение полёта через реальный tick loop после restart.

### P2 — тесты зелий пропускали неверные численные правила и attribution

Прежние assertions `near > far > 0` и `health_after < health_before` оставались
зелёными при многих неправильных коэффициентах, округлениях или damage sources.

Теперь timed test задаёт duration scale 1.75. Java AABB formula и извлечённые
размеры potion `.25` / adult pig `.9f`, tick-zero margin 0, SWIFTNESS 3600 дают
точные длительности **6300** и **2481** ticks. Дополнительная цель за пределами
радиуса не получает эффект. Это source-derived численный oracle, не новый запуск
Mojang server/Java simulation.

Instant test проверяет ровно **6** damage, тип `INDIRECT_MAGIC`, direct source —
конкретная potion allocation, causing owner отсутствует. Эффект не сохраняется
как timed instance. Удар этого fixture идёт по блоку рядом с целью; отдельный
новый тест entity-hit branch этим изменением не добавлен.

### P2 — неоднозначное ТЗ допускало несовместимую реализацию

- G010 теперь явно разделяет ручное зажигание/огниво и fire charge из dispenser.
  В Java последний зарегистрирован как projectile и требует SmallFireball/G047;
  прямое поджигание target вместо полёта нарушило бы эталон.
- W06-010/015 теперь требуют нормального removal/collision lifecycle, а не
  идемпотентности произвольного второго `onHit`. Java не содержит такого guard.
  Discard сам по себе не является доказательством теста всех повторных callbacks.
- G004 зависит от G005 (`AreaEffectCloud`) и не начата. Само отсутствие Rust
  реализации — зависимость разработки, а не внешний blocker для владельца.
  Реальный architecture/data blocker требует отдельной проверки и узкого вопроса.

### P3 — устаревшее описание registrations

Комментарий над `register_projectile_dispense_behaviors` перечислял только
snowball/bottle как готовые consumers. Заменён правилом: отсутствующий consumer
не регистрируется против заглушки, которая подавила бы default item drop.

## Сверенные контракты

- `ExperienceBottleItem.createDispenseConfig`, `ThrowablePotionItem.createDispenseConfig`:
  power `1.1 * 1.25`, uncertainty `6 * .5`, default position offset.
- `EggItem` не переопределяет dispenser config: power 1.1; ручной бросок 1.5.
- `DispenseItemBehavior.bootStrap`, `ProjectileDispenseBehavior.execute`,
  `ProjectileItem`, `Projectile.spawnProjectile`: registration, launch,
  single-item copy, rejection semantics, events и spawn hooks.
- `ThrownEgg`, `ThrownExperienceBottle`, `AbstractThrownPotion`, `ThrownSplashPotion`,
  `HealOrHarmMobEffect`: попадание, эффект/опыт, removal, distance/scale и attribution.
- `ChunkStorage` save/restore, `PersistentEntity`, concrete `from_saved` constructors
  и существующий factory codegen; generated Rust вручную не менялся.

Generated/extracted data, protocol, dependencies, toolchain, profiles и production
gameplay logic при исправлении ревью не менялись. Extractor не запускался.
Регистрация vanilla .9f/.25 и potion duration взята из текущих extracted assets;
ручных registry values в production не добавлено.

## Свежие проверки Codex

| Команда | Результат |
| --- | --- |
| `cargo test --locked -p vecta-core --lib dispenser_block -j 1` | 27 passed, exit 0 |
| `cargo test --locked -p vecta-core --lib splash_potion -j 1` | 5 passed, exit 0 |
| `cargo test --locked --workspace -j 1` | **4198 passed / 0 failed / 18 ignored**, exit 0; сборка 8m04s |
| `cargo test --locked -p vecta-core --lib dispensed_consumables_restore -j 1` | 1 passed, exit 0, после test-only Clippy правки |
| `cargo clippy --locked --workspace --all-targets --all-features -j 1 -- -D warnings` | exit 0, 1m15s после test-only правки |
| `cargo fmt --all --check` | exit 0 |
| `python -m unittest discover -s scripts -p 'test_*.py'` | 4 tests OK, exit 0 |
| `python scripts/audit_vanilla.py` | 3105 objects, 49 BE types, 233 source markers, exit 0 |

Build environment: target `D:\CodexBuilds\VectaMC`, TMP/TEMP
`D:\CodexBuilds\tmp`, `CARGO_INCREMENTAL=0`, Cargo последовательно с `-j 1`.
Свежие logs: `D:\CodexBuilds\logs\codex-review06-*.log`.
Во время написания нового теста исправлены test-only imports и сравнение ItemRef
с LazyLock; промежуточные compile errors не считаются успешными проверками.
Первый свежий Clippy выявил `borrow_as_ptr` в новом тесте. Сравнение заменено
обычным `ItemRef` equality, без suppression. Полный workspace с 4198 passed
выполнен до этой test-only правки; после неё затронутый persistent round-trip
повторно прошёл. Результат полного workspace Clippy указан в таблице.
Production gameplay не менялся.
Сохранилось прежнее Cargo manifest warning про optional `mimalloc` при all-features;
оно не подавлялось и не является ошибкой Clippy.

## Продолжение и границы

Код ревью: **`1fc3e504a`** (три Rust paths, только tests и комментарий).
Документация, audit provenance и новый запрос следуют отдельным docs commit
в той же ветке; продолжать её actual tip, не откатываться к этому SHA.

Следующий consumer — **G006 / W06-026…030**. Дальше G007, G009…013 и route.
Новый запрос: `WORKBUDDY_CONTINUE_06_02.md`; полный каталог — `NIGHT_TASK_06.md`.
Checkpoint обновляется по фактическому результату проверок.

Клиент 26.3, Linux, Mojang comparison, два сетевых игрока и живой restart/unload
не выполнялись. EULA не принималась. Старый workspace log исполнителя с 4197 tests
не выдаётся за свежий прогон Codex. Foreign dirty `planning/night-01/report.md`
и `WORKBUDDY_START_06.md` сохранены отдельно и не входят в commits ревью.
