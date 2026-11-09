# Night 06 — blockers

Проверенные блокеры. Формат: что проверено → чего не хватает → риск догадки →
точный path/output/decision → зависимые карточки.

## Сеанс7: актуальный статус B-06-07

Единая typed ContextMap и миграция текущих loot callers реализованы; выбирать
эту архитектуру повторно не нужно. Produced metadata16ключей/31set получена и
используется build generator. Нового общего provider codec/factory ещё нет.

Java StoredNumberAccess требует level→server command storage; ScoreboardValue
требует scoreboard. Проверены World и server-owned DomainScoreboards/
DomainCommandStorage. Согласованный read context домена реализован в
`70f27a545`: shared Arc backend, без World→Server ссылки. EntityRef передаёт
profile name/UUID как отдельную score-holder identity. Live Score/Storage и
EntityScores подключены и проверены; workspace4308/0/18. Этот доступ больше не
блокирует providers/compute. Повторно согласовывать владение не надо.

Из local SteelExtractor уже получен type catalog:28float+23int=51, включая
number_dispatcher в обеих группах. Output лежит в checkout, source/reproduction
сохранены в scripts/steel-extractor/loot-contexts.md. Configured provider values
тоже извлечены:4float+26int, RegistryOps codec round-trip прошёл; файл остаётся
в extractor output до реализации typed consumer. Недостающий extractor output
больше не блокер, factory/codecs и foundations consumers ещё открыты.
Не заменять отсутствующие consumers no-op variants.
Подробности: [сеанс07](report-next-07.md).

## Актуализация после ревью сеанса6, 2026-11-03

Ниже сохранена история диагнозов. Актуальные уточнения имеют приоритет:

- B-06-06: supported living loader, disk round-trip, Team/UUID и accessor
  реализованы. Ревью исправило numeric/partial codecs, Packed.apply, raw effects,
  sleeping и combat references. Brain/locator-bar icon остаются без foundations;
  это открытый объём G024, а не отсутствие всего LivingEntity loader.
- B-06-07: пользователь поручил выбрать путь; выбран единый typed ContextMap
  внутри существующего LootContext и миграция всех loot callers. Сохраняются
  shared seeded RNG/domains, второй контекст не вводится. Архитектурный выбор
  закрыт, реализация context map/providers/compute ещё не выполнена.
- Старое «providers только inline, extractor не нужен» неверно как общий
  контракт: TYPE и configured registries существуют. Inline constants не
  отменяют registry dispatch; missing данные извлекать SteelExtractor.
- B-06-10 остаётся: критерий durability требует реального trigger engine;
  существующий advancement state/ручные команды сначала проверить.

Доказательства и границы: [ревью](../reviews/night-06-session06-codex.md).
Следующий большой пакет: [запрос07](../../WORKBUDDY_CONTINUE_06_07.md).

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

**Уточнено Codex:** subtype load/save, typed `PersistentEntity` и disk factories
существуют; `nbt_for_data_compare` собирает command-visible compound.
`Entity::apply_spawn_data` уже читает Pos/Motion/Rotation/Air/Fire/tags/data и
часть флагов. Утверждение, что обратной загрузки этих полей совсем нет, неверно.
Но spawn partial-update не равен полному live load: отсутствующие поля сохраняют
старые значения, position меняется локально; UUID/passengers и world tracking
нужно проверить отдельно. `/execute store entity` пока имеет соответствующий TODO.

**Чего не хватает:** полный `Entity::load` contract с live world tracking и
`EntityDataAccessor.setData`, переиспользующий существующие base/subtype readers.
Без него `/data entity` был бы get-only, поэтому провайдер пока не зарегистрирован.

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

## B-06-08 · Ошибочная диагностика контейнерного comparator input

**Перепроверено Codex, 2026-11-03:** `world/redstone.rs` хранит torch bookkeeping,
а не логику comparator input. `behavior/blocks/redstone/diode/comparator.rs`
читает `BlockBehavior::get_analog_output_signal`; `container/barrel_block.rs`
считает fullness настоящего контейнера. `BlockEntity::set_changed` уже вызывает
`world/block_updates/mod.rs::update_neighbor_for_output_signal`.

Общая foundation существует. Новый расчёт в `get_signal` не нужен и создал бы
вторую реализацию. Регрессионный сценарий ревью: `/data` заполняет бочку,
компаратор получает отложенный tick и выдаёт 15; удаление Items возвращает 0.
Результаты проверок и точные границы — в `../reviews/night-06-g022-codex.md`.

