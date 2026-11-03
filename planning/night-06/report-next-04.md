# Отчёт сеанса 4: G022 — живой `/data`

> Последующее ревью Codex от 2026-11-03 исправило ошибки NBT paths/string sources
> и сняло ложную диагностику B-06-08. Актуальные выводы и проверки:
> [night-06-g022-codex.md](../reviews/night-06-g022-codex.md).
> Следующая очередь: [WORKBUDDY_CONTINUE_06_05.md](../../WORKBUDDY_CONTINUE_06_05.md).
> Текст ниже сохранён как исходный отчёт исполнителя.

Дата: 2026-11-02. Ветка `codex/night-vanilla-03`, вход `6e6726fc0`.
Target `0.16.1+mc26.3`, local source 26.3 (5037 Java files).

## Что сделано

### 1. Foundation путей NBT (`vecta-utils`)

`NbtPathArgument.NbtPath` в Java умеет не только `set`: `/data` требует
`remove`, `insert` и `getOrCreate`-мутацию. В Vecta был только `get` /
`count_matching` / `set`.

Добавлено в `vecta-utils/src/nbt/path.rs`:

| Метод | Java-аналог | Поведение |
| --- | --- | --- |
| `NbtPath::remove` | `NbtPath#remove` + `Node#removeTag` | удаляет выбранные теги; неразрешённые родители дают 0, а не ошибку |
| `NbtPath::insert` | `NbtPath#insert` | `getOrCreate(target, ListTag::new)`, отрицательный индекс считается от конца (`-1` = append), `IndexOutOfBounds` → `InvalidIndex` |
| `NbtPath::merge` | `modify … merge` | `getOrCreate(target, CompoundTag::new)` + `CompoundTag.merge` через `merge_nbt_compounds` |
| `is_too_deep` | `NbtPathArgument.NbtPath.isTooDeep` | теперь публичный, лимит 512 |

`NbtPathMutationError` получил `ExpectedList` / `ExpectedObject` / `InvalidIndex`,
чтобы команда отдавала ванильные `commands.data.modify.*` сообщения.
Внутренние помощники: `visit_selected_mut` (обход выбранных тегов на месте),
`modify_targets`/`modify_at` (getOrCreate-обход), `insert_collection_element`,
`remove_collection_element`, `visit_collection_elements`.

### 2. Аргументы SNBT

В `command/execution/nbt.rs` и `argument.rs` добавлены `compound_tag()` и
`nbt_tag()` — аналоги `CompoundTagArgument` и `NbtTagArgument`, протокольные
типы `ArgumentType::Nbt` (21) и `ArgumentType::NbtTag` (22), значения
`CompoundTagValue`/`NbtTagValue` и доступ через `VectaCommandContext`.

### 3. `/data` как builtin

Новый модуль `vecta-core/src/command/builtins/data/` (`mod.rs` + `accessor.rs`),
зарегистрирован в `builtins/mod.rs`.

Дерево повторяет `DataCommands.register` с `ArgProvider.wrap`: провайдер — это
литерал (`block` / `storage`), а имя аргумента — `target` / `source`:

```
data merge   <block|storage> <target> <nbt>
data get     <block|storage> <target> [path [scale]]
data remove  <block|storage> <target> <path>
data modify  <block|storage> <target> <targetPath> insert <index> …
                                                  prepend|append|set|merge …
             … from   <block|storage> <source> [sourcePath]
             … string <block|storage> <source> [sourcePath [start [end]]]
             … value  <tag>
```

Реализованные манипуляторы: `insert`, `prepend`, `append`, `set`, `merge`.
Источники: `from`, `string` (с ванильными числовыми суффиксами `1b/2s/3L/1.5f/1.0d`
и UTF-16 подстрокой), `value`.

`BlockDataAccessor`-эквивалент (`accessor.rs`):

- `get_data` = `save_with_full_metadata` (`BlockEntity.saveWithFullMetadata`).
- `set_data` = `load_with_owned_components` с **живым**
  `BlockEntityLoadContext::for_source`, затем `set_changed()` и
  `send_block_updated(pos)` — как `BlockDataAccessor.setData`
  (`loadWithComponents`, `setChanged`, `sendBlockUpdated(pos, state, state, 3)`).
- Ошибка загрузки восстанавливает снимок и возвращает ошибку команды, а не
  оставляет половину состояния.
- `storage` — через `CommandStorage` домена (`source_command_storage` теперь
  принимает `&CommandSource`, чтобы этим пользовались и `/execute store`, и `/data`).

## Доказательства по карточкам

