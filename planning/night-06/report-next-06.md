# Сеанс 06: живая загрузка G024, Team/UUID, разрыв предмета

Вход `65716df85` (ветка `codex/night-vanilla-03`), код ревью `1a8bb2457`.
Полностью прочитаны `AGENTS.md`, `planning/reviews/night-06-session05-codex.md`,
`WORKBUDDY_CONTINUE_06_06.md` и `WORKBUDDY_CONTINUE_06_05.md`.

## Что сделано

### 1. G024 — общий living load/save и настоящий disk round-trip (`fc85be1ea`)

Раньше `/data merge entity {Health:5.0f}` менял только compound: `load_command_data`
вызывал base reader и subtype loader, а `Sheep.load_additional` → `load_mob`
никогда не читал Health, attributes или effects. Запись чанка имела тот же
пробел, поэтому повреждённый моб после перезагрузки лечился до полного здоровья.

Новый `vecta-core/src/entity/living_data.rs` повторяет
`LivingEntity.readAdditionalSaveData` / `addAdditionalSaveData` поле за полем и
вызывается до subtype loader, поэтому порядок совпадает с
Living → Mob → Ageable → Animal → subtype.

Матрица реализации (Java 26.3 → Vecta → экспорт → load → disk):

| Поле Java | Vecta state | save | load | disk |
|---|---|---|---|---|
| `AbsorptionAmount` | `LivingEntityState.absorption_amount` | да | `internal_set_absorption_amount` (без клампа, как `internalSetAbsorptionAmount`) | да |
| `attributes` | `AttributeMap` | `to_vanilla_nbt` | `AttributeMap.apply`-семантика: base + modifiers по id, transient-модификаторы payload-а не стираются | да |
| `active_effects` | `active_mob_effects` | отсортировано по id | новый декодер `MobEffectInstance.CODEC`, прямой clear+fill | да |
| `Health` | synced data | да | после attributes, `set_health` клампит по max health | да |
| `HurtTime` / `DeathTime` | `LivingEntityState` | short | новые raw-сеттеры | да |
| `FallFlying` | `fall_flying` | да | `set_fall_flying` | да |
| `sleeping_pos` | `sleeping_pos` + pose | int array | `set_sleeping_pos` + pose Sleeping; `start_sleeping` только если `!first_tick` (как `!this.firstTick`), иначе `stop_sleeping` | да |
| `last_hurt_by_player` + memory time | uuid + memory | int array | `set_last_hurt_by_player` | да |
| `last_hurt_by_mob` + ticks since | weak entity + **новое** `last_hurt_by_mob_uuid` | int array + ticks | UUID-only ссылка, ленивое разрешение через world directory | да |
| `equipment` | `EntityEquipment` | да | один владелец: отсутствие compound очищает слоты на command/disk, spawn data не трогает | да |
| `current_impulse_context_reset_grace_time` / `current_explosion_impact_pos` | `LivingEntityState` | да | `restore_impulse_context` (raw, не gameplay-helper) | да |
| `Brain`, `locator_bar_icon` | — | нет | нет | нет |

`Brain` и `locator_bar_icon` не читаются и не пишутся: в Vecta нет ни brain
persistence, ни waypoint-иконки, и фиктивное поле скрыло бы это.

Найденный по пути реальный дефект: vanilla применяет модификаторы атрибутов
эффектов через `addPermanentModifier` (`MobEffect.addAttributeModifiers`), а
Vecta добавляла их как transient. Из-за этого они не попадали в сохраняемый
список `attributes`, и после перезагрузки эффект Speed переставал влиять на
скорость. Теперь они persistent, как в Java.

### 2. Team, UUID и accessor contract (`ec23d33c0`)

`Entity.load` в Java читает `UUID` (строка 2211), затем `Team` (2232) через
`level.getScoreboard()`, добавляет участника под **загруженным** UUID, и только
потом `EntityDataAccessor.setData` восстанавливает исходный UUID. `Team` нет в
`saveWithoutId`, поэтому поле имеет смысл только для NBT из команд/датапаков.

- `Entity::load_team` берёт domain scoreboard (`domain_scoreboard(source)`),
  повторяет vanilla-предупреждение «team does not exist» вместо ошибки и не
  снимает прежнее членство.
- Имя холдера — UUID из payload, поэтому payload с `UUID`+`Team` воспроизводит
  vanilla-поведение (членство на payload-UUID), а живая сущность сохраняет
  identity: world UUID index не переписывается.
- `Entity::post_data_manipulated` вызывается после load. В 26.3 его
  переопределяет только `Villager`; фиктивного Villager не добавлено.

### 3. Разрыв предмета (`0d29632d8`)

`PlayerInventory::hurt_item_in_hand` выбрасывал результат `hurt_and_break`,
поэтому пути стрижки (снаряжение, поводок, овца, муушрум) не вызывали
`onEquippedItemBroken`: не было entity event, очистки модификаторов атрибутов и
статистики `minecraft:item_broken`.

Теперь метод возвращает сломанный `ItemRef`, а `Player::hurt_item_in_hand`
оборачивает его `on_equipped_item_broken` для слота руки.

## Проверки

