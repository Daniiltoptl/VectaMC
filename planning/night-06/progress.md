# Night 06 — progress

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
