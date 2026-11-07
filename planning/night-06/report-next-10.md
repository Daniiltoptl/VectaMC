# Отчёт сеанса 10 (пакет 06, night-vanilla-03)

Дата: 2026-11-07. Старт: HEAD `62694c7f3` (ревью компостера Клодом),
baseline **4342 passed / 0 failed / 18 ignored**.

## Что сделано

Тема сеанса — **Farmland** (G108: W06-536 влага, W06-537 trampling,
W06-538 мотыга, W06-540 жизненный цикл фермы). Перед кодом целиком прочитаны
и сверены `FarmlandBlock.java` и `CropBlock.java` (26.3), включая то, чей
RandomSource использует каждый вызов.

### 1. Farmland (`vecta-core/src/behavior/blocks/vegetation/farmland_block.rs`, переписан)

Реализованы/исправлены все методы ванили 26.3:

- `getStateForPlacement`: если default-состояние не выживает (сверху
  немaintaining solid) — кладём **базовый блок dirt** (точная ваниль),
  иначе default moisture 0. Проверено placement-контекстом через реальный
  `BlockPlaceContext` на живом test-world.
- `updateShape`: сосед сверху при `!canSurvive` → `scheduleTick(1)`; боковые
  соседи ничего не планируют.
- `canSurvive`: `!above.isSolid() || shouldMaintainFarmland` — тег
  `MAINTAINS_FARMLAND` из extracted данных вместо прежнего ручного списка
  блоков (тег шире: fence gates, torchflower, moving_piston — прежний список
  был неполным).
- `tick`: `!canSurvive` → revert в dirt.
- `randomTick`: гидратация до 7 при воде рядом или дожде
  (`isRainingAt(above)`), иначе −1 шаг, на 0 без crop → dirt.
  `isNearWater` теперь ровно ваниль — проверка **fluid state**
  (источник/поток/waterlogged) в боксе (-4,0,-4)..(4,1,4) через
  `get_fluid_state().is_water()` вместо хардкода «WATER или waterlogged».
- `fallOn` (W06-537): бросок **из level random** на каждом приземлении до
  любых условий (`level.getRandom().nextFloat() < fallDistance - 0.5`),
  затем living-entity, затем игрок/mobGriefing, затем размер
  (w²·h > 0.512). Раньше был thread `rand::random` — расхождение того же
  класса, что ловило ревью сеанса 9 (чей RandomSource).
  Spawn-protection для игроков не смоделирована (см. blockers).
- `isPathfindable = false` (ванильный override).
- `turnToBaseBlock` — общий `turn_to_dirt` (pushEntitiesUp + game event).

### 2. Crop growth — RNG-поток исправлен

`CropLike::on_random_tick`: было `rand::random::<u32>().is_multiple_of(...)`
(thread RNG, неверная формула), стало ванильное
`nextInt((int)(25.0F / growthSpeed) + 1) == 0` на **level random**
(`world.random().with_level`). Формула скорости `crop_growth_speed`
(GROWS_CROPS, влага>0 → 3.0, соседи /4, скученность /2) сверена с Java —
совпадает, закреплена детерминированным тестом.

### 3. W06-538 мотыга — проверена, не дублирована

В 26.3 `HoeItem` — data-driven `BlockTransformers.HOE`. Extracted данные в
Vecta (`vanilla_block_transformers.rs`) совпадают с Java 1:1:
`all_of(turns_into_farmland, air@offset(0,1,0)) → farmland(moisture 0)`;
`coarse_dirt + air → dirt`; `rooted_dirt → dirt` + loot
`minecraft:till/rooted_dirt` (clicked_face); запрещённая грань DOWN; звук
`ITEM_HOE_TILL`. Закреплено тестами через реальный `ITEM_BEHAVIORS...use_on`:
dirt→farmland; dirt с блоком сверху → Pass (ванильный air-предикат);
coarse_dirt→dirt; grass_block→farmland; rooted dirt уже был покрыт.

### 4. W06-540 жизненный цикл фермы — тесты по реальным путям

- Рост wheat на гидратированной грядке под полным небом через реальный
  `random_tick` (инвариант: растёт в пределах 10 000 тиков при p=1/7).
- Trampling ломает crop: жёсткое падение → farmland→dirt, wheat сверху
  исчезает (update_shape куста), короткое падение (fallDistance 0.5) —
  никогда не tramples (бросок невозможен: nextFloat < 0).
- Высыхание −1 шаг, revert в dirt, удержание влаги-0 под crop, revert по
  scheduled tick под перекрытием, гидратация рядом с водой.
- Тесты trampling детерминированы краями ванильного условия — ни один тест
  не зависит от удачного броска (правило сеанса 9).

## Проверки

- `cargo fmt --all --check` — exit 0.
- `cargo test --locked --workspace -j 1` — **4357 passed / 0 failed /
  18 ignored** (было 4342; +15: farmland 14, crop growth/speed +3,
  transformer +2, минус переупорядочивание фильтров).
- `cargo clippy --locked --workspace --all-targets --all-features -j 1 --
  -D warnings` — exit 0 (поправлены redundant closure → `Random::next_f32`,
  absolute paths, assert_is_empty).
- Пост-коммитные audit-скрипты — см. ниже.

## Найдено попутно (записано в blockers.md)

1. **Bonemeal на thread RNG** (`behavior/items/bonemeal.rs`:
   `rand::rng()` для success/разброса; ваниль — `level.getRandom()`).
   Кандидат на маленький самостоятельный срез.
2. **Spawn protection не смоделирована**: ванильный player-branch
   `fallOn` требует `!isUnderSpawnProtection` (ops list, radius, respawn
   data). Реализовано без проверки = ваниль при пустом списке опов.
3. **Порядок-зависимый тест** `copper_chest_transforms_...` в
   block_transformer: при узком фильтре падает и на чистом HEAD ("Block
   entity registry not initialized"); в полном проге проходит. Лечится
   `init_globals()` в fixture `Interaction::new`.

## Коммит

Код+журнал одним коммитом на `codex/night-vanilla-03`, push в origin.
Следующий шаг: хвосты ревью при первом consumer (inline Resolvable*,
контекст печи, `block_entity_loot_ref`, B-06-11), bonemeal→level random,
затем раздел D файла 06_07 с пункта 29.
