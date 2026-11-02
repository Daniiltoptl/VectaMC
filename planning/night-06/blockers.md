# Night 06 — blockers

Проверенные блокеры. Формат: что проверено → чего не хватает → риск догадки →
точный path/output/decision → зависимые карточки.

## B-06-01 · G011/W06-052 — у саженцев нет bonemeal-роста

**Проверено:** `vecta-core/src/behavior/blocks/vegetation/sapling_block.rs`
реализует только `BlockBehavior` (`can_survive`, `get_state_for_placement`) и не
имеет `Bonemealable`, поэтому `BoneMealItem.growCrop` возвращает `false` для
саженца. Машинерия фичи уже есть и работает:
`MangrovePropaguleBlock::grow_tree` вызывает
`FeatureDecorationRunner::place_tree_feature` с
`vanilla_configured_features`. Все нужные configured features присутствуют в
`vecta-registry/src/generated/vanilla_configured_features.rs`: `OAK`,
`FANCY_OAK`, `OAK_BEES_005`, `FANCY_OAK_BEES_005`, `SPRUCE`, `MEGA_SPRUCE`,
`MEGA_PINE`, `BIRCH`, `BIRCH_BEES_005`, `JUNGLE_TREE_NO_VINE`,
`MEGA_JUNGLE_TREE`, `ACACIA`, `DARK_OAK`, `CHERRY`, `CHERRY_BEES_005`,
`PALE_OAK_BONEMEAL`, `AZALEA_TREE`.

**Чего не хватает:** аналога vanilla `TreeGrower`
(`minecraft-src/minecraft/src/net/minecraft/world/level/block/grower/TreeGrower.java`):
взвешенные списки `trees`/`megaTrees`/`flowerTrees` на саженец, выбор
`shortestTreeType`, проверка цветов вокруг (`hasFlowers`), поиск 2×2 саженцев
для mega-деревьев, удаление/восстановление саженцев при неудачном размещении.
Плюс `SaplingBlock: Bonemealable` с ванильными `isBonemealSuccess`
(`random < 0.45`) и `performBonemeal` (STAGE 0→1, затем рост).

**Риск догадки:** частичная реализация только 1×1-деревьев даст неверный набор
деревьев (у dark oak/pale oak `trees` пуст и рост идёт только через mega), а
также потеряет FANCY_OAK и bees-варианты. Это shared worldgen foundation, на
которую опирается `SaplingBlock`, поэтому частичная версия не заводилась.

**Решение:** реализовать `TreeGrower`-эквивалент одним срезом вместе с
`SaplingBlock: Bonemealable` и тестом на настоящий `OAK_LOG` из фичи.
До этого W06-052 — partial. Зависимые карточки: W06-052.

## B-06-02 · G010/W06-046 — fire charge из раздатчика

**Проверено:** vanilla регистрирует `Items.FIRE_CHARGE` через
`DispenserBlock.registerProjectileBehavior`, то есть запускает `SmallFireball`
(`DispenseItemBehavior.bootStrap`). В Vecta нет `SmallFireball` entity, поэтому
`FIRE_CHARGE` намеренно не зарегистрирован: иначе пришлось бы подменить полёт
прямым поджиганием блока.

**Чего не хватает:** `SmallFireball` entity + регистрация (G047).

**Решение:** оставить `FIRE_CHARGE` вне `DISPENSE_BEHAVIORS` (тест
`a_fire_charge_stays_unregistered_until_the_small_fireball_exists` это
фиксирует) и реализовать вместе с G047. Зависимые карточки: W06-046 (часть).

## B-06-03 · G013/W06-062 — соты из улья

**Проверено:** `BeehiveBlock` (`vecta-core/src/behavior/blocks/container/beehive_block.rs`)
имеет свойство `LEVEL_HONEY`, но не имеет `dropHoneycomb`. В Vecta нет
`Bee` entity, поэтому `releaseBeesAndResetHoneyLevel` не может выпустить
пчёл, и нет пути «лут блока с инструментом» для трёх сот.

**Чего не хватает:** `BeehiveBlock.dropHoneycomb` с interaction loot table
`BuiltInLootTables.HARVEST_BEEHIVE` и `Bee` entity для `BeeReleaseStatus::BEE_RELEASED`.
Codex сверил Java: это не block-break loot `blocks/beehive`.
В Vecta уже есть typed `vanilla_loot_tables::HARVEST_BEEHIVE` и
`drop_from_block_interact_loot_table`; проверить полноту его context с consumer.
Отсутствие Rust метода само по себе не является внешним blocker.

