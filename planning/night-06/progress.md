# Night 06 — progress

## Сеанс7: ContextMap и все текущие loot consumers

Реализована единая typed ContextMap с сохранением borrowed values и shared RNG.
Удалены отдельные параметры LootContext и LootContextRef. Generator использует
produced SteelExtractor output:16ключей/31param set. Текущие игровые consumers
переведены и проверяют свои sets; исправлены chest THIS_ENTITY, условие
KilledByPlayer по LAST_DAMAGE_PLAYER и block origin/empty tool.

Код `2a0ef1daf`: workspace4304/0/18, Clippy/fmt exit0, Python4,
audit3105/49/230. Детали: [report-next-07.md](report-next-07.md).
Код второго среза `70f27a545`: общий read context домена с shared Arc
scoreboard/storage реализован. Live Score/Storage/EntityScores и все текущие
числовые loot consumers используют полный контекст; identity profile/UUID,
изоляция доменов и boxed numeric casts проверены сквозными tests.
Workspace4308/0/18, Clippy/fmt exit0, Python4, audit3105/49/227.
SlotProvider consumer ещё не реализован. Повторно согласовывать владение не надо.
Provider type output уже
получен:28float+23int=51, а не49; новая factory/compute ещё не реализована.
Код `8b8140b35` добавил extractor configured values (4float+26int, Mojang codec
round-trip) и убрал silent fallback генератора на Constant. Output остаётся
в SteelExtractor до consumer. Последний узкий loot suite37/0/0,
registry Clippy/fmt exit0; полный4308/0/18 был перед этой правкой generator.

## Актуальное ревью сеанса6

Проверен incoming `65716df85..e1efc174b`; общий living loader, region I/O,
Team/UUID и break callback сохранены. Исправлены numeric/partial codec semantics,
Packed.apply с независимыми operation indices, raw NBT effect load без gameplay
packet hooks, setPosToBed вместо startSleeping, numeric UUID streams и combat
references/expiry в пределах domain. Добавлены5 сквозных server tests и1
attribute regression; старые attribute tests вынесены в отдельный модуль.

Код `a91820a33`: workspace **4299/0/18**, Clippy/fmt exit0, Python4,
audit3105/49/230. Cargo manifest warning mimalloc сохранён и объяснён в
[ревью](../reviews/night-06-session06-codex.md).
Brain/locator-bar icon и durability criteria остаются открытыми; это не full
vanilla LivingEntity parity. Полное ядро vanilla всё ещё не готово.

В [большом запросе07](../../WORKBUDDY_CONTINUE_06_07.md) срез ContextMap и
миграции callers теперь реализован; остаются реальные provider registries и
compute, затем вся очередь направлений3–12.
Новый вопрос о выборе этой архитектуры не требуется. Seeded RNG/domains сохранять.

Ветка `codex/night-vanilla-03`. Стартовый HEAD `3a79ec57f`
(`docs: expand vanilla implementation specification to 600 work items`).
Эталон: локальная Java 26.3, `minecraft-src/.vecta-reference.json`, 5037 файлов,
workspace `0.16.1+mc26.3`.

Не трогается: `planning/night-01/report.md` (чужое изменение) и
`WORKBUDDY_START_06.md` (чужая незакоммиченная правка порядка строк).

## G001 / W06-001…005 — пузырёк опыта из раздатчика

Старый ID: W05-001, продолжение W04-032. Статус: **реализовано и проверено**
на серверном уровне; настоящий клиент не запускался.

### Что было

`ThrownExperienceBottleEntity` и `ExperienceBottleItem` существовали и работали
(полёт, гравитация 0.07, события 2002/1053, опыт `3 + nextInt(5) + nextInt(5)`,
discard, NBT). Раздатчик знал только снежок: в
`register_projectile_dispense_behaviors` была одна регистрация, поэтому импульс
по раздатчику с пузырьком уходил в default-dispatch и выбрасывал предмет как
item entity.

### Что изменилось для игрока

Импульс раздатчика с пузырьком опыта теперь запускает настоящий
`ThrownExperienceBottle` с ванильной конфигурацией, расходует один предмет и
даёт те же события, что Java. Попадание разбивает пузырёк и выдаёт опыт через
существующий consumer.

### Ванильные методы

- `world/item/ExperienceBottleItem.java`: `createDispenseConfig` (uncertainty
  `DEFAULT.uncertainty * 0.5` = 3.0, power `DEFAULT.power * 1.25` = 1.375,
  position function по умолчанию).
- `core/dispenser/ProjectileDispenseBehavior.execute` (позиция → shoot → shrink 1)
  и `playSound` (событие 1002; анимация 2000 наследуется).
- `world/entity/projectile/throwableitemprojectile/ThrownExperienceBottle.onHit`
  (события 2002/1053, опыт, discard).

### Изменения Vecta

- `behavior/blocks/redstone/dispenser_block.rs`: регистрация
  `vanilla_items::EXPERIENCE_BOTTLE` в `register_projectile_dispense_behaviors`
  с `DispenseConfig { uncertainty: DEFAULT_UNCERTAINTY * 0.5, power: DEFAULT_POWER * 1.25, ..DEFAULT }`.
- Тестовый helper `snowballs` обобщён до `projectiles_of_type(fixture, pos, entity_type)`
  и переиспользован снежком, экипировкой и пузырьком.
- `tests/experience_bottle.rs` (новый): три сценария раздатчика.
- `thrown_experience_bottle/tests.rs`: сценарий «загруженный пузырёк всё ещё
  выдаёт опыт и удаляется при попадании».

### Карточки

| ID | Результат |
| --- | --- |
| W06-001 | запуск: позиция `center + facing*0.7 + (0,0.1,0)`, power 1.375, uncertainty 3.0 — тест `dispensed_experience_bottle_launches_one_projectile_from_the_java_position` |
| W06-002 | компоненты: стак с `CUSTOM_NAME` переносится на снаряд с count 1, исходный теряет ровно 1 — тест `..._keeps_the_dispensed_stack_components` |
| W06-003 | попадание и опыт через существующий `ExperienceOrbEntity::award_with_direction` — уже покрыто `block_impact_awards_experience_toward_each_hit_face_and_discards_bottle` и `entity_impact_...` |
| W06-004 | события 1002 + 2000 и расход при rejected insertion без возврата предмета — тест `rejected_experience_bottle_insertion_still_consumes_and_emits_events` |
| W06-005 | save/load сохраняет состояние (существующий `saved_bottle_preserves_shooter_and_rendered_item_components`), попадание после load выдаёт опыт и удаляет снаряд — тест `a_loaded_bottle_awards_experience_and_is_discarded_on_hit` |

