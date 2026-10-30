# Night 03 — продолжение 04: отчёт для ревью

Дата: 2026-10-31. Ветка: `codex/night-vanilla-03`.
Старт: `bed5ae35aaeea70c83d7bdead8ad4686a8e504ea` (пакет `NIGHT_TASK_04.md`).
Конец: см. `git log` и раздел «Ветка и коммиты» ниже.

## Что теперь делает игрок

- Взятие результата крафта начисляет `minecraft:crafted` за фактически взятое
  количество: ЛКМ, Shift, номер слота, Q/Ctrl+Q и ПКМ. Раньше ПКМ забирал весь
  preview, а Q выбрасывал весь результат; теперь ПКМ берёт половину, Q — один
  предмет, и обе ветки считают ровно то, что ушло.
- Камнерез и кузнечный стол начисляют `crafted` за взятый стак; наковальня,
  точило и ткацкий станок — нет (как в 26.3). Устаревший preview после
  изменения входов не берётся.
- Остатки рецепта возвращаются в свой слот сетки, несовместимый остаток уходит
  в инвентарь; сдвинутый shaped-рецепт расходует только свои слоты.
- Средний клик в креативе по книжной полке копирует её книги в
  `minecraft:container`; установка предмета возвращает книги на полку. NBT
  предмета не дублирует содержимое, а `/data get block` снова показывает
  `components`, включая custom name.
- Сундук работает как контейнер: ПКМ открывает меню 3x9 с заголовком
  `container.chest`, начисляется `open_chest`, закрытие освобождает зрителя;
  содержимое переживает NBT round-trip, компаратор показывает заполненность,
  разрушение выбрасывает предметы, установка ставит сундук лицом к игроку.

## Закрытые и частичные карточки

| ID | Статус | Что проверено |
| --- | --- | --- |
| W04-001 | закрыта | `ResultSlot.remove` уважает количество; один `on_take` на взятие; регрессии на ПКМ/throw |
| W04-002 | закрыта | `ITEM_CRAFTED` за реальное количество во всех ветках взятия, включая выход > 1 и повторный крафт |
| W04-003 | закрыта, кроме item callback | pickup/shift/swap/throw/clone проверены; creative clone не ест ингредиенты и не начисляет stat |
| W04-004 | закрыта | positioned grid, возврат тары, несовместимый остаток, сдвинутый shaped-рецепт |
| W04-005 | частично | stat-часть и `onCraftedBy`-порядок сделаны; `Item.onCraftedPostProcess` заблокирован картами (W04-092..095) |
| W04-006 | частично | consumers crafting/stonecutter/smithing начисляют, anvil/grindstone/loom — нет; их собственные XP/rename awards остаются W04-041/042 |
| W04-011 | закрыта | хуки `apply_implicit_components`/`collect_implicit_components`/`remove_components_from_tag` + книжная полка |
| W04-012 | частично | размещение полки идёт через общий hook; общего пути `BLOCK_ENTITY_DATA` → NBT всё ещё нет |
| W04-013 | закрыта для блоков с BE | `includeData` копирует NBT и компоненты через per-BE hooks; проверено на полке |
| W04-014 | частично | `save_with_full_metadata` снова содержит `components`; owned/borrowed load и chunk unload/load не покрыты |
| W04-021 | частично | одиночный сундук: контейнер, меню, счётчик зрителей, компаратор, drops, NBT |

Не начаты из первого маршрута: W04-007..010 (книга рецептов), W04-015..020,
W04-022..030.

## Ветка и коммиты

- Ветка: `codex/night-vanilla-03`, запушена в `origin`, HEAD совпадает с
  remote tip.
- `52bbb0222` — feat(crafting): count taken results and award ITEM_CRAFTED
  (W04-001..006)
- `da8b47084` — feat(block-entities): collect and apply implicit item
  components (W04-011..013)
- `4fb61ced8` — feat(chest): implement the single chest container (W04-021)
- `e12315b7e` — docs: record the W04 continuation results and refresh the audit
  (этот отчёт, README и перегенерированный audit; Rust-код не менялся)
- Чужой файл `planning/night-01/report.md` остался незакоммиченным и не
  staged; других dirty paths нет.

## Проверки

Полный набор на кодовом tip `4fb61ced8` (следующий коммит — только документация), логи в `D:/CodexBuilds/logs`:

| Команда | Результат |
| --- | --- |
| `cargo fmt --all --check` | exit 0 |
| `cargo check --locked --workspace -j 1` | exit 0 |
| `cargo test --locked --workspace -j 1` | exit 0, **3909 passed / 0 failed / 18 ignored** |
| `cargo clippy --locked --workspace --all-targets --all-features -j 1 -- -D warnings` | exit 0 |
| `python -m unittest discover -s scripts` | exit 0, 4 tests OK |
| `python scripts/audit_vanilla.py` | exit 0, 3105 objects / 49 block entity types / 250 source markers |

Узкие прогоны по ходу работы: `crafted_result` 11 passed, `chiseled_bookshelf`
18 passed, `chest_block` 3 passed, `chunk::` 256 passed. Логи:
`w04_001_*.log`, `w04_011_*.log`, `w04_021_*.log`, `w04_core*.log`,
`w04_final_workspace.log`.

Baseline до сеанса был 3890 passed; прирост +19 тестов. Cargo по-прежнему
предупреждает о `unused_dependencies` для mimalloc при `--all-features`
(известное поведение dhat-heap, не подавлялось). `typos`/`prek` не запускались:
команд нет в PATH.

## Изменения foundations

- `Slot` получил `on_quick_craft`/`on_swap_craft`; реализует только
  `ResultSlot`, поэтому crafting-семантика не навязана другим меню.
- `ResultSlot` получил `CraftedStat` (`Ignored`/`Accumulated`/`TakenStack`) —
  три разных контракта 26.3 выбираются каждым меню.
- `BlockEntity` получил implicit-хуки и `remove_components_from_tag`;
  `apply_components_from_item` теперь применяет implicit-часть и хранит
  остальной patch без `block_entity_data`/`block_state`.
- `vecta-registry`: `ItemStack::set_raw`, `DataComponentPatch::clear_raw`.
- `ChestKind` получил `with_close_hook`, чтобы освобождать зрителя при
  закрытии меню.
- Extractor/codegen не менялись: новые поведения зарегистрированы через
  `#[block_behavior]`, сгенерированный Rust не редактировался.

## Границы фактической проверки

- Клиент 26.3, Linux, Mojang comparison и два настоящих сетевых клиента не
  проверялись. Все проверки — unit/harness в `vecta-core`.
- Сундук проверен одним игроком; двойной сундук, loot table, lock и
  scheduled recheck зрителей не реализованы.
- Книжная полка проверена через pick → place в тесте, не в живом клиенте.

## Блокеры и следующий шаг

- W04-005/W04-092..095: `MapItem.onCraftedPostProcess` требует
  `MapItemSavedData`; пустой hook не добавлен.
- W04-012/014: общего `updateCustomBlockEntityTag`-пути нет; NBT применяется
  per-block (жукбокс, лектерн, таблички). Нужен общий placement-путь и
  регрессия на owned/borrowed load.
- W04-022: двойной сундук требует `CHEST_TYPE`, `CompoundContainer` и
  переписывания соседа при установке.
- Ближайший независимый ID: **W04-022** (двойной сундук), затем W04-023
  (openers recheck), W04-016 (loot), W04-017 (lock/name).