## B-06-09 · G022 — item custom-data load без resolution context

**Проверено:** `BlockEntity::load_item_custom_data` вызывает `load_additional`
(detached), поэтому `LecternBlockEntity::load_book` не резолвит книгу при
установке предмета с `minecraft:block_entity_data`. Ревью G021 отметило, что
Java разрешает книгу для уже прикреплённой кафедры при `loadCustomOnly`.

**Уточнение Codex, 2026-11-03:** `Player::server()` уже предоставляет `Arc<Server>`;
новая ссылка World→Server не нужна. Явный read-only контекст загрузки ранее
согласован владельцем. Ветка авторизованной установки protected-NBT кафедры
уже требует игрока. Не ограничивать разрешение только `@s`/координатами и не
передавать detached как подмену отсутствующего live consumer.

**Осталось реализовать:** провести существующий контекст от placement caller
к custom-only loader с сохранением OP/type gates, item snapshot и порядка
custom-data → components → setPlacedBy. Для playerless callers отдельно
проверить фактические зависимости. Это открытая реализация, а не подтверждённый
внешний архитектурный блокер; следующий пакет начинается с неё.

## B-06-09 · СНЯТ — placement получает контекст из игрока

Ревью G022 верно указало на ложную диагностику: `Player::server()` существует,
явный `BlockEntityLoadContext` согласован ранее, новая ссылка World→Server не
нужна. Реализовано `BlockEntityLoadContext::for_server_world(server, world)` и
передача из `BlockItem::place_with` через `player.server.upgrade()` (тот же
паттерн, что в `lectern_block.rs`/`books.rs`); playerless placement остаётся
detached. Server-backed тест: OP+creative ставит кафедру с unresolved книгой,
страница резолвится из storage и не переразрешается после смены storage.
Зависимые карточки: G021/G022 (item context).

## B-06-06 · PARTIAL: провайдер есть, полный living load отсутствует

Общий reset-load реализован: `Entity::apply_entity_nbt(nbt, EntityNbtLoad)`
переиспользует существующие readers `apply_spawn_data` (Pos/Motion/Rotation/
Air/Fire/tags/флаги/subtype loader) и различается только трактовкой
отсутствующего ключа и способом коммита позиции (`try_set_position` для живой
entity). `/data entity` и `/execute store entity` используют
`Entity::load_command_data`; UUID загрузчиком не читается.

Ревью Codex выявило больший остаток: общий LivingEntity loader не вызывается.
Health/attributes/effects/hurt/death/absorption и прочие living fields
экспортируются, но не загружаются. Disk путь также сохраняет subtype NBT
отдельно от command dump. Не считать провайдер полной foundation. Сначала G024.

Player bypass через execute store, single target и numeric conversions/sync
исправлены в ревью. Team API в scoreboard.rs уже существует; отсутствие UX
команд не внешний блокер. Нужны Team/UUID order, postDataManipulated и оставшийся
общий load contract. Детали: `../reviews/night-06-session05-codex.md`.

## B-06-07 · уточнён — `compute` это типизированные providers, не extractor

`ContextFloatProvider`/`ContextIntProvider` — inline codecs с диспетчеризацией
по типу (`ContextFloatProviderTypes`), а не registry-данные из SteelExtractor.
В 26.3 это ~30 реализаций на float и столько же на int
(`Absolute/Average/Ceiling/ConditionalValue/ConstantValue/Cosine/Difference/
EnchantmentLevel/EnvironmentAttribute/Floor/FromInt/Length/Maximum/Minimum/
Modulus/Negate/Power/Product/Quotient/Round/Sine/SquareRoot/StorageValue/Sum/
Truncate/Uniform/WeightedList`), плюс `LootContextSources` для источников
контекста. Это отдельный крупный срез: типизированные провайдеры + источники +
реальный command consumer. Половину узла команды не коммитил.

## B-06-07 · уточнён повторно — `compute` требует param-map LootContext

**Проверено по Java 26.3:**

- `DataCommands` compute-ветка: `ResourceOrIdArgument.floatProvider/intProvider`
  + `LootContextSources.addContextSources`.
- `ContextFloatProviders.DIRECT_CODEC` — `BuiltInRegistries.CONTEXT_FLOAT_PROVIDER_TYPE.byNameCodec().dispatch(...)`
  плюс inline `ConstantValue.INLINE_CODEC`; сами провайдеры — реестр
  `Registries.CONTEXT_FLOAT_PROVIDER`, бутстрап кодом (4 float-ключа).