### Проверки

```
cargo check -p vecta-core --lib --tests -j 1                     → exit 0
cargo test -p vecta-core --lib dispenser -j 1                    → 34 passed, 0 failed
cargo test -p vecta-core --lib experience_bottle -j 1            → 8 passed, 0 failed
cargo fmt --all --check                                          → exit 0
```

Логи: `D:\CodexBuilds\logs\w06-g001-*.log`. Прежний фильтр раздатчика был
31 passed (ревью W04-019-034); 31 + 3 новых = 34.

### Границы

- Клиент 26.3 не запускался: стартовый звук/частицы и полёт проверены на уровне
  событий и состояния, не глазами.
- Опыт выдаётся через `award_with_direction`; раскладка орбов по попаданию
  проверена существующими тестами, но не сверялась с живым Mojang-сервером.

## G002 / W06-006…010 — три варианта яиц

Старый ID: W05-002, продолжение W04-032. Статус: **реализовано и проверено**
на серверном уровне; клиент не запускался.

### Что было

`EggItem` уже был привязан кодогеном ко всем трём предметам (`egg`, `blue_egg`,
`brown_egg`), `ThrownEggEntity` умел вылупление с ванильными вероятностями и
наследованием `chicken/variant`, а `BLUE_EGG`/`BROWN_EGG` несут вариант в
извлечённых данных предмета. Не было регистрации в раздатчике и тестов на
ручное использование.

### Что изменилось для игрока

Импульс раздатчика с любым из трёх яиц запускает настоящий `ThrownEgg`
(единый entity type `minecraft:egg`), расходует одно яйцо и сохраняет вариант
курицы. Ручной бросок из обеих рук расходует яйцо только в survival.

### Ванильные методы

- `world/item/EggItem.use` (звук, `spawnProjectileFromRotation(…, 0.0, 1.5, 1.0)`,
  стат, расход). `EggItem` **не** переопределяет `createDispenseConfig`, поэтому
  раздатчик использует `DispenseConfig.DEFAULT` (power 1.1), а не 1.5.
- `world/entity/projectile/throwableitemprojectile/ThrownEgg.onHit`
  (`nextInt(8)`, `nextInt(32)` → 1 или 4 цыплёнка, `setAge(-24000)`, `snapTo`,
  вариант из компонента).

### Изменения Vecta

- `dispenser_block.rs`: регистрация `EGG`, `BLUE_EGG`, `BROWN_EGG` с
  `DispenseConfig::DEFAULT` и общей фабрикой `ThrownEggEntity`.
- `behavior/items/egg/tests.rs` (новый): матрица ручного броска.
- `dispenser_block/tests/egg.rs` (новый): три цвета через раздатчик.
- `thrown_egg.rs`: тест сохранения варианта в NBT.

### Карточки

| ID | Результат |
| --- | --- |
| W06-006 | три item-варианта через общий consumer, velocity = Java dispenser config — `dispensed_eggs_launch_one_projectile_that_keeps_the_egg_item` |
| W06-007 | вариант проходит item → projectile → (существующий) hatch — тест на компонент у снаряда + существующий `hatchling_inherits_egg_variant_component` |
| W06-008 | вылупление, возраст, вероятности — существующие `hatch_count_matches_vanilla_odds`, `hatchling_spawns_baby_chicken_at_egg_position`, `hatchling_fits_when_egg_lands_on_top_of_solid_block`, `hatchling_does_not_spawn_fully_enclosed_in_solid_block` |
| W06-009 | обе руки × survival/creative, расход и стат — `egg_use_spawns_owned_projectile_and_consumes_only_in_survival` |
| W06-010 | reload сохраняет вариант — `saved_egg_keeps_its_variant_component`. Вторая половина карточки: повторного вылупления нет, потому что попадание делает `set_removed(Discarded)`; отдельного guard внутри `on_hit` в vanilla нет, и добавлять его значило бы разойтись с эталоном |

### Проверки

```
cargo test -p vecta-core --lib egg -j 1        → 19 passed, 0 failed
cargo test -p vecta-core --lib dispenser -j 1  → 35 passed, 0 failed
cargo fmt --all --check                        → exit 0
```

Логи: `D:/CodexBuilds/logs/w06-g002-*.log`.

### Границы

- Клиент не проверялся: частицы разбивания (`handleEntityEvent` 3) и вид
  снаряда подтверждены только серверным событием.
- Вероятности вылупления проверены функцией `hatch_count`, а не статистическим
  прогоном на живом сервере.

## G003 / W06-011…015 — взрывное зелье из раздатчика

Старый ID: W05-003, продолжение W04-032. Статус: **реализовано и проверено**
на серверном уровне; клиент не запускался.

### Что было

`SplashPotionItem` и `SplashPotionEntity` существовали и были полными
(радиус 4, спад по расстоянию, мгновенные и timed-эффекты, тушение огня,
knockback, save/load), но у предмета стоял TODO «Implement the `ProjectileItem`
dispenser dispatch», и раздатчик выбрасывал зелье как предмет.

### Что изменилось для игрока

Импульс раздатчика с splash-зельем запускает настоящий `ThrownSplashPotion` с
ванильной конфигурацией, без владельца, расходует ровно одну единицу и
сохраняет `potion_contents`/`potion_duration_scale` на всём пути полёта.
При попадании эффекты достаются целям в радиусе 4 со спадом.

### Ванильные методы

- `world/item/ThrowablePotionItem.createDispenseConfig` (uncertainty × 0.5,
  power × 1.25; `SplashPotionItem` его не переопределяет).
- `world/entity/projectile/throwableitemprojectile/AbstractThrownPotion.onHit`
  (`SPLASH_RANGE = 4.0`, `1 - distance / 4`, фильтр spectator/`isAffectedByPotions`,
  мгновенные эффекты через `applyInstantaneousEffect`).

### Изменения Vecta

- `dispenser_block.rs`: общая константа `THROWN_CONSUMABLE_CONFIG` (её же теперь
  использует пузырёк опыта) и регистрация `SPLASH_POTION`.
- `dispenser_block/tests/potion.rs` (новый): запуск и сохранение содержимого.
- `splash_potion.rs`: два теста на эффекты (спад по расстоянию и мгновенный эффект).

### Карточки