**Решение:** `ShearsDispenseItemBehavior` реализует только entity-ветку
(настоящий `Shearable`), улей пока не стрижётся. Зависимые карточки: W06-062.

## B-06-04 · снят после ревью — неверная трактовка persistence

Прежняя запись на `4afc44fd7` фактически неверна и не является планом изменения кода.
Codex повторно прочитал local Java 26.3:

- `BaseContainerBlockEntity.loadAdditional` читает `lock` и `CustomName`;
  `saveAdditional` пишет их обычными NBT-полями.
- `BlockEntity.loadWithComponents` вызывает `loadAdditional`, затем читает
  карту `components`; `applyComponents` при обычном reload не вызывается.
- `saveWithoutMetadata` пишет `saveAdditional` и только stored explicit map.
  `collectComponents` нужен для item/pick-пути, а не для обычного chunk save.
- `applyComponentsFromItemStack` — отдельный путь placement. `removeComponentsFromTag`
  удаляет дубли из item `BLOCK_ENTITY_DATA`, не из обычного сохранения мира.

Vecta следует этому разделению. Новый regression загружает нормальную NBT-бочку
через `load_with_components` и сохраняет имя, lock и contents. Применение пустых
item components на reload могло бы стереть эти значения и содержимое контейнера.
Не добавлять предложенный прежним отчётом hook и не удалять корневые поля.
G021 остаётся задачей проверки реальных disk/item boundaries, без подтверждённого
этим пунктом архитектурного blocker. Подробности — в Codex review сеанса 2.

## Дополнительные известные partial после ревью

- G006/W06-026: общий bucket consumer ещё использует dimension-level
  `water_evaporates` и упрощённый level event. Java читает позиционный
  `WATER_EVAPORATES`, проигрывает `FIRE_EXTINGUISH` и отправляет large smoke.
  Это существующий пробел; добавление dispenser consumer не закрывает его.
- G011/W06-053: `grow_water_plant` пока игнорирует clicked face и biome/tag coral
  selection. Seagrass test не доказывает полную underwater bonemeal parity.
- G010: в Java перед блоком проверяются explosive SulfurCube entities.
  Такой реализации в Vecta нет; обычные TNT/fire tests не закрывают эту ветку.

Это зависимости дальнейшей реализации, не запросы владельцу без проверки
конкретной отсутствующей архитектуры или produced data.

## B-06-05 · снят — produced authorization metadata получена

Владелец запустил точную команду с JAVA_HOME=25 в обычном CMD: BUILD SUCCESSFUL
in 29s, output 26.3 / 49 типов. JSON скопирован побайтно; generator и настоящий
placement gate реализованы без ручного списка. См. актуальный review G021 и
`scripts/steel-extractor/block-entity-types.md`. Далее сохранена история диагностики;
указания ниже о необходимости получить output больше не являются текущим blocker.

Проверены Java `BlockItem.updateCustomBlockEntityTag`, `Player.canUseGameMasterBlocks`,
`BlockEntityType.onlyOpCanSetNbt`, Vecta `Player::is_operator`/permission manager
и generator `vecta-registry/build/block_entity_types.rs`. Права в Vecta есть;
отсутствует extracted flag у типов block entity, а текущий placement не проверяет gate.

Подготовлен dedicated SteelExtractor task `extractBlockEntityTypes` для 26.3.
Но обычный запуск и диагностический запуск с `--stacktrace` упали до Kotlin
compilation/extraction: `Unable to establish loopback connection`, причина
`java.net.SocketException: Invalid argument: connect` в `UnixDomainSockets.connect0`.
После повторного сбоя варианты flags не перебирались; EULA/server не запускались.

Запрошено: из обычного CMD в `D:\CodexBuilds\SteelExtractor` выбрать process-local
JAVA_HOME=`C:\Program Files\Eclipse Adoptium\jdk-25.0.4.7-hotspot`, затем выполнить
`.\gradlew.bat extractBlockEntityTypes --no-daemon --max-workers=1` и сообщить
результат либо путь к produced
`run\steel_extractor_output\steel-registry\build_assets\block_entities.json`.
Нужны `minecraft_version: 26.3` и реальные `only_op_can_set_nbt` для всех типов.
Ручной список/serde default false запрещены. Подробнее —
`scripts/steel-extractor/block-entity-types.md`; patch подготовлен, но extraction
пока не проверена. Независимую G022 продолжать, G021 целиком completed не ставить.

