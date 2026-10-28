# Ночной сеанс 01 — отчёт для ревью Codex

## 1. Ветка и коммиты

| Параметр | Значение |
| --- | --- |
| Ветка | `codex/night-vanilla-01` |
| Исходный коммит | `180ee147ed50cac0f13ad2dd53d045e0efa54466` |
| Итоговый коммит | `bfbdd45e13fa388e5fc64b3a37324f01ccda1b1a` |
| Compare | https://github.com/Daniiltoptl/VectaMC/compare/180ee147e...bfbdd45e1 |
| Push | выполнен, ветка создана на `origin` (`codex/night-vanilla-01`) |
| Рабочее дерево | чистое, незакоммиченных изменений нет |

Коммиты сеанса:

| SHA | Тип | Содержание |
| --- | --- | --- |
| `ceb8821f3` | docs | база сеанса, `progress.md`, `blockers.md`, `scenarios.md` |
| `ca764fa5d` | feat | N02: клики мешка, mutable-содержимое, пакет выбора |
| `ac6d04fb5` | test | N02: runtime-выбор, вес и остаток |
| `1f24303d8` | docs | журнал N02 и CHANGELOG |
| `bfbdd45e1` | fix | N01 (часть): `QUICK_MOVE` с `-999` выбрасывает предмет с курсора |

## 2. Статус очереди

| ID | План | Статус |
| --- | --- | --- |
| N00 | V00 | проверено, уже работало (baseline 3748 passed) |
| N01 | V04.01–V04.03 | **частично**: один исправленный дефект, остальная сверка не закончена |
| N02 | V05.05 | **реализовано и проверено** |
| N03–N20 | — | не начато |

## 3. Что сделано по задачам

### N02 · Мешки (реализовано)

Сценарий до: левый клик мешком по слоту с предметом не переносил ничего —
`tryItemClickBehaviorOverride` отсутствовал, `BundleItem` не существовал,
содержимое мешка нельзя было наполнить или опустошить кликами, выбор предмета
не доходил до сервера.

Сценарий после:

- мешок на курсоре, левый клик по слоту — предмет переносится в мешок
  (частично, если не влезает), правый клик по пустому слоту — один стак
  возвращается в слот;
- мешок в слоте: левый клик предметом на курсоре кладёт его внутрь
  (с учётом `slot.allowModification`), правый клик пустым курсором выдаёт
  верхний стак на курсор;
- `use` из руки запускает использование на 200 тиков с анимацией `Bundle`,
  содержимое выбрасывается в ванильном порядке: первый стак сразу, далее
  каждые 2 тика после 10-го, со звуком `ITEM_BUNDLE_DROP_CONTENTS` и статом
  `ITEM_USED`;
- выбор предмета клиентом приходит пакетом и применяется к открытому меню.

Ванильные ориентиры: `BundleItem.overrideStackedOnOther`,
`overrideOtherStackedOnMe`, `toggleSelectedItem`, `dropContent`,
`AbstractContainerMenu.tryItemClickBehaviourOverride`/`setSelectedBundleItemIndex`,
`BundleContents`/`BundleContents.Mutable`,
`ServerboundSelectBundleItemPacket`, `ServerGamePacketListenerImpl.handleBundleItemSelectedPacket`.

Vecta-пути: `vecta-core/src/behavior/items/bundle.rs` (новый),
`vecta-core/src/inventory/menu/behavior.rs` (`try_item_click_behavior_override`),
`vecta-core/src/inventory/menu/mod.rs` (`clicked`, `set_selected_bundle_item_index`),
`vecta-registry/src/data_components/components/recursive_items.rs`
(`BundleContentsMut`, `remaining_units_of`),
`vecta-protocol/src/packets/game/s_select_bundle_item.rs` (новый),
`vecta-core/src/player/player_inventory/player_handlers.rs`
(`handle_select_bundle_item`).

### N01 · Инвентарь (частично)

Сверено с `AbstractContainerMenu.doClick` и
`ServerGamePacketListenerImpl.handleContainerClick`:

- `process_container_click` повторяет порядок vanilla: spectator/мёртвый игрок →
  полный resync, `still_valid`, проверка индекса слота, `state_id`,
  `suppressRemoteUpdates`, клик, применение `changed_slots`/`carried_item`,
  `resumeRemoteUpdates`, `broadcastFullState` или `broadcastChanges`;
- `do_pickup`, `do_quick_move`, `do_swap`, `do_throw`, `do_clone`,
  `do_pickup_all` и quick-craft-фазы сопоставлены с ветками `doClick`:
  расхождений в порядке операций не найдено, `safeClone` в 26.3 не проверяет
  `mayPickup` (проверено по исходнику), поэтому `do_clone` соответствует эталону;
- исправлен один подтверждённый дефект: `QUICK_MOVE` с `slot_num == -999`
  отвергался парсером, тогда как vanilla выбрасывает предмет с курсора
  (ветка `-999` общая для `PICKUP` и `QUICK_MOVE`).

Незаконченное в N01 (см. § 7): TODO в `ResultSlot::on_take`
(`vecta-core/src/inventory/slots/result_slot.rs:96`) — учёт `removeCount` и
выдача `ITEM_CRAFTED` не реализованы; сверка оставшихся сценариев
(закрытие меню при смерти/disconnect, уничтожение блочного контейнера,
несколько игроков) не проводилась.

## 4. Изменённые общие контракты