| ID | Результат |
| --- | --- |
| W06-011 | запуск с ванильной config и без владельца — `dispensed_splash_potion_launches_one_unowned_projectile` |
| W06-012 | `potion_contents` сохраняются на снаряде — `dispensed_splash_potion_keeps_its_potion_contents` |
| W06-013 | мгновенный эффект идёт через instant-consumer, timed не остаётся — `splash_instant_effect_damages_the_target_without_leaving_a_timed_effect` |
| W06-014 | две цели на разных расстояниях, длительность падает — `splash_applies_timed_effects_with_distance_falloff` |
| W06-015 | при ударе зелье удаляется (`on_hit_discards_the_splash_potion`), расход один; save/load зелья уже покрыт `save_additional`/`load_additional` и общим тестом throwable-item. Отдельного «повторного hit» нет по той же причине, что у яиц: vanilla не имеет guard внутри `onHit` |

### Проверки

```
cargo test -p vecta-core --lib splash_potion -j 1  → 5 passed, 0 failed
cargo test -p vecta-core --lib dispenser -j 1      → 37 passed, 0 failed
cargo fmt --all --check                            → exit 0
```

Логи: `D:/CodexBuilds/logs/w06-g003-*.log`.

### Границы

- Клиент не проверялся: цвет частиц и звук разбивания подтверждены уровнем
  событий, не глазами.
- `W06-013` «attribution» проверена тем, что мгновенный эффект применяется через
  общий consumer; конкретный source/damage-type в живом клиенте не наблюдался.

## Codex review · 2026-11-02 · G001–G003

Входящий диапазон `3a79ec57f..78da455e5` сверён с local Java 26.3.
Ошибок новых игровых registrations не подтверждено. Добавлен общий бинарный
persistent-record/factory round-trip пяти новых projectile items: UUID,
position/velocity/rotation, item/components и отсутствие owner. Усилены potion
tests: durations 6300/2481 при scale 1.75, outside-range цель, ровно 6 damage,
INDIRECT_MAGIC, direct projectile allocation и отсутствие causing owner.

Свежие узкие проверки: dispenser_block — 27 passed, splash_potion — 5 passed;
workspace — **4198 passed / 0 failed / 18 ignored**, exit 0. Clippy/fmt — exit 0;
после test-only lint fix persistent round-trip повторно прошёл (1 test).
Python — 4 tests OK, audit — 3105 objects / 49 BE / 233 markers.
Полные проверки и commits — [отдельный review](../reviews/night-06-g001-g003-codex.md).

ТЗ уточнено: fire charge из dispenser требует SmallFireball, а не прямого
поджигания; Java не требует guard от принудительного повторного onHit.
G004/G005 — зависимости реализации, не внешний blocker только из-за отсутствия
Rust consumer. Следующий ID: **G006 / W06-026…030**.

## G006…G013 · сеанс 2

Полные детали, проверки и границы — в [report-next-02.md](report-next-02.md).
Кратко по группам:

| Группа | Что сделано | Статус |
| --- | --- | --- |
| G006 | `DispensibleContainerItem` capability, общий `BucketItem.emptyContents`, `SolidBucketItem.emptyContents`, регистрация water/lava/powder_snow | реализовано |
| G007 | `pickup_bucket_content` общий для руки и раздатчика, `consume_with_remainder`, регистрация пустого ведра | реализовано |
| G009 | тесты занятого слота, тега `can_equip_saddle`, persistent round-trip свиньи с седлом | проверено; body-slot partial |
| G010 | `FlintAndSteelDispenseItemBehavior` (костёр/свеча/огонь/TNT, прочность, 1000/1001) | реализовано; fire charge partial |
| G011 | `BoneMealDispenseBehavior` на общих `grow_crop`/`grow_water_plant` | реализовано; sapling partial |
| G012 | `GlassBottleDispenseBehavior` (мёд, вода source и flowing), `spawn_item`/`eject` | реализовано; bee release partial |
| G013 | capability `Shearable`, `ShearsDispenseItemBehavior` (одна цель из AABB) | реализовано; соты partial |

Свежие проверки: workspace **4240 passed / 0 failed / 18 ignored** (exit 0),
`cargo fmt --all --check` exit 0, workspace Clippy exit 0 после коммита
`1f277a4b7`. Узкие: dispenser 79, bucket 26, equipment 11, flint 8, bonemeal 4,
bottle 7, shears 4, empty_bucket 8.

Partial и их точные зависимости — `blockers.md` (B-06-01…03).
Следующий ID: **G021 / W06-101…105**.

## Codex review сеанса 2 · 2026-11-02

Входящий диапазон `cfc95d0a3..4afc44fd7` сверён с local Java 26.3.
Исправлены: lava-canPlaceLiquid с WATER вместо содержимого; потеря ножниц при
неудаче; пропущенный leash-first path; категория Players вместо Blocks при
dispenser shearing; отсутствие final 1000/2000 у bucket consumers.

B-06-04 снят как неверный: Vanilla сохраняет корневые CustomName/lock и не
вызывает applyComponents при обычном reload. Новый normal-NBT regression
защищает имя, lock и inventory бочки. Общая persistence foundation не менялась.
Исправлена ссылка B-06-03 на HARVEST_BEEHIVE и уточнены остальные partial.

Свежие проверки: dispenser — 83 passed; normal_nbt_reload — 1 passed;
workspace — **4245 passed / 0 failed / 18 ignored**, exit 0; Python — 4 tests OK.
Окончательные проверки и commits —
[review](../reviews/night-06-g006-g013-codex.md).
Следующий ID: **G021/W06-101…105**, без предложенного прежним отчётом hook.
Запрос — `WORKBUDDY_CONTINUE_06_03.md`.
Код ревью — `df47b146e`. Workspace Clippy/fmt — exit 0;
audit — 3105 objects / 49 BE / 232 source markers, provenance обновлён.

## G021 / W06-101…105 · сеанс 3

Полные детали — в [report-next-03.md](report-next-03.md). B-06-04 снят:
Java хранит `CustomName`/`lock` корневыми NBT-полями и не вызывает
`applyComponents` при reload; формат не менялся.

Реализовано: `BlockEntity::load_item_custom_data` (зеркало
`TypedEntityData.loadInto(BlockEntity)`) и вызов из `BlockItem.place` перед
`apply_components_from_item`. Раньше предмет с `minecraft:block_entity_data`
терял payload при установке.

Проверено: disk save без дублей (W06-101), borrowed/owned load с незнакомым
компонентом (W06-102), placement в настоящую BE (W06-103), pick → item →
block с удалением дублей и без воскрешения удалённого компонента (W06-104),
декодер `ChunkStorage` восстанавливает бочку с именем, lock и содержимым
(W06-105).