- 27 float- и 22 int-реализации (полный список — в report-next-06.md), у каждой
  codec и правила вычисления.
- `LootContextSources` строит `LootParams` с
  `LootContextParamSets.COMMAND_COMPUTE_DEFAULT/POSITION/ENTITY` и параметрами
  `THIS_ENTITY/ORIGIN/BLOCK_STATE/BLOCK_ENTITY/TARGET_ENTITY`.

**Чего не хватает в Vecta:** `LootContext` (`vecta-registry/src/loot_table/context.rs`)
— плоская borrow-структура без param map/param sets. Compute на ней потребовал
бы вторую контекст-модель рядом с существующей.

**Решение:** отдельный срез «param-map LootContext + param sets + два реестра
провайдеров с типовой диспетчеризацией (нужны extracted данные) + 49 реализаций
+ ResourceOrIdArgument», затем `compute float|integer`. Половину узла команды не
коммитил.

## B-06-10 · `ITEM_DURABILITY_CHANGED` criteria

`ItemStack::hurt_and_break` по-прежнему содержит TODO на
`CriteriaTriggers.ITEM_DURABILITY_CHANGED`: advancement-criteria foundation в
Vecta отсутствует целиком. Само событие разрыва (`onEquippedItemBroken`) теперь
вызывается — см. `0d29632d8`. Зависимые карточки: advancement-срез.

## B-06-11 · `environment_attribute`, owned `LevelBasedValue` и inline-условия

Три типа/формы из каталога провайдеров не имеют foundation в Vecta и поэтому
честно отвергаются (генератор паникует, парсер даёт ошибку):

- `environment_attribute` — нужен настоящий API environment attributes и
  origin/context, который Java использует; постоянным значением не заменяется.
- `enchantment_level` — payload это `LevelBasedValue`, а существующий тип хранит
  `'static`-внутренности; для command payloads нужен owned-вариант.
- Inline-условия в command payloads — `LootCondition::MatchBlock` хранит
  `&'static [PropertyCheck]`; owned-представление ещё не сделано. Registry
  предикаты (`block/fast_cooking`, `tool/can_shear`, `tool/can_silk_touch`)
  работают.

Пока это так, `compute` не может принять эти payload-формы; отсутствие не
маскируется значениями по умолчанию.


## Снято: compute больше не блокируется

B-06-07 в части command consumer закрыт: `/data modify … compute` работает на
фундаменте typed providers (см. report-next-08). Открытыми остаются только
три формы из B-06-11 (`environment_attribute`, owned `LevelBasedValue`,
inline-условия) — они по-прежнему дают ошибку вместо вычисления.

## Сеанс 9: WorldlyContainerHolder больше не блокирует автоматизацию

Пункт ревью сеанса 8 «Воронка не видит `WorldlyContainerHolder`» снят:
`behavior/worldly_container_holder.rs` + `BlockBehavior::as_worldly_container_holder`,
`HopperBlockEntity::container_at` спрашивает держателя до блок-энтити (порядок
Java `getBlockContainer`). Компостер — первый consumer; дроппер и будущий
crafter идут через тот же общий путь.

Остаются открытыми хвосты ревью сеанса 8 (все — зависимые срезы, не внешние
блокеры): контекст печи без `BLOCK_ENTITY`/`CONTAINER`; `CookingFuel` хранит
`Identifier` без inline-формы `ResolvableInt`/`ResolvableFloat`;
`block_entity_loot_ref` в compute заполняет только тип; открытые формы
B-06-11 (`environment_attribute`, owned `LevelBasedValue`, inline-условия).

Границы компостера, зафиксированные сверкой Java: `Compostable` в Vecta
хранит только ссылку на configured provider (`layers: Identifier`), как все
ванильные предметы 26.3 в extracted data. В 26.3
`ResolvableInt.STREAM_CODEC` — `ByteBufCodecs.either(Constant, Reference)`,
то есть сеть допускает и inline-число; предмет с inline `compostable` сейчас
в Vecta декодируется ошибочно. Это тот же дефект, что у `CookingFuel`
(ревью сеанса 8); закрытие — общий срез Resolvable* inline-форм, не отдельный
blocker компостера.

## Сеанс 10: Farmland — открытые расхождения, зафиксированные сверкой Java

