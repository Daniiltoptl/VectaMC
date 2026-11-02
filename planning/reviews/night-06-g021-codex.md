# Codex review: G021 placement data and persistence boundaries

Дата: 2026-11-02, ветка `codex/night-vanilla-03`.
Входящий диапазон `cea3e5115..0c6cefaf9`, игровой commit `69bcdde45`.
Target `0.16.1+mc26.3`, local reference 26.3 / 5037 Java files,
server SHA1 `33680f5f2ac32864d6d7cf5e56a705fdb3e05f4c`.

## Вердикт: G021 нельзя считать полностью закрытой

Пропущенный прежде `updateCustomBlockEntityTag` действительно подключён:
item payload теперь попадает в настоящую block entity до implicit components
и `setPlacedBy`. Тип payload проверяется. Корневые CustomName/lock и отдельный
обычный reload сохранены верно; B-06-04 по-прежнему снят.

После ответа владельца `onlyOpCanSetNbt` gate реализован на настоящем produced
output. W06-105 всё ещё подтверждена только decoder/factory boundary, без
region I/O/unload/restart; attached-lectern text context остаётся для G022.
Закрытие части сценариев не является доказательством полной матрицы всех BE.

## P1 — авторизация защищённого NBT пропущена, исправлена после extraction

Java `BlockItem.updateCustomBlockEntityTag` разрешает protected custom data
только при `player != null && player.canUseGameMasterBlocks()`. Последнее
требует creative/instabuild и gamemaster permission. Входящий код пропускал
payload любого совпавшего типа, независимо от игрока и режима.

Объяснение «нет системы прав» неверно: уже есть permission manager, OP-группа,
`Player::is_operator`, а entity item spawn использует этот operator contract.
На входе ревью `vecta-registry::BlockEntityType` и его extracted asset не имели
`only_op_can_set_nbt`. Нельзя просто подставить ручной список типов или false.

Подготовлен дополнительный output актуального SteelExtractor: версия и флаг
из реального `BlockEntityType.onlyOpCanSetNbt()`, без запуска MinecraftServer.
Обе первые попытки остановились до extraction: `UnixDomainSockets.connect0`
→ `Invalid argument: connect`, обёрнутый в `Unable to establish loopback connection`.
Вторая попытка — только диагностика `--stacktrace`; дальнейшие варианты не перебирались.

Был запрошен working output или результат запуска из обычного CMD.
Владелец прислал новый факт: его обычный Gradle работал на Java 17 и требовал 25.
Проверка подтвердила JAVA_HOME=17 при PATH java=25. Явный process-local выбор
установленного JDK 25 тоже упал с loopback error в tool environment. Владелец
получил точную CMD-команду с JAVA_HOME=25 и нужным task.
Это не установка нового toolchain и не изменение системных настроек.
В обычном CMD владельца task завершился `BUILD SUCCESSFUL in 29s`. Produced JSON
26.3 содержит flags всех 49 типов и скопирован без изменения байтов; SHA-256
`bfe9f72f37d43fc9297ccf39e5f2bf0cecdf053679e48c130e74e5e01abfc88b`.
Generator требует flag без default и проверяет версию. Typed field передаётся
в placement gate: protected payload разрешён только creative + существующий
published OP contract Vecta, которым уже пользуется entity-item spawn.
Модель permissions/config/domains не заменялась, новых nodes/OP levels не вводилось.
Сравнение произведённого JSON с прежним asset подтвердило совпадение всех 49
ID/имён/valid_blocks; добавлены только version и реальные authorization flags.

Тот же predicate теперь используется при начальном `LecternBlock.HAS_BOOK`:
прежняя creative-only проверка могла выставить его для неавторизованного payload.
Два новых tests проверяют реальный server/player/OP cache и `BlockItem::use_on`
для четырёх combinations operator/non-operator × creative/survival, включая Book
и HAS_BOOK; второй проверяет реальный playerless directional placement. Обычный
barrel payload и mismatch type по-прежнему проходят свой regression.
Источник/команда/предупреждения extraction — в `scripts/steel-extractor/block-entity-types.md`.

## P2 — snapshot custom NBT сохранялся дважды, исправлено

Java `TypedEntityData.loadInto(BlockEntity)` делает один `saveCustomOnly` и
копию полученного compound. Новый Rust helper вместо копии повторно вызывал
`save_custom_only`, что удваивало serialization и не гарантировало один снимок
для расширяемого serializer. Теперь save вызывается один раз, затем compound
копируется для merge/comparison. Это необходимая snapshot copy, не clone ради
borrow checker.

Усилен тест реальной бочки: payload сохраняет опущенные name/lock, explicit
CUSTOM_DATA остаётся в stored map, повтор того же payload возвращает false.

## P2 — callbacks загрузки выполнялись под inventory lock, исправлено

`BlockPlaceContext.with_item` берёт player inventory lock. Новый код внутри
closure вызывал custom-data load, component callbacks и `set_changed`, который
заходит в world/comparator callbacks. Это нарушало краткую область inventory lock
и создавало риск повторного входа для расширяемых behaviors.

Теперь caller получает item snapshot и отпускает inventory lock до загрузки
и применения компонентов. Порядок custom data → components → setPlacedBy сохранён.
Реального зависшего vanilla-world сценария здесь не воспроизводили; исправлена
подтверждённая область удержания lock, не заявляется пойманный runtime deadlock.