Проверки: workspace **4252 passed / 0 failed / 18 ignored** (exit 0), fmt и
workspace Clippy exit 0. Узкие: block_entity 119, chunk_saver 54,
block_interaction 2.

G022 не начата: builtin `/data` отсутствует, нужен отдельный срез командной
поверхности. Следующий ID: **G022 / W06-106…110**.
## Codex review G021 · 2026-11-02

Входящий диапазон `cea3e5115..0c6cefaf9` проверен по local Java 26.3.
G021 уточнена как partial: W06-103 без onlyOpCanSetNbt gate, W06-105 подтверждает
decoder/factory, а не region I/O/unload/restart. Права в Vecta уже существуют;
отсутствие permission system из входящего отчёта — неверное объяснение.

Исправлено: один saveCustomOnly + snapshot для merge вместо двух сохранений;
item snapshot до вызова load/component/world callbacks без player inventory lock.
Усилены type-mismatch placement, no-op merge и сохранность omitted identity/
stored explicit components. Формат контейнеров/B-06-04 не менялись.

Свежие проверки: block_item — 1 passed, item_block_entity_data — 2 passed,
workspace — **4252 passed / 0 failed / 18 ignored**, exit 0; Python — 4 tests OK.
Полный результат и commits — [review](../reviews/night-06-g021-codex.md).

Новый real blocker B-06-05: extractor task подготовлен, но Java/Gradle loopback
failure не дал produced metadata. Владелец прислал Java-17 mismatch из обычного
CMD; явный JDK 25 из tool environment сохранил IPC failure. Запрошен точный
CMD run с JAVA_HOME=25 и extractBlockEntityTypes. Не писать ручные flags/false.
Пока gate остановлен, G022 независима. Запрос — `WORKBUDDY_CONTINUE_06_04.md`.
Код ревью — `8b581a658`. Workspace Clippy/fmt — exit 0, audit —
3105 objects / 49 BE / 232 markers; provenance обновлён.

### Продолжение после ответа владельца: B-06-05 снят

Точный task на JDK 25 в обычном CMD владельца завершился BUILD SUCCESSFUL in 29s.
Получен output 26.3 / 49 типов, SHA-256
`bfe9f72f37d43fc9297ccf39e5f2bf0cecdf053679e48c130e74e5e01abfc88b`;
asset скопирован побайтно. Generator требует flag и version; runtime type получил
only_op_can_set_nbt. Player predicate использует creative/instabuild и существующий
published OP contract, без новой модели permissions. Gate применён к item custom
NBT и prefill HAS_BOOK кафедры. Узкие tests: 2 gate + 1 ordinary/type-mismatch,
exit 0. Полные результаты — в актуальном review G021. Не повторять старые
loopback attempts: extraction выполнена, B-06-05 больше не blocker.
Gate commit — `673c7a558`; итоговый workspace **4254 passed / 0 failed / 18 ignored**,
Clippy/fmt exit 0. Audit — 3105 objects / 49 BE / 232 markers, provenance обновлён
на gate commit. Следующий ID — G022; реальные I/O/context acceptance ещё открыты.

## G022 / W06-106…110 · сеанс 4

Полные детали — в [report-next-04.md](report-next-04.md).

Реализовано: foundation путей (`NbtPath::remove/insert/merge`, публичный
`is_too_deep`, `ExpectedList`/`ExpectedObject`/`InvalidIndex`), SNBT-аргументы
`compound_tag`/`nbt_tag` (протокол 21/22), builtin `/data` для `block` и
`storage` с `get`/`merge`/`remove`/`modify` (`insert`/`prepend`/`append`/`set`/
`merge` × `from`/`string`/`value`).

Живой путь: `BlockDataAccessor.setData`-эквивалент грузит через
`BlockEntityLoadContext::for_source`, затем `set_changed()` + `send_block_updated`;
ошибка восстанавливает снимок. `source_command_storage` теперь принимает
`&CommandSource` (общий с `/execute store`).

Проверено: lectern-книга резолвится через `/data` и не резолвится повторно
(W06-106/110), открытое меню бочки владеет живым контейнером и получает
обновление (W06-107), `set_changed` помечает чанк unsaved (W06-108 partial),
ошибочные payload не меняют состояние (W06-109).

Проверки: workspace **4273 passed / 0 failed / 18 ignored** (exit 0),
fmt exit 0, узкие: nbt::path 27, data graph/helpers 4, data_command 6.

Partial: `/data entity` (нет общего entity NBT load), `modify … compute`
(нет loot Context*Provider), comparator-половина W06-108 (нет контейнерного
redstone-сигнала), item custom-data context для кафедры (нет World→Server).
Следующий ID: **G023 / W06-111…115**.

## Codex review G022 · 2026-11-03

Входящий `7a7f68e41` проверен по локальной Java26.3. Исправлены wildcard result
counts, пропуск несовместимых array sources, обход неоднородных родителей,
depth check для merge source, float string conversion и UTF-16 substring без
замены unpaired surrogate. NBT path tests вынесены в отдельный модуль.
Убран фиктивный rollback: owned bridge может вернуть ошибку только до BE load.

B-06-08 снят как неверная диагностика существующего comparator capability;
добавлен `/data` → заполненная бочка → scheduler → output15 → remove → output0.
B-06-09 — задача передачи явного контекста: Player::server уже доступен,
World→Server pointer не нужен. Entity provider/compute и реальный region I/O
остаются незавершёнными.

Проверки и границы: [ревью](../reviews/night-06-g022-codex.md).
Codex workspace **4279 passed /0 failed /18 ignored**, utils228, paths31;
fmt/Clippy exit0, Python4 OK, audit3105 objects/49 BE/232 markers. При all-features
Cargo предупредил о mimalloc/dhat cfg; dependency сохранена, см. review.
Код исправлений — `cd11aa81a`.
Следующий ID: placement read context, затем G024/entity NBT и
[большая очередь](../../WORKBUDDY_CONTINUE_06_05.md) из12 направлений (>150 карточек).
G023 без карт не делать главным объёмом следующего сеанса.

## Сеанс 5: placement context, region I/O, `/data entity`

Полные детали — в [report-next-05.md](report-next-05.md).

1. **B-06-09 снят.** `BlockEntityLoadContext::for_server_world` + передача
   контекста из `BlockItem::place_with` через `player.server.upgrade()`.
   `load_item_custom_data` теперь `load_additional_with_context`. Книга кафедры
   разрешается при авторизованной установке и не переразрешается при reload.