1. **Spawn protection не смоделирована.** Ванильный `FarmlandBlock.fallOn`
   для игрока требует `!server.isUnderSpawnProtection(level, pos, player)`
   (DedicatedServer: список опов непуст И игрок не оп И spawnProtectionRadius>0
   И позиция в Chebyshev-радиусе от спавна измерения респавна). В Vecta список
   опов и радиус защиты спавна недоступны из поведения блока; ветка игрока
   реализована без проверки — это в точности поведение ванили при пустом
   списке опов (DedicatedServer возвращает false). Закрытие — срез
   spawn-protection (ops list + radius + respawn data), не блокер фарма.
2. **Bonemeal использует thread RNG вместо level random.** Пока правил срез
   crop/farmland, найдено: `behavior/items/bonemeal.rs` берёт `rand::rng()`
   для is_bonemeal_success, смещений разброса и `random_range(0..10)`;
   ваниль везде берёт `level.getRandom()` (в т.ч.
   `Mth.nextInt(level.getRandom(), 2, 5)` в `getBonemealAgeIncrease`). Тот же
   класс расхождения, что исправлен у crop growth в этом сеансе. Срез:
   перевести bonemeal на `world.random().with_level`.
3. **Порядок-зависимый тест copper chest transformer.**
   `copper_chest_transforms_both_halves_and_preserves_independent_properties`
   падает при запуске с узким фильтром `block_transformer` ("Block entity
   registry not initialized") и на чистом HEAD тоже; в полном проге проходит,
   потому что реестр инициализирует более ранний тест. Лечится `init_globals()`
   в `Interaction::new` (fixture) — не сделано в этом сеансе, чтобы не трогать
   чужой fixture без необходимости.
4. Хвосты ревью сеансов 8–9 остаются: inline-формы ResolvableInt/ResolvableFloat
   (Compostable/CookingFuel), контекст печи без BLOCK_ENTITY/CONTAINER,
   `block_entity_loot_ref` заполняет только тип, B-06-11, формы компостера не
   сверялись.

## Сеанс 11: остатки и новые находки

1. **CONTAINER без типа значения.** `ContextValueKind::Container` — только
   метаданные реестра; ваниль `LootContextParams.CONTAINER` (SlotProvider)
   требуется в `CONTAINER_PROCESS`. Печь в Vecta не может отдать live-слоты:
   `FurnaceContainer::tick` держит мьютекс контейнера, ваниль же передаёт
   `this`. Нужен typed value family (`SlotProvider`) и дизайн «live-слоты без
   повторного захвата» — отдельный срез, до того контекст печи остаётся без
   CONTAINER.
2. **У печи нет ContainerIdentity** (ваниль `AbstractFurnaceBlockEntity`
   extends `BaseContainerBlockEntity` → CustomName/lock). BLOCK_ENTITY ref
   печи отдаёт custom_name = None. Связано с G-задачами меню печи.
3. **Фurnace custom_name из Resolvable-рефов: только текст.**
   `plain_custom_name` в /data compute извлекает только `Content::Text`;
   translate/selectors без резолютора считаются безымянными. Потребитель
   (`copy_name` в vecta-registry) пока стаб — расширить при реализации
   copy_name.
4. Хвосты предыдущих сеансов остаются: bonemeal на thread RNG; spawn
   protection не смоделирована; порядок-зависимый copper chest transformer
   тест; B-06-11 (`environment_attribute`, owned `LevelBasedValue`,
   inline-условия, `slot_source` protocol id 57) — по потребителю.
5. **RecipeDisplay не расширяется кастомными типами** (в отличие от
   SlotDisplay с PayloadCodec::Custom): enum покрывает только 5 ванильных
   видов, kind→type диспетчеризация всегда идёт к глобальным статикам.
   Если появится потребность в модовых recipe display — отдельный срез
   (добавить Custom-вариант по образцу CustomSlotDisplay).
6. **`special: false` у CustomRecipe-рецептов в extractor-данных.** Ваниль
   `CustomRecipe.isSpecial()` = true (armor_dye, book_cloning, repair и
   т.п.), а generated vanilla_recipes.rs помечает их `special: false` —
   распакованные entries таких рецептов получают пустой
   `craftingRequirements` список (Some(vec![])) вместо None (wire: bool 1 +
   VarInt 1 против bool 0). Клиентский highlight «can craft» при пустом
   списке true — поведенчески близко, но байты отличаются. Лечится в
   extractor (special по сериализатору) — отдельный срез, при первом
   живом consumer recipe book.
