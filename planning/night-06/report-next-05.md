# Отчёт сеанса 5: живая загрузка, region I/O и `/data entity`

Дата: 2026-11-03. Ветка `codex/night-vanilla-03`, вход `77bd4142c`.
Target `0.16.1+mc26.3`, local source 26.3.

## 1. Контекст загрузки книг при placement (B-06-09 снят)

Ревью G021/G022 верно указало: `Player::server()` уже есть, новая ссылка
World→Server не нужна, ограниченный resolver вместо storage/score/selectors
был бы неверным. Проверил Java `LecternBlockEntity.loadAdditional` → `resolveBook`
(`level instanceof ServerLevel`) — книга разрешается у **прикреплённой** кафедры,
в центре блока, без source entity.

Сделано:

- `BlockEntityLoadContext::for_server_world(server, world)` — конструктор
  placement-фазы; `for_source` теперь делегирует в него, дублирования нет.
- `BlockEntity::load_item_custom_data(payload, context)` использует
  `load_additional_with_context`; последовательность snapshot → merge → один
  load сохранена (ревью P2 не откатывал).
- `BlockItem::place_with` строит живой контекст из `player.server.upgrade()` и
  мира размещения. Playerless placement и «сервер уже ушёл» остаются
  `detached` — это и есть ванильное «нет ServerLevel ⇒ не разрешаем».
  Прямого `player.server()` (с `expect`) не вызываю, чтобы не паниковать в
  fixture и в окне завершения сервера; используется тот же `Weak::upgrade`
  паттерн, что в `lectern_block.rs`/`books.rs`.

Порядок custom data → components → `setPlacedBy` и snapshot предмета вне
inventory lock сохранены.

Тест `authorized_placement_resolves_a_written_book_through_the_live_context`
(server-backed): OP+creative игрок ставит кафедру с книгой, страница которой
читает `storage`; после `BlockItem::use_on` страница `resolved()` и равна
`"from placement"`. Затем storage меняется и делается detached-reload —
страница не переразрешается.

## 2. Настоящий region I/O и две ошибки сохранения (G021/W06-105)

Раньше «storage» проверялся только decoder-ом на вручную собранном
`PersistentBlockEntity`. Добавлен тест
`a_barrel_survives_a_real_region_file_round_trip`, который пишет **настоящий
`.mca`** через `RegionManager` и читает его обратно.

Тест сразу нашёл две реальные ошибки сохранения block entity:

1. **Запись**: `prepare_chunk_save` писал `entity.save_additional(&mut nbt)`,
   то есть терял карту компонентов. Vanilla `LevelChunk` сохраняет
   `saveWithFullMetadata`, чей payload — `saveWithoutMetadata`
   (`saveAdditional` + `components`). Теперь пишется
   `entity.save_without_metadata()`.
2. **Чтение**: `create_and_load_or_unimplemented` вызывал `load_additional`,
   а vanilla `LevelChunk` грузит `BlockEntity.loadWithComponents`. Теперь
   вызывается `load_with_components(nbt, &detached)`; detached сохраняет
   прежнюю семантику «диск не разрешает серверный текст» (тест кафедры
   `initial_factory_load_does_not_resolve_books_before_level_binding` проходит).

Проверено на реальном файле: имя, lock, 5 алмазов в слоте 7, рабочий контейнер
после восстановления и явный `minecraft:custom_data` компонент.
Тест также фиксирует требование `RegionManager`: `load_chunk` работает только
после `acquire_chunk`.

Это не живой restart сервера: тест использует настоящий region-файл и реальный
storage-API, но не поднимает второй сервер и не делает unload/load через
ticket-машину.

## 3. Общий entity NBT load и `/data entity` (B-06-06 снят для провайдера)

`Entity::apply_spawn_data` уже читал Pos/Motion/Rotation/Air/Fire/tags/флаги и
вызывал subtype loader — с нуля ничего не писал. Но его семантика — merge:
отсутствующий ключ сохраняет текущее значение. Vanilla `Entity.load` —
reset load: отсутствующий ключ возвращается к дефолту, а `EntityDataAccessor`
затем восстанавливает UUID и вызывает `postDataManipulated`.

Сделано:

- Общий `Entity::apply_entity_nbt(nbt, mode)` с `EntityNbtLoad::{Merge, Reset}`;
  различие только в трактовке отсутствующего ключа (`load_value_or`) и в
  применении позиции. Один список полей, без копипасты.
- `Entity::load_command_data` = reset-загрузка; позиция коммитится через
  `try_set_position` (world entity tracking), а не `set_position_local`.
  Merge-путь (spawn data) остался локальным, как раньше.