## Проверка типа и границы доказательств

Placement regression расширен двумя реальными `BlockItem::use_on` сценариями:
BARREL payload применяется к бочке, CHEST payload игнорируется, implicit имя
применяется в обоих случаях. Это meaningful rejection path, не тест поля реестра.

Входящие tests подтверждают разделение custom NBT и stored components, item pick
helper, component application, NBT decoder/factory. «Unknown component» в тесте —
зарегистрированный `minecraft:custom_data`, который не потребляется бочкой;
сохранение произвольного неизвестного registry ID этим не доказано.

`picking_a_barrel_moves_implicit_components_onto_the_item` вызывает настоящий
pick helper, но обратная часть использует `apply_components_from_item` напрямую.
Отдельный placement test вызывает `use_on`; это не единый сетевой pick→place прогон.
Storage regression вручную строит `PersistentBlockEntity` и вызывает loader:
region-file write/read и world chunk unload/load не выполнялись.

Дополнительная открытая часть для G022: helper custom-data load вызывает обычный
`load_additional`. У кафедры это detached context. Java разрешает книгу при
`loadCustomOnly` уже прикреплённой кафедры; существующий Vecta
`BlockEntityLoadContext` должен быть учтён при следующем сквозном срезе, без
новой World→Server ссылки и без притворного server context у detached fixtures.

G022 действительно не реализована как builtin `/data`; инфраструктура частично
есть. Размер задачи — не внешний blocker. Это следующий самостоятельный срез,
который можно делать независимо от сбоя Java extractor.

## Проверки первого среза ревью (до authorization metadata)

| Команда | Результат |
| --- | --- |
| `cargo test --locked -p vecta-core --lib behavior::items::block_item -j 1` | 1 passed, exit 0 |
| `cargo test --locked -p vecta-core --lib item_block_entity_data -j 1` | 2 passed, exit 0 |
| `cargo test --locked --workspace -j 1` | **4252 passed / 0 failed / 18 ignored**, exit 0; сборка 3m56s |
| `cargo clippy --locked --workspace --all-targets --all-features -j 1 -- -D warnings` | exit 0, 1m34s |
| `cargo fmt --all --check` | exit 0 |
| `python -m unittest discover -s scripts -p 'test_*.py'` | 4 tests OK, exit 0 |
| `python scripts/audit_vanilla.py` | 3105 objects, 49 BE types, 232 source markers, exit 0 |
| `git apply --reverse --check .../block-entity-types.patch` в extractor | exit 0; проверен только patch, не extraction |

## Проверки authorization slice после produced output

| Команда | Результат |
| --- | --- |
| `cargo test --locked -p vecta-core --lib block_entity_item_data -j 1` | 2 passed, exit 0; OP/creative matrix и playerless placement |
| `cargo test --locked -p vecta-core --lib behavior::items::block_item -j 1` | 1 passed, exit 0; ordinary/mismatched type |
| `cargo test --locked --workspace -j 1` | **4254 passed / 0 failed / 18 ignored**, exit 0; сборка 8m19s |
| `cargo clippy --locked --workspace --all-targets --all-features -j 1 -- -D warnings` | exit 0, 3m43s |
| `cargo fmt --all --check` | exit 0 |
| `python -m unittest discover -s scripts -p 'test_*.py'` | 4 tests OK, exit 0 |
| `python scripts/audit_vanilla.py` | 3105 objects, 49 BE types, 232 markers; provenance на commit gate |

Первый compile новых gate tests выявил test-only missing `BlockStateExt` import;
исправлено. После этого узкие проверки выше прошли. Предыдущие 4252 tests
не выдаются за проверку нового gate.

Build env: target `D:\CodexBuilds\VectaMC`, TMP/TEMP `D:\CodexBuilds\tmp`,
`CARGO_INCREMENTAL=0`, Cargo последовательно `-j 1`.
Logs `D:\CodexBuilds\logs\codex-review06c-*` и `codex-be-types-extraction*`.
Входящие 4252 passed — результат исполнителя, до свежих проверок не присваивался Codex.
Полный прогон Codex также дал 4252 passed: существующие tests усилены дополнительными
сценариями без увеличения счётчика. Clippy прошёл с первого прогона ревью;
сохранилось прежнее Cargo manifest warning про optional mimalloc при all-features.
Клиент, Linux, Mojang comparison, два сетевых игрока и живой restart/unload не выполнялись.
EULA не принималась. Foreign dirty START_06 и report Night 01 сохранены.

Первый срез ревью: **`8b581a658`**, три Rust paths. Audit provenance тогда указывал на него.
Документы, подготовленный extractor patch и запрос — отдельный docs commit
той же ветки; продолжать actual tip, не откатываться к историческому SHA.

После успешного extraction добавлен **`673c7a558`**: typed authorization metadata,
placement/lectern gates и два server-backed regressions, девять paths.
Актуальный audit provenance теперь указывает на `673c7a558`.
Оба полных прогона и финальный Clippy завершились exit 0; финальные числа —
4254 passed / 0 failed / 18 ignored. B-06-05 снят. Остались region I/O/unload/restart
acceptance и attached lectern item text context, не новая причина менять disk format.
