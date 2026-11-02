# Night 06 — отчёт сеанса 2 (G006, G007, G009…G013)

Дата: 2026-11-02. Ветка `codex/night-vanilla-03`.
Вход: HEAD `cfc95d0a3`, workspace **4198 passed / 0 failed / 18 ignored**.
Выход: см. «Проверки» ниже и `git log` в конце.

## Что сделано

| Группа | Карточки | Статус |
| --- | --- | --- |
| G006 | W06-026…030 наполненные вёдра | реализовано и проверено |
| G007 | W06-031…035 подбор пустым ведром | реализовано и проверено |
| G009 | W06-041…045 equippable | проверено; body-slot partial |
| G010 | W06-046…050 огниво | реализовано; fire charge partial |
| G011 | W06-051…055 костная мука | реализовано; sapling partial |
| G012 | W06-056…060 бутылочки | реализовано; bee release partial |
| G013 | W06-061…065 ножницы | реализовано; соты partial |

## G006 / G007 — вёдра (commit `58bfb86db`)

Раздатчик не знал ни одного ведра: импульс выбрасывал предмет. Добавлена
capability `DispensibleContainerItem` (`empty_contents`) и аксессор
`as_dispensible_container` по образцу `SignApplicator`.

- `BucketItem.emptyContents` вынесен в одну функцию, которую делят ручной путь и
  раздатчик; сохранены ванильные sneak-правила и retry на соседний блок.
- `SolidBucketItem.emptyContents` для powder snow.
- `BlockSource::container` + `DefaultDispenseItemBehavior::consume_with_remainder`
  и `add_to_inventory_or_dispense`.
- `pickup_bucket_content` общий для руки и пустого ведра в раздатчике, включая
  waterlogged fallback.
- Зарегистрированы water/lava/powder_snow и пустое ведро.

Проверено: размещение source-воды, waterlogging в `OAK_SLAB`, лава со своим
звуком (`ITEM_BUCKET_EMPTY_LAVA`), powder snow со своим `place_sound`,
отклонённая цель → default eject, полностью занятый раздатчик, обе руки и
creative, source vs flowing (flowing не подбирается), обновление соседа
(лилия без опоры ломается и дропается).

## G009 — equippable (commit `b28f141c0`)

Код `EquipmentDispenseItemBehavior` уже совпадал с 26.3
(`canEquipWithDispenser`: alive, не spectator, `dispensable`, `canUseSlot`,
`canBeEquippedBy`, пустой слот, `canDispenserEquipIntoSlot`). Добавлены тесты:
занятый слот не перезаписывается, седло отвергается овцой (тег
`#minecraft:can_equip_saddle`), свинья с седлом сохраняет седло,
`PersistenceRequired` и drop chance через настоящий persistent record и
entity factory.

Body-slot (волк/лошадь) partial: таких мобов в Vecta нет, фиктивных не заводил.

## G010 — огниво (commit `d14a4b01b`)

`FlintAndSteelDispenseItemBehavior` не был зарегистрирован. Реализовано:
костёр/свеча/candle cake → `LIT=true`, огонь на пустой цели, `TntBlock::prime`
+ удаление блока, один пункт прочности, события 1000/1001 + 2000.

Важная деталь: в 26.3 `success` остаётся true, даже если ни одна ветка не
сработала, поэтому огниво из раздатчика изнашивается и о занятую цель. Тест это
фиксирует как ванильную особенность, а не как желаемое поведение.

`FIRE_CHARGE` намеренно не зарегистрирован: vanilla регистрирует его как
projectile (`SmallFireball`). Тест
`a_fire_charge_stays_unregistered_until_the_small_fireball_exists` это
фиксирует; см. blockers B-06-02.

## G011 — костная мука (commit `65d3817d1`)

`BoneMealDispenseBehavior` использует те же `grow_crop` / `grow_water_plant`, что
и ручной путь; `grow_water_plant` теперь принимает `Option<Direction>` как
ванильный `clickedFace`. Проверено: рост пшеницы, максимальный возраст,
seagrass в воде, неподходящая цель, расход ровно одного предмета, событие 1505
и пара 1000/1001.

W06-052 partial: у `SaplingBlock` нет `Bonemealable`, нет аналога `TreeGrower`.
См. blockers B-06-01 (все нужные configured features уже есть в generated data).

## G012 — бутылочки (commit `fb9ec81ed`)