| Карточка | Проверка |
| --- | --- |
| W06-106 | `data_merge_resolves_a_lectern_book_with_the_live_command_context`: `/data merge block <lectern>` с книгой, страница которой ссылается на `storage`, резолвится в реальном контексте команды (`resolved()`, текст `"stored page"`) |
| W06-107 | `data_merge_reaches_an_open_barrel_menu`: меню открыто настоящим `BarrelBlock::use_without_item`; `has_container_open_for` доказывает, что меню владеет живым контейнером, а не копией; после `/data merge` и `broadcast_inventory_changes` зрителю уходит обновление слота |
| W06-108 | `data_merge_marks_the_chunk_unsaved`: чанк очищен, `/data merge` → `set_changed()` помечает его unsaved. Comparator-половина не проверена (см. partial) |
| W06-109 | `data_get_merge_and_remove_operate_on_live_block_entity_state` + `data_rejects_invalid_targets_and_unchanged_merges` + `data_modify_moves_items_between_block_entities`: неразрешённый `remove`, неизменный `merge`, невалидный индекс и цель без block entity дают **провал команды**, а содержимое остаётся прежним; мутации идут по локальной копии и пишутся только при успехе |
| W06-110 | Тот же lectern-тест: после смены storage и detached-reload страница остаётся `"stored page"`, повторного resolve нет |

Плюс юнит-тесты командного слоя: `tag_value` (ванильные result-типы, включая
clamp `Long` → `i32::MAX`), числовые суффиксы `string`, UTF-16 подстроки и
отказ на инвертированном диапазоне, форма графа `data modify … append from block <source>`.

## Partial / blocked

1. **`/data entity`** не реализован. В Vecta нет общего NBT-сериализатора
   сущности: `save_additional` есть только у конкретных типов, а
   `ChunkStorage` сохраняет типизированный `PersistentEntity`. Чтение можно
   собрать из `Entity::nbt_for_data_compare`, но `setData` требует обратной
   загрузки Pos/Motion/Rotation/UUID/Passengers и т.д. Это тот же гейт, что уже
   записан как TODO в `/execute store entity`. Половину провайдера не коммитил.
2. **`/data modify … compute`** не реализован: нужны loot-провайдеры
   `ContextFloatProvider`/`ContextIntProvider` и `LootContextSources`, которых в
   `vecta-registry::loot_table` нет.
3. **W06-108, comparator**: `World::get_redstone_signal` не умеет читать сигнал
   из `Container` (в `world/redstone.rs` нет ни одного обращения к контейнеру),
   поэтому компаратор рядом с бочкой всегда видит 0 и `check_tick_on_neighbor`
   не планирует пересчёт. Это предсуществующий пробел редстоуна, не следствие
   `/data`; проверена только половина с dirty flag. Не выдаю за полный W06-108.
4. **ITEM custom-data контекст (из ревью G021)**: `BlockEntity::load_item_custom_data`
   по-прежнему грузит через `load_additional` (detached). Чтобы дать кафедре
   разрешать книгу при `loadCustomOnly` в прикреплённом состоянии, нужен
   resolution context при `BlockItem::place`, а `World → Server` ссылку вводить
   запрещено. Вопрос архитектуры, не угадывал.
5. `/data get` печатает тег как канонический SNBT plain-текстом, а не
   `NbtUtils.toPrettyComponent` с подсветкой ключей. Это command UX, не игровое
   поведение; отмечено в коде `accessor::render_tag`.

## Проверки

| Команда | Результат |
| --- | --- |
| `cargo test -p vecta-utils --lib nbt::path -j 1` | 27 passed, exit 0 |
| `cargo test -p vecta-core --lib command::builtins::data -j 1` | 4 passed, exit 0 |
| `cargo test -p vecta-core --lib data_command -j 1` | 6 passed, exit 0 |
| `cargo test --locked --workspace -j 1` | **4273 passed / 0 failed / 18 ignored**, exit 0 |
| `cargo fmt --all --check` | exit 0 |
| `cargo clippy --locked --workspace --all-targets --all-features -j 1 -- -D warnings` | exit 0 (четыре круга правок: `explicit_counter_loop`, `semicolon_if_nothing_returned`, `unused_mut`, `absolute_paths`, `unused_imports`; production-логика не менялась, кроме `actual_index + offset` вместо инкремента) |

Build env: target `D:\CodexBuilds\VectaMC`, `CARGO_INCREMENTAL=0`,
TMP/TEMP `D:\CodexBuilds\tmp`, Cargo последовательно `-j 1`.
Logs: `D:\CodexBuilds\logs\w06-g022-*`.

## Следующий шаг

Следующий ID — **G023 / W06-111…115 (NBT рамок)**. Перед началом проверено
состояние (не переписывать готовое):

- `ItemFrameEntity`/`GlowItemFrameEntity` уже пишут и читают `Item`, `ItemRotation`,
  `Invisible`, `Fixed`, `ItemDropChance` (`entity/entities/objects/display_ui/item_frame.rs:635-680`).
- `item_frame/review_tests.rs` уже покрывает payload `Fixed`+`Invisible`,
  знаковый `ItemRotation` и legacy direction ids, сброс rotation/drop chance при
  отсутствующих полях, числовые NBT-конверсии, placement, overlap и support loss.
- Значит W06-111/112/114 — «подтвердить, не переписывать»; W06-113 (drop chance
  с gamerules и creative) и W06-115 (map linkage, нужен G082) остаются.
- «Command data» путь для рамок упирается в B-06-06: провайдера `/data entity`
  нет, пока нет общего entity NBT load.

Независимо открыты: `/data entity` (B-06-06), `compute` (B-06-07),
контейнерный redstone-сигнал для comparator (B-06-08), item custom-data context
кафедры (B-06-09).