2. **W06-105: настоящий region I/O.** Новый тест пишет `.mca` через
   `RegionManager` и читает обратно. Нашёл две реальные ошибки: сохранение
   писало `save_additional` без карты компонентов, а factory-загрузка читала
   `load_additional` вместо `load_with_components`. Обе исправлены по vanilla
   `LevelChunk`; detached-контекст сохраняет «диск не разрешает текст».
3. **B-06-06 снят для провайдера.** `Entity::apply_entity_nbt` с режимами
   Merge/Reset, `Entity::load_command_data`, провайдер `entity` в `/data`
   (get/merge/remove/modify, отказ игрокам, UUID не читается), `/execute store
   entity` на том же загрузчике.

Partial: `modify … compute` (нужны типизированные `ContextFloatProvider`/
`ContextIntProvider` + `LootContextSources`, ~30 реализаций — отдельный срез),
recipe-book award при взятии результата печи (нужен `ServerRecipeBook`).

Проверки: workspace **4285 passed / 0 failed / 18 ignored**, fmt и Clippy exit 0.

## Codex review сеанса05 · 2026-11-03

Исправлены player NBT bypass через execute store, множественный target вместо
одиночного, numeric vector/array и scalar conversions, velocity sync marker и
command invulnerable_time. Entity tests вынесены в отдельный файл, добавлены3
regression tests. Workspace **4288/0/18**, Clippy/fmt exit0; Cargo warning
mimalloc/dhat сохранён и объяснён в review. Python scripts4 OK.

B-06-06 возвращён в partial: регистрация provider не закрывает отсутствующий
общий living loader. Health/attributes/effects/timers/absorption и disk living
state требуют G024. Team API уже есть, отсутствие UX команд не внешний blocker.
Placement context и два region I/O fixes сохраняются; их не реализовывать заново.

[Ревью](../reviews/night-06-session05-codex.md),
[следующий большой запрос](../../WORKBUDDY_CONTINUE_06_06.md): закончить G024 и
entity accessor, затем compute и направления3–12 файла05. Итог — report-next-06.

## Сеанс 6: G024 living load/save, Team/UUID, разрыв предмета

Полные детали — в [report-next-06.md](report-next-06.md).

1. **G024 (`fc85be1ea`).** Новый `entity/living_data.rs` повторяет
   `LivingEntity.readAdditionalSaveData`/`addAdditionalSaveData`: absorption,
   packed attributes, active effects (свой декодер `MobEffectInstance.CODEC`),
   clamped health, hurt/death timers, FallFlying, sleeping, combat references,
   equipment, impulse context. Вызывается до subtype loader, поэтому порядок
   Living → Mob → Ageable → Animal → subtype сохранён. Запись чанка и
   registry-загрузка используют тот же payload, поэтому Health/effects/equipment
   теперь реально переживают `.mca`.
2. **Побочный дефект:** модификаторы атрибутов эффектов были transient, а Java
   применяет их `addPermanentModifier`; из-за этого они не попадали в
   сохраняемый `attributes` и эффект после reload не влиял на атрибуты. Исправлено.
3. **Team (`ec23d33c0`).** `Entity::load_team` через domain scoreboard,
   имя холдера — UUID из payload (как Java: UUID → Team → восстановление
   identity в accessor), `post_data_manipulated` вызывается после load.
4. **Разрыв предмета (`0d29632d8`).** `hurt_item_in_hand` возвращает сломанный
   `ItemRef`, `Player::hurt_item_in_hand` вызывает `on_equipped_item_broken`
   (entity event, refresh модификаторов, статистика).

Проверки: workspace **4293 passed / 0 failed / 18 ignored**, fmt exit 0.
`modify … compute` не начат: это отдельная подсистема (param-map LootContext,
2 реестра, 49 провайдеров), детали в отчёте и blockers.

## Сеанс 8: настоящие context number providers и потребитель-печь

Полные детали — в [report-next-08.md](report-next-08.md).

1. **Providers (`9190929d2`).** `loot_table/number`: `FloatProvider` (27 типов) и
   `IntProvider` (22 типа) с `ProviderRef`/`ConditionRef`/`WeightedList`, две
   точки входа Java (safe wrapper → 0 при ошибке, unsafe → проброс),
   арифметика по конкретным Java implementation (проверено JDK 25 oracle),
   configured values 4 float + 26 int, скомпилированные build script'ом из
   побайтовой копии extractor output, и три код-бутстрапнутых предиката Java
   для `conditional`/`number_dispatcher`.
2. **Печь (`9efa4655a`).** Burn time и speed multiplier теперь читаются из
   `minecraft:cooking_fuel` предмета и вычисляются против block state;
   `total_cook_time = ceil(рецепт / множитель)`, порядок тика и NBT приведены к
   26.3 (`speed_multiplier` сохраняется, таймеры int), `consume_fuel` выбрасывает
   remainder в мир.
3. **Границы:** `environment_attribute`, inline `enchantment_level` и
   inline-условия честно отвергаются; компостер без взаимодействия, поэтому
   `compostable/*` пока без потребителя; `compute` не начат (объём в отчёте).

## Дополнение сеанса 8: `/data modify … compute`

Реализован сквозной путь приоритета C из `WORKBUDDY_CONTINUE_06_07.md`:

1. **Protocol**: `context_float_provider` (55) и `context_int_provider` (56) в
   `c_commands.rs` по реальному порядку регистрации Java.
2. **Аргументы**: `parse_context_provider` — resource key / inline SNBT compound
   / голое число (`INLINE_CODEC`), ошибки `no_such_element` / `failed_to_parse`
   как в Java, unknown resource падает на parse.
3. **Дерево и context sources**: `<manipulator> compute` → `default` /
   `block <pos>` / `entity <target>` → `float|integer <provider>`;
   `ORIGIN`/`THIS_ENTITY`/`BLOCK_STATE`/`BLOCK_ENTITY`/`TARGET_ENTITY` с
   валидацией `COMMAND_COMPUTE_*` через настоящий `ContextMap`; оценка внутри
   mutable level RNG и live domain reader.
4. **Запись**: результат идёт в существующий manipulator через общий
   `apply_source` — single-target guard, запрет players, rollback сохранены.

Тесты: constants (float/int/inline compound), configured resource на smoker
против furnace, вложенная арифметика, score provider (fallback 42 → живая 5),
и четыре негативных пути без мутации. Workspace **4326/0/18**, Clippy/fmt exit0.

## Сеанс 9: компостер как потребитель compostable/*

Полные детали — в [report-next-09.md](report-next-09.md).