| Команда | Результат |
|---|---|
| `cargo test --locked --workspace -j 1` | **4293 passed / 0 failed / 18 ignored**, exit 0 (вход 4288) |
| `cargo fmt --all --check` | exit 0 |
| `cargo clippy --locked --workspace --all-targets --all-features -j 1 -- -D warnings` | exit 0 (1m34s; без новых suppressions) |
| `python -m unittest discover -s scripts -p 'test_*.py'` | 4 tests OK |
| `python scripts/audit_vanilla.py` | 3105 objects / 49 BE / 231 markers, provenance обновлён |
| узкие: `data_entity` 9, `player::tests::a_broken_held_item` 1, `a_living_entity_keeps_health…` 1 | зелёные |
| `vecta-core --lib` | 3359 passed / 0 failed / 1 ignored |

Новые тесты:
- `data_merge_writes_living_health_attributes_and_timers` — Health/HurtTime/
  Absorption на живой овце, кламп по загруженному max-health, reset через
  `remove`.
- `data_merge_writes_active_effects_and_equipment` — эффект с duration/amplifier/
  `show_particles:0b`, очистка `active_effects`, экипировка и её очистка.
- `data_merge_team_uses_the_domain_scoreboard_and_the_loaded_uuid` — существующая
  команда, отсутствующая команда, payload UUID+Team.
- `a_living_entity_keeps_health_effects_and_equipment_across_a_region_file` —
  настоящий `.mca`: здоровье 5.0, эффект, меч, UUID игрока после чтения.
- `a_broken_held_item_awards_the_break_stat_and_clears_the_hand`.

## Незавершённое

### `modify … compute` — не начато, это отдельная подсистема

Проверено по Java 26.3:

- `DataCommands` compute-ветка использует `ResourceOrIdArgument.floatProvider` /
  `intProvider` и `LootContextSources.addContextSources`.
- `ContextFloatProvider`/`ContextIntProvider` — **реестры**
  (`Registries.CONTEXT_FLOAT_PROVIDER`) с типовым реестром
  (`CONTEXT_FLOAT_PROVIDER_TYPE.byNameCodec().dispatch`), плюс inline-ветка
  `ConstantValue.INLINE_CODEC`. Данные бутстрапятся кодом
  (`ContextFloatProviders.bootstrap`: `cooking/speed_default`,
  `cooking/normal_speed_multiplier`, `cooking/fast_speed_multiplier`,
  `brewing/speed_default`).
- Реализаций: 27 float (`Absolute, Average, Ceiling, ConditionalValue,
  ConstantValue, Cosine, Difference, EnchantmentLevelProvider,
  EnvironmentAttributeValue, Floor, FromInt, Length, Maximum, Minimum, Modulus,
  Negate, Power, Product, Quotient, Round, Sine, SquareRoot, StorageValue, Sum,
  Truncate, UniformGenerator, WeightedListValue`) и 22 int (плюс
  `BinomialDistributionGenerator`, `FloorModulus`, `FloorQuotient`, `FromFloat`,
  `ScoreboardValue`), у каждого свой codec и правила вычисления.
- `LootContextSources` строит `LootParams` с
  `LootContextParamSets.COMMAND_COMPUTE_DEFAULT/POSITION/ENTITY` и параметрами
  `THIS_ENTITY`, `ORIGIN`, `BLOCK_STATE`, `BLOCK_ENTITY`, `TARGET_ENTITY`.

В Vecta `LootContext` (`vecta-registry/src/loot_table/context.rs`) — плоская
borrow-структура (`rng`, `tool`, `origin`, `this_entity`, …) без param map и
param sets. Реализация compute на ней означала бы второй контекст-модель рядом с
существующей, то есть ровно ту частичную foundation, которую запрещает AGENTS.

Нужен отдельный срез: param-map `LootContext`/`LootParams` + param sets, два
реестра провайдеров с типовой диспетчеризацией (новые extracted данные), 49
реализаций, `ResourceOrIdArgument`, и только затем `compute float|integer`.
Оценка: это не «половина узла команды», а полноценная подсистема loot context.

### Прочее

- `ITEM_DURABILITY_CHANGED` criteria в `ItemStack::hurt_and_break` остаётся TODO:
  advancement-criteria foundation в Vecta отсутствует.
- `onEquippedItemBroken` для мобов (`LivingEntity::on_equipped_item_broken`)
  пока только обновляет модификаторы атрибутов; entity event для мобов не
  проверялся отдельным тестом.
- Dispenser: `get_dispense_method` не имеет ветки `SpawnEggItemBehavior`
  (нет spawn egg с `ENTITY_DATA`) и `SulfurCubeBlockDispenseItemBehavior`
  (уже открытый blocker по SulfurCube). `NOOP`-поведение пишет предмет обратно
  без изменений — наблюдаемой разницы с Java нет.

## Следующее действие

1. `modify … compute` отдельным срезом: сначала `LootContext` с param map и
   `LootContextParamSets.COMMAND_COMPUTE_*`, затем реестры провайдеров
   (типовая диспетчеризация + extracted данные), затем `compute`.
2. Затем направления 3–12 файла 05: раздатчик/экипировка (NBT уже полный,
   остались equip-capability и seeded-выбор слота), печи, recipe book,
   растения, облака зелий, стрелы, фейерверки, пчёлы, карты, автоматизация.

## Ограничения проверок

Клиент 26.3, Linux, Mojang differential, два сетевых игрока и живой
restart/unload сервера **не запускались**. Region-тесты используют настоящий
`.mca` и реальный storage API, но не поднимают второй сервер и не гоняют
ticket-машину. EULA не принималась.

Чужие файлы не тронуты:
`planning/night-01/report.md` (`06680CBC…`), `WORKBUDDY_START_06.md`
(`48430DC1…`).