Уточнение по ответу владельца: запуск без task дошёл до Gradle configuration,
но использовал Java 17. JAVA_HOME действительно указывает на 17 при PATH java=25.
Попытка из tool environment с явно выбранным установленным JDK 25 сохранила
loopback failure. Владелец получил точные CMD-команды: process-local JAVA_HOME
на `C:\Program Files\Eclipse Adoptium\jdk-25.0.4.7-hotspot`, затем task
`extractBlockEntityTypes --no-daemon --max-workers=1`. Ожидается новый результат;
не запускать просто gradlew без имени задачи и не считать старый output подходящим.

## B-06-06 · G022 — `/data entity` требует общего entity NBT load

**Проверено:** `vecta-core/src/entity/` не имеет общего NBT-сериализатора.
`Entity::save_additional` заполняет только типо-специфичные поля,
`Entity::nbt_for_data_compare` собирает `saveWithoutId`-подобный compound для
предикатов, но обратной загрузки Pos/Motion/Rotation/UUID/Air/Fire/Passengers/
data нет: `ChunkStorage` восстанавливает сущности из типизированного
`PersistentEntity`, а не из NBT. Тот же гейт уже записан как TODO в
`/execute store entity` (`command/builtins/execute/store.rs`).

**Чего не хватает:** `Entity::load`-эквивалент (общая загрузка base-полей) плюс
`EntityDataAccessor.setData` с живым контекстом. Без него `/data entity`
был бы get-only, поэтому провайдер не зарегистрирован вовсе.

**Риск догадки:** порядок/набор base-полей и правила «player data restricted»
(`commands.data.entity.invalid`) — это shared foundation для всех сущностей,
включая сохранение. Полу-реализация затронула бы существующее поведение.

**Решение:** отдельный срез «общий entity NBT load/save» → затем провайдер
`entity` в `/data` и снятие TODO в `/execute store`. Зависимые карточки:
G022 (провайдер entity), G024 (living persistence).

## B-06-07 · G022 — `modify … compute` требует loot number providers

**Проверено:** `vecta-registry/src/loot_table/` содержит conditions/entries/
functions/context, но не `ContextFloatProvider`/`ContextIntProvider`, а
`LootContextSources` (источники контекста для `/execute`-подобного провайдера)
отсутствует целиком. `vecta_utils::value_providers::IntProvider` — worldgen,
не loot.

**Чего не хватает:** loot `ContextFloatProvider`/`ContextIntProvider` с
`getFloat`/`getInt(LootContext)` и `LootContextSources.addContextSources`.

**Решение:** `compute float|integer` не реализован; остальные четыре источника
(`from`, `string`, `value`) и все манипуляторы работают. Зависимые карточки:
G022 (compute), G027/G028 (loot-driven автоматизация).

## B-06-08 · G022/W06-108 — компаратор не видит содержимое контейнера

**Проверено:** `vecta-core/src/world/redstone.rs` не обращается к block entity
или `Container` ни в одной ветке `get_signal`; `ComparatorBlock::calculate_output`
через `DiodeBlock::get_input_signal` получает 0 для бочки с предметами.
`BlockEntity::set_changed` при этом корректно вызывает
`update_neighbor_for_output_signal`, и компаратор получает neighbor update.

**Следствие:** проверена только половина W06-108 (dirty flag + neighbor
notification); фактическое изменение output signal компаратора после `/data`
не воспроизводится, пока нет контейнерного redstone-сигнала
(`Container.getRedstoneSignal` / `AbstractContainerMenu`-независимый расчёт).

**Решение:** W06-108 остаётся partial. Отдельный срез редстоуна: сигнал из
контейнера в `get_signal`, затем тест comparator поверх `/data`.

## B-06-09 · G022 — item custom-data load без resolution context

**Проверено:** `BlockEntity::load_item_custom_data` вызывает `load_additional`
(detached), поэтому `LecternBlockEntity::load_book` не резолвит книгу при
установке предмета с `minecraft:block_entity_data`. Ревью G021 отметило, что
Java разрешает книгу для уже прикреплённой кафедры при `loadCustomOnly`.

**Чего не хватает:** resolution context в `BlockItem::place`. `for_source`
требует `CommandSource` (а значит `Arc<Server>`), а вводить ссылку
`World → Server` запрещено запросом. У `UseOnContext` есть только игрок и мир.

**Решение:** не угадывать. Нужно решение: либо контекст разрешения строится из
игрока размещения без `Server` (например, разрешать только `@s`/`~ ~ ~`), либо
`load_item_custom_data` получает `Option<&BlockEntityLoadContext>` и placement
передаёт `None` с явным отказом от резолва. Зависимые карточки: G022 (item
context), G024.