1. **WorldlyContainerHolder (`behaviors` foundation).** Новый трейт
   `behavior/worldly_container_holder.rs` (`get_container(state, world, pos)`)
   и `BlockBehavior::as_worldly_container_holder` (по образцу
   `as_bonemealable`). `HopperBlockEntity::container_at` теперь спрашивает
   держателя **до** блок-энтити и не проваливается к ним (порядок
   `HopperBlockEntity.getBlockContainer`), поэтому воронка, выбрасыватель и
   далее crafter работают через один общий путь без компостерных особых
   случаев. Закрыт пункт ревью сеанса 8 «Воронка не видит
   WorldlyContainerHolder».
2. **Компостер (`composter_block.rs`).** Полный `ComposterBlock` 26.3:
   `useItemOn` (`fillLevel < 8` + `COMPOSTABLE`; при `fillLevel < 7` —
   `addLayer`, level event 1500 с data `state != newState`, stat ITEM_USED,
   `consume(1)`; иначе `TRY_WITH_EMPTY_HAND` как `super.useItemOn`),
   `useWithoutItem` (level 8 → `extractProduce`: bone meal item entity в
   `pos+(0.5,1.01,0.5)` с `offsetRandomXZ(0.7)` и скоростью из level random,
   `empty`, звук `BLOCK_COMPOSTER_EMPTY`), `addLayer` (BLOCK_INTERACT контекст:
   BLOCK_STATE/ORIGIN/опц. INTERACTING_ENTITY, слои из
   `context_int_providers.by_key(compostable.layers)`, clamp 0..7, game event
   BLOCK_CHANGE, scheduleTick(20) при уровне 7), `empty`, `tick` (7→8 + звук
   READY), `onPlace` (уровень 7 → scheduleTick), comparator, pathfinding.
   Шанс — только configured providers `compostable/*` (always_add_one, low,
   low_medium, medium, medium_high), никакого ручного списка.
3. **Виртуальные контейнеры.** `InputContainer` (1 слот, stack 1, только UP,
   вставка только compostable и до первого изменения; `set_changed` →
   `addLayer(null, …)` + level event + `removeItemNoUpdate`),
   `OutputContainer` (bone meal, только DOWN, `set_changed` → `empty`),
   `EmptyContainer` (0 слотов на уровне 7 — контейнер есть, поэтому дроппер
   перед ним оставляет предмет, а не выбрасывает). Компостер без блок-энтити,
   как в ванили.
4. **Проверки.** 13 компостерных тестов: гарантированный слой на уровне 0,
   event data инвариант при взвешенном шансе (и что предмет расходуется
   всегда), extraction (item entity + звук + уровень 0), tick 7→8 со звуком,
   onPlace schedule, компаратор, воронка сверху (1 предмет за трансфер),
   воронка снизу (bone meal + пустой компостер), дроппер сверху кормит,
   дроппер перед уровнем 7 сохраняет предмет без выброса, клик на уровне 7
   ничего не меняет. Звуки/level events проверяются по реально отправленным
   пакетам. Workspace **4341 passed / 0 failed / 18 ignored** (было 4329),
   fmt/Clippy exit 0, Python 4 OK, audit 3105/49/227.

## Сеанс 10 (2026-11-07): Farmland W06-536…538 + W06-540

Старт: HEAD `62694c7f3` (ревью компостера), 4342/0/18. Сверены целиком
`FarmlandBlock.java` и `CropBlock.java` (26.3) с текущей реализацией.

1. **Farmland (`farmland_block.rs`, переписан).** Все методы ванили:
   `getStateForPlacement` (не выживает → базовый блок **dirt**, иначе
   default moisture 0), `updateShape` (сосед сверху при !canSurvive →
   scheduleTick(1)), `canSurvive` (`!above.isSolid() || MAINTAINS_FARMLAND`),
   `tick` (!canSurvive → dirt), `randomTick` (влага: рядом вода или дождь
   `isRainingAt(above)` → 7; иначе −1; на 0 без crop → dirt),
   `fallOn` (trampling), `turnToBaseBlock` (общий `turn_to_dirt`:
   pushEntitiesUp + game event), `isPathfindable=false`. `isNearWater` —
   точно ваниль: fluid state (источник/поток/waterlogged) в боксе
   (-4,0,-4)..(4,1,4); прежний хардкод «WATER или waterlogged» заменён на
   `get_fluid_state().is_water()`. `shouldMaintainFarmland` — тег
   `MAINTAINS_FARMLAND` вместо ручного списка (тег шире: fence gates,
   torchflower, moving_piston).
2. **RNG-поток.** `fallOn` теперь берёт бросок из **level random**
   (`world.random().with_level(Random::next_f32)`) на каждом приземлении до
   любых условий — раньше был thread `rand::random`. У crop growth то же
   исправление: `on_random_tick` теперь
   `nextInt((int)(25.0F/growthSpeed)+1)==0` на level random вместо
   `rand::random::<u32>().is_multiple_of`.
   Тесты trampling детерминированы краями ванильного условия:
   fallDistance=0.5 → бросок невозможен (никогда), 100 → всегда.
3. **W06-538 мотыга — проверена, не дублирована.** В 26.3 `HoeItem` —
   data-driven `BlockTransformers.HOE`; extracted data в Vecta совпадает
   с Java 1:1 (all_of(turns_into_farmland, air@offset(0,1,0))→farmland m0;
   coarse_dirt+air→dirt; rooted_dirt→dirt+loot till/rooted_dirt,
   clicked_face; disallowed face DOWN; звук ITEM_HOE_TILL). Закреплено
   тестами через реальный `use_on`: dirt→farmland (и запрет при блоке
   сверху → Pass), coarse_dirt→dirt, grass_block→farmland;
   rooted dirt уже был покрыт.
4. **W06-540 жизненный цикл.** Рост crop: формула скорости
   (`crop_growth_speed`: GROWS_CROPS, влага>0→3.0, /4 соседи, /2 скученность)
   сверена с Java и закреплена детерминированным тестом; реальный
   random_tick растит wheat на гидратированной грядке через level random.
   Trampling ломает crop: тест «жёсткое падение → farmland→dirt, wheat
   сверху исчезает» (update_shape куста). Высыхание без воды, удержание
   влаги-0 под crop, revert в dirt по tick под перекрытием — все на реальных
   путях.
5. **Проверки.** Farmland-тестов 14 (+3 старых pure), crop-тестов +3,
   transformer +2. Workspace **4357 passed / 0 failed / 18 ignored**
   (было 4342), fmt exit0, Clippy `-D warnings` exit0. Найдено попутно и
   записано в blockers: bonemeal на thread RNG, spawn-protection не
   смоделирована, порядок-зависимый copper chest transformer тест.

