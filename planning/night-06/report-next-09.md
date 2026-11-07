# Отчёт сеанса 9 (пакет 06)

Дата: 2026-11-07. Ветка `codex/night-vanilla-03`, вход с `5d80c7cc8`
(ревью Claude сеанса 8). Проверяющий — Claude.

## Что заработало для игрока

- **Компостер работает целиком.** Клик compostable-предметом (пшеница, саженец,
  хлеб и т.д.) добавляет слой с ванильным шансом: на уровне 0 всегда
  добавляет (как в 26.3 через MatchBlock-условие configured провайдера), выше —
  взвешенный шанс `compostable/low|low_medium|medium|medium_high`, `cake`/
  `pumpkin_pie` — всегда 1. Предмет расходуется даже при неудачном броске,
  stat ITEM_USED начисляется, клиент получает level event 1500 (звук/частицы
  успеха или неудачи). Правый клик по готовому (уровень 8) — костная мука
  выпадает сверху и компостер пустеет со звуком; полный на уровне 7 созревает
  по scheduled tick со звуком READY. Компаратор читает уровень.
- **Автоматизация.** Воронка сверху кладёт по одному compostable-предмету за
  трансфер; воронка снизу забирает готовую костную муку и опустошает
  компостер; дроппер сверху кормит компостер; дроппер напротив компостера
  уровня 7 сохраняет предмет (ванильное различие «контейнер есть, но пуст»);
  воронка под пустым компостером ничего не тянет и не переключается на
  item entity.
- **Общий фундамент.** `WorldlyContainerHolder` — настоящее расширение
  `container_at` (воронка/дроппер/crafter), не компостерный особый случай.

## Реализовано

- `behavior/worldly_container_holder.rs` (новый) + `as_worldly_container_holder`
  на `BlockBehavior`; `HopperBlockEntity::container_at` спрашивает держателя
  до блок-энтити (порядок `getBlockContainer`), держатель не проваливается к
  блок-энтити.
- `behavior/blocks/building/composter_block.rs`: полный блок 26.3
  (useItemOn/useWithoutItem/addLayer/empty/extractProduce/tick/onPlace,
  level event 1500, звуки EMPTY/READY, game events BLOCK_CHANGE, stat,
  scheduleTick(20), comparator, pathfinding) плюс виртуальные
  Input/Output/Empty контейнеры; компостер без блок-энтити, как в ванили.
- Шанс — только configured providers `compostable/*`; ручных списков нет.

## Проверки (реальные результаты)

- Narrow: composter 13 passed (уровень 0, взвешенный шанс + event data,
  extraction, tick, onPlace, компаратор, воронка×3, дроппер×2, уровень 7);
  hopper+dropper 54 passed.
- `cargo test --locked --workspace -j 1`: **4341 passed / 0 failed / 18
  ignored** (вход 4329). Лог: `D:\CodexBuilds\logs\full-test-composter-final.log`.
- `cargo clippy --locked --workspace --all-targets --all-features -j 1 --
  -D warnings`: exit 0. `cargo fmt --all --check`: exit 0.
- `python scripts/audit_vanilla.py`: 3105 objects / 49 block entity types /
  227 source markers. `python -m unittest discover -s scripts`: 4 OK.
- Настоящий клиент 26.3 не проверялся.

## Коммиты

- Код среза: см. git log (`feat: ...composter...`), push в
  `origin codex/night-vanilla-03`.
- Документы/audit: отдельный docs-коммит.

## Blockers / partial

- Новых внешних блокеров нет. Снят пункт ревью «воронка не видит
  WorldlyContainerHolder».
- Открытые хвосты ревью сеанса 8 (зависимые срезы): контекст печи без
  `BLOCK_ENTITY`/`CONTAINER`; `CookingFuel`/`Compostable` без inline-формы
  `ResolvableInt`/`ResolvableFloat` (сеть 26.3 допускает inline — сверено с
  Java); `block_entity_loot_ref` в compute заполняет только тип; B-06-11
  (`environment_attribute`, owned `LevelBasedValue`, inline-условия).
- W06-539 закрыт; W06-536/537/538 (Farmland) и W06-540 (farm lifecycle) —
  ещё не начаты.

## Следующий шаг

Farmland (W06-536…538) из группы G108: moisture по соседней воде/дождю,
trampling по gamerule/entity predicate, hoe consumer. Затем хвосты ревью
сеанса 8 и раздел D файла 06_07 с пункта 29.
