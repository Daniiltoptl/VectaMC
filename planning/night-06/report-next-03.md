# Night 06 — отчёт сеанса 3 (G021, disk/item boundaries)

Дата: 2026-11-02. Ветка `codex/night-vanilla-03`.
Вход: HEAD `cea3e5115`, workspace **4245 passed / 0 failed / 18 ignored**.
Выход: см. «Проверки».

## Важно: B-06-04 снят

Прошлый отчёт предлагал убрать корневые `CustomName`/`lock` и добавить
`applyComponents` на reload. Это было неверно: local 26.3
`BaseContainerBlockEntity.loadAdditional`/`saveAdditional` пишут эти поля
корневыми ключами, а `BlockEntity.loadWithComponents` читает только
`loadAdditional` + `components`, без `applyComponents`.
`collectComponents`/`removeComponentsFromTag` относятся к item/pick-пути.
Ничего из существующего формата не менялось; regression Codex
`normal_nbt_reload_preserves_identity_and_inventory_without_applying_item_components`
сохранён и проходит.

## G021 / W06-101…105 — реальные disk/item boundaries

### Найденный и исправленный разрыв (W06-103)

`BlockItem.place` вызывал только `apply_components_from_item`, то есть
`updateBlockEntityComponents`, и пропускал `updateCustomBlockEntityTag`.
Vanilla порядок: `updateBlockStateFromTag` → `updateCustomBlockEntityTag` →
`updateBlockEntityComponents` → `setPlacedBy`. Поэтому предмет с
`minecraft:block_entity_data` терял payload при установке.

- Добавлен `BlockEntity::load_item_custom_data(&NbtCompound) -> bool` —
  зеркало `TypedEntityData.loadInto(BlockEntity)`: сохранить собственные custom
  поля сущности, наложить payload через `CompoundTag.merge`, перезагрузить
  только если что-то изменилось.
- `BlockItem.place` вызывает его до `apply_components_from_item` с ванильной
  проверкой типа block entity.
- Гейт `BlockEntityType.onlyOpCanSetNbt` не реализован: в Vecta нет системы
  прав (`block_breaking.rs` уже содержит такой же TODO). Это осознанное
  расхождение, а не забытая ветка.

### Матрица карточек

| ID | Доказательство |
| --- | --- |
| W06-101 | `a_disk_save_keeps_implicit_fields_out_of_the_component_map`: `save_without_metadata` пишет корневые `CustomName`/`lock`/`Items`, а `components` содержит только явный `custom_data`; `custom_name`/`lock` в карте отсутствуют; `save_custom_only` карту не пишет |
| W06-102 | `borrowed_and_owned_load_restore_the_same_state_with_unknown_components`: `load_with_components` и `load_with_owned_components` восстанавливают имя, lock, `Items` и незнакомый компонент одинаково |
| W06-103 | `placing_a_barrel_item_loads_its_block_entity_data_before_the_components` (реальный `BlockItem::use_on`): payload `LootTable` попадает в настоящую BE, затем implicit `custom_name` применяется сверху; `item_block_entity_data_merges_into_a_live_barrel`; `applying_item_components_replaces_the_container_from_the_component` фиксирует ванильное `getOrDefault(CONTAINER, EMPTY).copyInto` |
| W06-104 | `picking_a_barrel_moves_implicit_components_onto_the_item`: pick переносит name/lock/`container`/явный компонент в предмет, `removeComponentsFromTag` + пустой payload убирают дубли, повторная установка восстанавливает состояние, а удалённый компонент не воскресает |
| W06-105 | `a_saved_barrel_restores_identity_and_contents_through_the_storage_decoder`: `save_without_metadata` → `PersistentBlockEntity` → настоящий `ChunkStorage::persistent_to_block_entity_at` → бочка с именем, lock и пятью алмазами в слоте 7, контейнер читается |

Последний тест — декодер хранилища и реальная entity factory, **не** живой
restart/unload сервера.

## Что осталось по G021

- `onlyOpCanSetNbt` (нужна система прав).
- Реальный chunk save через `ChunkMap` (в тестах используется
  `persistent_to_block_entity_at`, тот же код, что у загрузчика, но без
  асинхронного region I/O).

## G022 — не начата

`/data` как builtin отсутствует: в `vecta-core/src/command/builtins/` есть
только `execute … data` пути. Инфраструктура для live-доступа уже есть
(`BlockEntityLoadContext`, `load_with_owned_components`, `save_without_metadata`,
`set_changed`, `send_block_updated` в `command/execution/block.rs`), но сам
`get/merge/modify/remove` для block/entity/storage — отдельный крупный срез
командной поверхности. В этом сеансе не начинался, чтобы не коммитить
половину foundation.

## Проверки

| Команда | Результат |
| --- | --- |
| `cargo test -p vecta-core --lib block_entity -j 1` | 119 passed, exit 0 |
| `cargo test -p vecta-core --lib block_interaction -j 1` | 2 passed, exit 0 |
| `cargo test -p vecta-core --lib chunk_saver -j 1` | 54 passed, exit 0 |
| `cargo test -p vecta-core --lib behavior::items::block_item -j 1` | 1 passed, exit 0 |
| `cargo fmt --all --check` | exit 0 |
| `cargo test --locked --workspace -j 1` | **4252 passed / 0 failed / 18 ignored**, exit 0 |
| `cargo clippy --locked --workspace --all-targets --all-features -j 1 -- -D warnings` | exit 0 |

Логи: `D:\CodexBuilds\logs\w06-g021-full.log`,
`w06-g021-full2.log`, `w06-g021-clippy.log` (первый прогон с `absolute_paths`
в новых тестах), `w06-g021-clippy2.log` (exit 0).
Build env: target `D:\CodexBuilds\VectaMC`, TMP/TEMP `D:\CodexBuilds\tmp`,
`CARGO_INCREMENTAL=0`, последовательно с `-j 1`.

Clippy потребовал убрать `absolute_paths` из новых тестов (`crate::bootstrap`,
`simdnbt::owned::NbtList`, `std::slice`, `std::sync::Weak`); production-код не
менялся, повторный прогон — exit 0.

## Границы

- Клиент 26.3, Linux, Mojang comparison, два сетевых игрока и живой
  restart/unload сервера **не выполнялись**.
- Rust fixtures и record codec не выдаются за реальный клиент.
- Partial списка прошлого сеанса без изменений: W06-042, W06-046, W06-052,
  W06-057, W06-062; открыты water evaporation, underwater coral selection,
  SulfurCube ignition.
- `planning/night-01/report.md` и `WORKBUDDY_START_06.md` не трогались,
  SHA256 совпадают с checkpoint.

## Следующий ID

**G022 / W06-106…110** — `/data` (block/entity/storage), menu resync после
изменения, comparator/ticker/dirty, ошибочный NBT, книги кафедры. Начинать с
проверки существующего `BlockEntityLoadContext` и `command/execution/block.rs`.
