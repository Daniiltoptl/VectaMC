# Night 06 — progress

## Сеанс7: ContextMap и все текущие loot consumers

Реализована единая typed ContextMap с сохранением borrowed values и shared RNG.
Удалены отдельные параметры LootContext и LootContextRef. Generator использует
produced SteelExtractor output:16ключей/31param set. Текущие игровые consumers
переведены и проверяют свои sets; исправлены chest THIS_ENTITY, условие
KilledByPlayer по LAST_DAMAGE_PLAYER и block origin/empty tool.

Код `2a0ef1daf`: workspace4304/0/18, Clippy/fmt exit0, Python4,
audit3105/49/230. Детали: [report-next-07.md](report-next-07.md).
Score/Storage legacy stubs и SlotProvider consumer не объявляются готовыми.
Общий read context домена с shared Arc scoreboard/storage **одобрен
пользователем**; это следующий срез, повторно разрешение не спрашивать.
Provider type output уже
получен:28float+23int=51, а не49; новая factory/compute ещё не реализована.

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