`GlassBottleDispenseBehavior`: полный улей → мёд + `resetHoneyLevel`, любое
water-состояние → water potion, иначе eject. `eject` разделён на `spawn_item` и
`eject`, чтобы неуспешная ветка могла выбросить предмет без звука успеха.

Проверено: source-вода, flowing-вода (раздатчик принимает её, ручной путь
raycast-ит только source), полный улей, неполный улей, неподходящая цель,
свободный слот, полностью занятый раздатчик.

W06-057 partial: `releaseBeesAndResetHoneyLevel` не выпускает пчёл — нет `Bee`
entity; honey level сбрасывается, бутылка выдаётся. См. blockers B-06-03.

## G013 — ножницы (commits `e485e254a`, `1f277a4b7`)

Добавлена capability `Shearable` (`ready_for_shearing` / `shear`) с аксессором
`as_shearable` по образцу `as_living_entity`; реализована овцой и mushroom cow
поверх существующих shear-consumers. `ShearsDispenseItemBehavior` выбирает по
AABB только живых и готовых к стрижке и стрижёт ровно одну, затем `SHEAR` game
event и один пункт прочности.

Проверено: полная овца (белая шерсть + износ), уже остриженная овца, две овцы в
одном блоке (стрижётся одна), пустая цель.

W06-062 partial: соты из улья требуют `BeehiveBlock.dropHoneycomb` и `Bee`
entity. См. blockers B-06-03.

## Проверки

| Команда | Результат |
| --- | --- |
| `cargo test -p vecta-core --lib dispenser -j 1` | 79 passed, exit 0 |
| `cargo test -p vecta-core --lib bucket -j 1` | 26 passed, exit 0 |
| `cargo test -p vecta-core --lib dispenser_block::tests::equipment -j 1` | 11 passed, exit 0 |
| `cargo test -p vecta-core --lib dispenser_block::tests::flint_and_steel -j 1` | 8 passed, exit 0 |
| `cargo test -p vecta-core --lib dispenser_block::tests::bonemeal -j 1` | 4 passed, exit 0 |
| `cargo test -p vecta-core --lib dispenser_block::tests::bottle -j 1` | 7 passed, exit 0 |
| `cargo test -p vecta-core --lib dispenser_block::tests::shears -j 1` | 4 passed, exit 0 |
| `cargo test -p vecta-core --lib dispenser_block::tests::empty_bucket -j 1` | 8 passed, exit 0 |
| `cargo fmt --all --check` | exit 0 |
| `cargo test --locked --workspace -j 1` | **4240 passed / 0 failed / 18 ignored**, exit 0 |
| `cargo clippy --locked --workspace --all-targets --all-features -j 1 -- -D warnings` | exit 0 |

Логи: `D:\CodexBuilds\logs\w06-buckets-full.log`,
`w06-buckets-clippy.log` (первый прогон, три находки),
`w06-buckets-clippy2.log` (exit 0). Build env: target
`D:\CodexBuilds\VectaMC`, TMP/TEMP `D:\CodexBuilds\tmp`, `CARGO_INCREMENTAL=0`,
последовательно с `-j 1`.

Первый Clippy выявил `redundant_closure_for_method_calls` и два `absolute_paths`
в новом коде; исправлено отдельным коммитом `1f277a4b7`, после него Clippy
exit 0 и затронутые shears-тесты повторно прошли.

## Границы

- Клиент 26.3, Linux, Mojang server comparison и два сетевых игрока **не
  запускались**. Всё проверено серверными тестами и уровнем событий.
- Живой restart/unload сервера не выполнялся: persistent-record round-trip идёт
  через настоящий codec и entity factory, но не через перезапуск.
- Partial: W06-042 (body slot), W06-046 (fire charge), W06-052 (sapling),
  W06-057 (bee release), W06-062 (соты). Причины и точные зависимости — в
  `blockers.md`.
- `planning/night-01/report.md` и `WORKBUDDY_START_06.md` не изменялись:
  SHA256 `06680CBC…522` и `48430DC1…D5A` совпадают с checkpoint.

## Следующий ID

**G021 / W06-101…105** — implicit components round-trip, затем G022, G024, G025,
G026 и далее по `route.md`. Перед ними имеет смысл вернуться к B-06-01
(`TreeGrower`), если приоритет на P0-карточках G011.