| Контракт | Изменение | Риск для потребителей |
| --- | --- | --- |
| `ItemBehavior` | добавлен `as_bundle() -> Option<&BundleItem>` (по умолчанию `None`) | низкий: дефолтная реализация, остальные поведения не затронуты |
| `MenuBehavior::clicked` → pickup | перед `do_pickup` вызывается `try_item_click_behavior_override` | средний: любой предмет с переопределением получает приоритет над обычным pickup; других переопределений пока нет |
| `Menu::clicked` | убран TODO, добавлена ветка override | поведение прочих предметов не изменилось (тесты меню зелёные) |
| `BundleContents` | добавлено поле `selected_item` (runtime-only), ручной `PartialEq` | кодек, NBT и `HashComponent` его не пишут; `PartialEq` намеренно игнорирует выбор, как vanilla `equals` |
| `BundleContentsMut` | новый публичный тип в `vecta-registry` | новый API, потребителей кроме `BundleItem` нет |
| Сеть | новый пакет `SSelectBundleItem` (`S_BUNDLE_ITEM_SELECTED` = 3, VarInt `slotId`, VarInt `selectedItemIndex`, проверка `>= -1`) | неизвестные ранее пакеты теперь декодируются; id взят из сгенерированной таблицы, не выдуман |
| `Click::parse` | `QUICK_MOVE` + `-999` → `DropCarried` | только для пакетов, которые клиент обычно не отправляет |

Сохранения: формат содержимого мешка не менялся (только `items`), миграции не
требуются. Codegen/extractor не трогались; сгенерированный Rust не правился.

## 5. Дефекты и проверки

Воспроизведённые дефекты:

1. Мешок не работал вовсе: `as_bundle`/`BundleItem` отсутствовали, TODO в
   `menu/mod.rs` фиксировал отсутствие override.
2. `QUICK_MOVE` с `-999` игнорировался: предмет оставался на курсоре вместо
   выбрасывания.

Регрессии закреплены тестами:

- `vecta-core/src/inventory/menu/tests.rs`: 5 тестов кликов мешка
  (перенос в мешок, выдача стака на курсор, запрет шулкера, остаток при
  переполнении, выбор только для мешка) + `set_selected_bundle_item_index`
  на не-мешке;
- `vecta-registry/.../recursive_items.rs`: выбор не попадает в кодек/NBT/хеш,
  `max_amount_for` = 24 при 40/64 и остаток 40 остаётся вызывающему;
- `vecta-protocol/.../s_select_bundle_item.rs`: чтение полей и отказ при
  индексе < -1;
- `vecta-core/src/inventory/click.rs`: `QUICK_MOVE` + `-999` для обеих кнопок.

Фактически выполненные проверки:

| Команда | Результат |
| --- | --- |
| `cargo test --locked -p vecta-core --lib -j 1` | 2883 passed, 0 failed, 1 ignored (лог `D:\CodexBuilds\logs\test-core-n02.log`) |
| `cargo test --locked -p vecta-registry --lib bundle` | 5 passed |
| `cargo test --locked -p vecta-protocol --lib select_bundle` | 2 passed |
| `cargo test --locked -p vecta-core --lib inventory::click` | 7 passed |
| `cargo clippy -p vecta-core -p vecta-registry -p vecta-protocol --all-targets --all-features -- -D warnings` | чисто |
| `cargo fmt --all --check` (через `cargo fmt --all`) | чисто |

`--locked` использовался везде; baseline сеанса — 3748 passed по workspace.

## 6. Что не проверено

| Проверка | Статус |
| --- | --- |
| Настоящий клиент 26.3 | не выполнено: клиента и принятой EULA нет |
| 2+ игрока одновременно | не выполнено |
| Mojang comparison | не выполнено |
| unload/restart с мешком в инвентаре | не выполнено; проверен только кодек компонента |
| Linux | не выполнено: Windows-машина |

## 7. Ограничения, блокеры и следующая задача

Оставшиеся игровые ограничения по N02:

- подсказка/tooltip содержимого мешка на клиенте не реализована
  (`BundleTooltip` — клиентская часть, серверного поведения не требует);
- `isBarVisible`/`getBarWidth` (полоска заполнения) в 26.3 — клиентский
  рендер по компоненту, серверных пакетов не требует, поэтому не реализовано;
- синхронизация выбора проверена на уровне пакета и меню, визуально — нет.

Блокеры (в `planning/night-01/blockers.md` пока пусто, потому что ни одна
задача не была остановлена вопросом):

- для N01 остаётся открытым вопрос по `ResultSlot`: нужен ли `on_crafted_by`
  hook в `ItemBehavior` (в vanilla его переопределяет только `MapItem` через
  `onCraftedPostProcess`), или достаточно выдавать `ITEM_CRAFTED` стат.
  Без решения владельца реализовывать хук «на будущее» нельзя (AGENTS:
  никаких заготовок без потребителя).

Следующая безопасная задача: **N03 (книги)** — она не зависит от спорного
контракта `ResultSlot`, а её точки входа изолированы. Уточнение по факту
проверки: в `vecta-protocol` нет ни `ServerboundEditBookPacket`, ни
`ClientboundOpenBookPacket` (файлов `s_edit_book.rs`/`c_open_book.rs` нет,
id `S_EDIT_BOOK = 24` и `C_OPEN_BOOK = 59` есть только в сгенерированной
таблице), поэтому N03 начинается с двух пакетов и трёх поведений предмета —
это объём на отдельный сеанс, а не «добить по мелочи».
Перед ней стоит добить оставшуюся сверку N01 (закрытие меню,
смерть/disconnect, несколько игроков) и решить вопрос по `ResultSlot`.