## Сеанс 11 (2026-11-09): блок A — хвосты (A1, A2, A3)

Старт: HEAD `69ca44bac`, baseline 4357/0/18. Режим: марафон по
WORKBUDDY_CONTINUE_06_10.md.

1. **A1 — Farmland принят (`bd82a24d9`, push).** Повторно сверены все методы
   `FarmlandBlock.java` с реализацией; 17/17 тестов. Найдена и закрыта
   микро-дивергенция: ваниль считает `getBbWidth()²·getBbHeight() > 0.512F`
   во float — хелпер переведён на f32 (`bounding_box_width_squared_height`),
   порог 0.512f32. W06-536…538, W06-540 закрыты. audit 3105/49/227, python OK.
2. **A2 — inline-формы Resolvable.** Новый общий модуль
   `data_components/components/resolvable.rs`: `ResolvableInt` /
   `ResolvableFloat` (Constant/Reference) точно по Java `ResolvableInt` /
   `ResolvableFloat` 26.3: wire = `ByteBufCodecs.either` (bool true →
   Constant, потом BE int `readInt` / BE float `readFloat`; false → key),
   NBT = число или строковый ключ, hash = хэш выбранной ветки.
   `Compostable`, `CookingFuel`, `BrewingFuel` переведены; конструкторы
   `new(Identifier…)` сохранены (generated данные не тронуты), добавлены
   `constant_layers`/`constant`. Компостер: Constant возвращает слои без
   построения LootContext — не тратит вызов level random (как ваниль
   `Constant.get`); Reference без провайдера → 0 (vanilla `.orElse(0)`);
   печь аналогично с дефолтами 0/1.0. Тесты: 6 в resolvable, +2
   compostable, +2 cooking_fuel, +2 brewing_fuel, +1 компостер (inline
   Constant кладёт ровно 3 слоя).
3. **A3 — контекст печи и /data compute.** `FurnaceProviders` теперь несёт
   identity блочной сущности и передаёт `BLOCK_ENTITY` в контекст топлива
   (ваниль `BaseContainerBlockEntity.getLootContext`, параметр-сет
   `CONTAINER_PROCESS`); тест «контекст топлива несёт identity печи».
   `CONTAINER` остался — metadata-only, см. blockers. `BlockEntity` получил
   `custom_name()` (7 переопределений: barrel, beacon, chest, dispenser,
   dropper, hopper, shulker_box) и `container_items()` (снимок через
   container_ref, lock стал pub(crate)); `/data compute` (block) теперь
   заполняет в `BlockEntityRef` и имя, и инвентарь.
4. **Проверки:** fmt exit 0; workspace см. отчёт; clippy `-D warnings` exit 0.
   Во время сеанса рабочий файл abstract_furnace.rs был случайно обнулён
   python-скриптом — восстановлен из HEAD и правки наложены заново (проверено
   сборкой); прог workspace, шедший в этот момент, признан недействительным
   и перезапущен.

### Сеанс 11 — продолжение: B1/G031 (2026-11-09)

**G031 — RecipeDisplay wire-типы + извлечённый реестр** (коммит
`e26c19aa9`, push):

- Java-сверка всех кодеков по исходникам 26.3: `RecipeDisplay.STREAM_CODEC`
  (registry dispatch `minecraft:recipe_display`), все 5 вариантов
  (Shapeless/Shaped/Furnace/Stonecutter/Smithing) поле-в-поле,
  `RecipeDisplayEntry.STREAM_CODEC` (id, display, OPTIONAL_VAR_INT group,
  registry-категория, optional(list(Ingredient.CONTENTS_STREAM_CODEC))),
  `ByteBufCodecs.list()` = VarInt-счётчик с bound `Integer.MAX_VALUE`,
  `OPTIONAL_VAR_INT` (0=absent, v+1).
- `recipe_displays.json` извлечён штатным путём: новый
  `RecipeDisplayRegistry.kt` + gradle-задача `extractRecipeDisplays`
  (потребовался gradle 9.7.0 — дистрибутив скачан, sha256 сверен с
  официальным; JAVA_HOME переключён на Temurin 25, fabric-loom требует
  JVM 25). Bootstrap-порядок 0..4 подтверждён тестом.
- Rust: `recipe_display.rs` + `types.rs` по образцу slot displays
  (`impl_standard_methods!` + `impl_registry!`), codegen
  `build/recipe_displays.rs` + wiring в build.rs/lib.rs/registry/mod.rs.
- Урок теста: kind→type диспетчеризация у RecipeDisplay всегда ведёт к
  глобальным статикам (как ваниль) — локально зарегистрированный тип
  не используется `RecipeDisplay::write`; тест переписан на реальные
  семантики. Кастомные display-типы не поддерживаются (blocker 5).
- Проверки: fmt OK; narrow 8 passed; workspace 4378/0/18; clippy
  `-D warnings` exit 0; audit 3105/49/227; python 4 OK.

### Сеанс 11 — продолжение: W06-153 (2026-11-09)

**G031/W06-153 — RecipeManager↔display linkage** (коммит `7bab2d0ba`,
push): DisplayIndex::build по образцу RecipeManager.unpackRecipeInfo —
порядок регистрации, последовательные RecipeDisplayId, group id по первому
вхождению строки группы (Object2IntMap semantics), special→crafting
requirements None, иначе placement ingredients → holder sets; display()-
паритет всех сериализаторов с display() (shaped, shapeless, transmute с
пересчётом результата по количеству материала, dye через
OnlyWithComponent(DYE)+DyedSlotDemo, imbue 3×3 с WithAnyPotion, cooking с
furnaceIcon по типу, stonecutter только для книги, smithing transform/trim
со SmithingTrimDemoSlotDisplay); RecipeBookCategoryKind::resolve по
ванильным переключателям; Ingredient::display (TagSlotDisplay) +
holder_set. Java-сверка: Recipe.java display() default пуст, RecipeManager
unpackRecipeInfo/getRecipeFromDisplay/listDisplaysForRecipe,
SmithingTrimDemoSlotDisplay(base, material, Holder<TrimPattern>),
DyedSlotDemo(dye, target). 6 новых тестов в registry.rs. Проверки: fmt OK,
workspace 4384/0/18, clippy -D warnings exit 0, audit/python OK.
W06-155 (stonecutter) подтверждён: synchronized_stonecutter_recipes и
initial CUpdateRecipes не тронуты — DisplayIndex отдельный builder.

### Сеанс 11 — продолжение: G032 (2026-11-09)