- `/data entity` через провайдер `entity` в `data/accessor.rs`:
  get = `nbt_for_data_compare` (vanilla `NbtPredicate.getEntityTagToCompare`),
  set = reset load, игроки отвергаются `commands.data.entity.invalid`.
  UUID не читается загрузчиком вообще, поэтому payload не может сменить
  identity — это тот же наблюдаемый результат, что «load + restore UUID».
- `/execute store entity` подключён к тому же `load_command_data`; старый TODO
  снят.

Тесты (server-backed, реальные Server/World/Player/Sheep):

- get/merge/modify/remove на живой овце (имя, NoGravity, Motion, удаление);
- reset-семантика через `remove` (отсутствующий `NoGravity`/`CustomName`/`Tags`
  сбрасываются, а не сохраняются);
- payload `UUID:[I;0,0,0,0]` не меняет `uuid()`;
- `data merge entity @s` отвергается, имя игрока не меняется;
- `execute store result entity … run data get entity @s Air` пишет
  `PortalCooldown` через общий загрузчик.

## 4. Partial / не сделано

- **`modify … compute`** не реализован. Это не «нет данных extractor-а»:
  `ContextFloatProvider`/`ContextIntProvider` — inline codecs с
  диспетчеризацией по типу (`ContextFloatProviderTypes`), плюс
  `LootContextSources`. В Java 26.3 это ~30 реализаций
  (`Absolute/Average/Ceiling/ConditionalValue/ConstantValue/Cosine/Difference/
  EnchantmentLevel/EnvironmentAttribute/Floor/FromInt/Length/Maximum/Minimum/
  Modulus/Negate/Power/Product/Quotient/Round/Sine/SquareRoot/StorageValue/Sum/
  Truncate/Uniform/WeightedList` и int-аналоги). Это отдельный крупный срез:
  типизированные провайдеры + `LootContextSources` + реальный consumer, как и
  требует запрос. Половину регистрации узла не коммитил.
- **`postDataManipulated`**: в vanilla переопределён только `Villager`; в Vecta
  нет villagers, поэтому вызов не добавлен. При появлении — отдельный hook.
- **Recipe-book award при взятии результата печи** (G027) не сделан: нужен
  `ServerRecipeBook` (направление 5 очереди). XP-часть (`pop_furnace_experience`)
  уже есть и используется при разрушении блока.
- **W06-113/115** (drop chance рамок, map linkage) не трогал.
- Настоящий клиент, Linux, Mojang differential, два сетевых клиента и живой
  restart/unload не запускались. EULA не принималась.

## 5. Проверки

| Команда | Результат |
| --- | --- |
| `cargo test -p vecta-core --lib block_entity_item_data -j 1` | 3 passed |
| `cargo test -p vecta-core --lib a_barrel_survives_a_real_region -j 1` | 1 passed |
| `cargo test -p vecta-core --lib data_entity -j 1` | 3 passed |
| `cargo test -p vecta-core --lib execute_store_entity -j 1` | 1 passed |
| `cargo test -p vecta-core --lib chunk_saver -j 1` | 55 passed |
| `cargo test -p vecta-core --lib block_entity -j 1` | 123 passed |
| `cargo test -p vecta-core --lib entity -j 1` | 1154 passed |
| `cargo fmt --all --check` | exit 0 |
| `cargo test --locked --workspace -j 1` | **4285 passed / 0 failed / 18 ignored**, exit 0 |
| `cargo clippy --locked --workspace --all-targets --all-features -j 1 -- -D warnings` | exit 0 (пять кругов: unused imports, `private_interfaces`, `needless_borrow`, `too_many_lines`, `absolute_paths`; логика не менялась) |
| `python -m unittest discover -s scripts -p 'test_*.py'` | 4 tests OK, exit 0 |
| `python scripts/audit_vanilla.py` | 3105 objects, 49 BE, 231 markers; TODO `execute store entity` удалён, provenance → `7347646c7` |

Логи: `D:\CodexBuilds\logs\w06-g022-entity-*`.

## 6. Следующее действие

1. `modify … compute`: типизированные `ContextFloatProvider`/`ContextIntProvider`
   + `LootContextSources` + command consumer (один срез).
2. Направление 3 очереди: NBT раздатчика/durability/equip hooks — проверить
   `hurt_and_break`/`process_durability_change` и equip capability на реальных
   предметах, не переписывая готовое.
3. Направление 4: печь — result-slot XP hook и `recipes_used` decrement
   (после появления `ServerRecipeBook` — award).