**G032 — ServerRecipeBook** (коммит `63f06b151`, push): новый
`vecta-registry/src/recipe/recipe_book.rs` (+2 в mod.rs). RecipeBookType
(Crafting/Furnace/BlastFurnace/Smoker), TypeSettings{open,filtering},
RecipeBookSettings (get/set_open/set_filtering/set_book_setting/
replace_from; все const где возможно) + WriteTo/ReadFrom — 8 bool в
порядке типов (vanilla RecipeBookSettings.STREAM_CODEC). Packed (vanilla
ServerRecipeBook.Packed): settings/known/toBeDisplayed — ToNbtTag/
FromNbtTag с точными ванильными ключами isGuiOpen/isFilteringCraftable/
isFurnaceGuiOpen/isFurnaceFilteringCraftable/isBlastingFurnaceGuiOpen/
isBlastingFurnaceFilteringCraftable/isSmokerGuiOpen/
isSmokerFilteringCraftable (optional bool, default false) и списками
ключей рецептов; absence → пустой список, битые записи → None.
ServerRecipeBook: known/highlight как FxHashSet<Identifier>,
add/contains/remove/remove_highlight/is_highlighted, award_recipes
(vanilla addRecipes state: скип без properties и special, новые получают
highlight), forget_recipes (возвращает известные, снимает highlight),
pack/apply/load_untrusted (валидатор ключей, неизвестные — tracing::error
как vanilla loadRecipes). 6 тестов: дефолты настроек, wire 8-bool
round trip с точными байтами [1,0,0,1,0,0,0,1], NBT имена полей,
award/forget tracking, load_untrusted с отбрасыванием неизвестных,
pack→apply и полный pack→NBT→Packed→load_untrusted round trip.
Нюанс simdnbt: borrow NbtList::strings() = None для пустого списка
(End-kind) — чтение ключей через owned as_nbt_tags() (Empty → vec![]),
декод строки через .parse::<Identifier>(); тесты не должны писать
голый NbtCompound (write без type-байта) — только NbtTag::Compound.
Проверки: fmt OK; narrow 6 passed; workspace 4390/0; clippy
`-D warnings` exit 0 (после const-fn + is_some_and правок); audit
3105/49/227; python 4 OK.

### Сеанс 11 — продолжение: G033 протокол (2026-11-09)

**G033.1 — recipe-book packets** (коммит `e3f5c55fe`, push): 5 новых
пакетов в vecta-protocol по Java 26.3:
- `c_recipe_book_add.rs` — CRecipeBookAdd{entries,replace};
  Entry{contents: RecipeDisplayEntry, flags: u8} с
  FLAG_NOTIFICATION=1/FLAG_HIGHLIGHT=2, конструктором new(notification,
  highlight) и accessor-ами (const). Wire: VarInt-список entry +
  bool replace. Тест строит реальный entry через DisplayIndex по BAKED_POTATO.
- `c_recipe_book_remove.rs` — CRecipeBookRemove{Vec<RecipeDisplayId>} (derive).
- `c_recipe_book_settings.rs` — CRecipeBookSettings{RecipeBookSettings} (8 bool).
- `s_recipe_book_change_settings.rs` — SRecipeBookChangeSettings{book_type,
  is_open, is_filtering}: writeEnum=VarInt ordinal (0..3), орфан-рул —
  ordinal-хелперы свободными функциями; reject ordinal >3 (InvalidData).
- `s_recipe_book_seen_recipe.rs` — SRecipeBookSeenRecipe{RecipeDisplayId}
  (VarInt).
Java-сверка: ClientboundRecipeBookAddPacket (+Entry), RemovePacket,
SettingsPacket, ServerboundRecipeBookChangeSettingsPacket,
ServerboundRecipeBookSeenRecipePacket, GamePacketTypes id-шники уже в
generated vanilla_packets (C 75/76/77, S 47/48). 9 тестов. Проверки:
fmt OK; narrow 9 passed; workspace 4399/0; clippy exit 0; audit
3105/49/227; python 4 OK.

Следующий маршрут: G033.2 — vecta-core интеграция: хранение книги на
игроке, sendInitialRecipeBook в spawn_sync (после CUpdateRecipes, vanilla
PlayerList.placeNewPlayer), serverbound handlers (change settings →
set_book_setting; seen recipe → display lookup → remove_highlight).

### Сеанс 11 — продолжение: G033.2 core (2026-11-09)

**G033.2 — recipe book в vecta-core** (коммит `de0ae9370`, push):
- `vecta-registry` ServerRecipeBook: +`known_keys()` (итерация known для
  sendInitialRecipeBook).
- Новый `vecta-core/src/player/recipe_book.rs`: Player::recipe_book
  (SyncMutex<ServerRecipeBook>, поле + init в new); process-wide
  `display_index()` (LazyLock<DisplayIndex::build(&REGISTRY.recipes)> —
  аналог ванильного DisplayResolver);
  `send_initial_recipe_book` (CRecipeBookSettings затем CRecipeBookAdd
  replace=true, entry флаги highlight из книги, notification=false —
  vanilla ServerRecipeBook.sendInitialRecipeBook);
  `handle_recipe_book_change_settings` → set_book_setting;
  `handle_recipe_book_seen_recipe` → get_recipe_from_display →
  remove_highlight(parent.key()).
- `connection/java.rs`: ScheduledPlayPacketKind::{RecipeBookChangeSettings,
  RecipeBookSeenRecipe}, decode по S_RECIPE_BOOK_CHANGE_SETTINGS(47)/
  S_RECIPE_BOOK_SEEN_RECIPE(48), исполнение PlayerLocal (как ClientCommand —
  только состояние игрока), dispatch на хендлеры.
- `lifecycle/spawn_sync.rs`: send_initial_recipe_book после CUpdateRecipes
  под тем же `if send_recipes` (vanilla PlayerList.placeNewPlayer).
- 3 теста с RecordingConnection: initial sync шлёт ровно settings(8 нулей)
  + add(count=1, replace=1) по реальному BAKED_POTATO; seen recipe снимает
  highlight, оставляя known; change settings пишет tab state.
W06-154 (пакетный wire consumer) закрыт полностью: wire → decode →
dispatch → state. Проверки: fmt OK; narrow 3 passed; workspace 4402/0;
clippy exit 0; audit 3105/49/227; python 4 OK.

Остаток G033/G034+: персистенция книги в playerdata (PlayerDataFile
строгой версии, отдельный срез), award/forget senders (критерии разбора —
advancements/commands), G034 place recipe.
