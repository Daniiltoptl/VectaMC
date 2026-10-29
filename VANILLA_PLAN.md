# VectaMC: план до ванильной совместимости

Цель — поведение официального **Minecraft Java 26.3**: выживание, творчество,
приключение, наблюдатель, мир, сущности, команды и сохранения. Плагины,
возможности Paper/Purpur/Folia и другие версии идут после этой основы.

Это полный план областей работ и проверки, а не утверждение, что каждая
перечисленная механика отсутствует. SteelMC уже реализует значительную часть
основ. Существующие реализации сначала сравнивать с эталоном, затем дополнять.
Новые расхождения, найденные при проверке, добавлять в этот файл.
Не вычислять процент готовности по числу предметов, классов или тестов.

## Что передать Gemini

Начальный запрос: [GEMINI_TASK.md](GEMINI_TASK.md). Поимённая инвентаризация:
[planning/audit/README.md](planning/audit/README.md).
Для продолжения в WorkBuddy / DeepSeek: [актуальная очередь из 72 задач](NIGHT_TASK_03.md)
и [готовый стартовый запрос](WORKBUDDY_START_03.md). В пакете указаны текущая база,
зависимости, vanilla references, проверки и формат передачи на ревью.
[N00–N20](NIGHT_TASK.md) и [сеанс 02](NIGHT_TASK_02.md) сохраняют предыдущие задания;
их старые указания начать заново не заменяют актуальную историю реализации.

- [objects.csv](planning/audit/objects.csv) — все 3105 записей текущего
  `classes.json`: 1658 предметов, 1286 блоков и 161 тип сущности, включая
  уже зарегистрированные реализации.
- [block_entities.csv](planning/audit/block_entities.csv) — все 49 типов
  блок-сущностей и наличие фабрики в текущем реестре.
- [source_markers.csv](planning/audit/source_markers.csv) — указатели TODO/FIXME
  с файлами и строками. Часть комментариев может быть устаревшей.

У каждой строки `objects.csv` с регистрацией `no` нужно проверить, требуется ли
специальный серверный код, либо работу уже выполняют общие компоненты,
наследуемое поведение или отдельный путь. Обычный `Item`, обычный `Block`,
воздух и `Player` сами по себе не являются пропущенными реализациями.
Регистрация `yes` также не означает, что механика закончена.

Эталон берётся из `Cargo.toml`, затем готовится командой
`python scripts/prepare_vanilla.py`. Исходники находятся в
`minecraft-src/minecraft/src/net/minecraft/`; происхождение загрузки — в
`minecraft-src/.vecta-reference.json`. Эталон и серверные JAR в Git не добавлять.
Правила [AGENTS.md](AGENTS.md) обязательны; извлечённые JSON и generated Rust
вручную не редактировать.

## Подтверждённые примеры пробелов

Это точки входа в код, а не исчерпывающий перечень дефектов.

| Наблюдение | Где проверено | Связанные задачи |
| --- | --- | --- |
| Часть блок-сущностей сохраняет только NBT через fallback | `vecta-core/src/block_entity/registry.rs`, `entities/unimplemented.rs` | V06 |
| Рамки поддерживают размещение, взаимодействия, опору, дроп и pick; отслеживание карт остаётся открытым | `vecta-core/src/entity/entities/objects/display_ui/item_frame.rs`, `planning/reviews/N03-N19.md` | V05, V08 |
| End crystal содержит пропуски взрыва и связи с боем дракона | `vecta-core/src/entity/entities/objects/explosives/end_crystal.rs` | V07, V08 |
| Chest minecart пока не даёт полноценные движение и контейнер | `vecta-core/src/entity/entities/objects/vehicles/chest_minecart.rs` | V08 |
| Обработчики кнопок и состояния слотов меню имеют TODO | `vecta-core/src/player/player_inventory/player_handlers.rs` | V04 |
| При добыче отмечены незавершённые эффекты инструмента и среды | `vecta-core/src/player/game_mode/block_breaking.rs` | V03, V11 |
| Часть шаблонных структур отказывается размещаться с DataMarkers | `vecta-core/src/worldgen/structure/piece_placer/template_piece.rs` | V10 |
| Неизвестные фабрике мобы структур пропускаются | `vecta-core/src/worldgen/structure/piece_placer/mod.rs` | V08, V10 |
| Superflat с features/lakes отклоняется как неподдержанный | `vecta-core/src/worldgen/generator/registry.rs` | V10 |
| Есть пропущенные ветки execute, locate и summon NBT | `vecta-core/src/command/builtins/` | V12 |

Пузырёк опыта из рук и инструмент козьего рога уже добавлены в VectaMC.
Их не переписывать с нуля; оставшиеся взаимодействия проверять вместе с общей
системой раздатчиков, предметов, сетевой синхронизации и сохранений.

## Порядок работы

Сначала V00–V02: эталон, данные, протокол и сохранения. Затем V03–V07:
игрок, инвентарь, предметы, контейнеры, физика и редстоун. После необходимых
общих оснований — V08–V10: сущности, живой мир и генерация. V11–V13 вести
по готовности зависимостей. V14 проверяет результат каждого этапа; V15
следует после корректности. V16 отложен.

Одна задача — законченный небольшой набор взаимодействий с тестами.
Если следующая механика требует общего основания, сначала реализовать
основание целиком в пределах объявленного контракта. Не добавлять пустые
фабрики сущностей или поведение «для галочки».

## V00 · Воспроизводимость и проверка эталона

- [ ] **V00.01** На свежем клоне подтвердить toolchain, сборку, тесты и Clippy
  на Linux и Windows; сохранить базовые результаты и исходный коммит.
- [ ] **V00.02** Проверить соответствие версии Cargo, исходников Mojang,
  встроенных datapack-данных и всех нужных результатов SteelExtractor.
- [ ] **V00.03** После сборки обновить каталог `python scripts/audit_vanilla.py`;
  разобрать каждую строку без специализации и каждый маркер исходников.
- [ ] **V00.04** Завести журнал сценариев: условия, seed, действия, результат
  ваниллы, результат VectaMC, доказательства, статус и исправляющий коммит.
- [ ] **V00.05** Подготовить сценарий входа настоящего клиента 26.3 и двух
  игроков. Запуск сервера с принятием EULA — только по явному согласию владельца.
- [ ] **V00.06** Для завершённых задач фиксировать точные пределы проверки;
  не заменять игровой сценарий успешной компиляцией.

Приёмка: другой разработчик воспроизводит проверки по сохранённым командам.

## V01 · Данные, реестры, компоненты и протокол

- [ ] **V01.01** Сверить идентификаторы и порядок всех реестров с 26.3,
  теги, feature flags, размеры и типы полей configuration/play.
- [ ] **V01.02** Проверить все item components: NBT, сетевой формат, значения
  по умолчанию, optional-поля, удаления из patch, валидацию и хеширование.
- [ ] **V01.03** Проверить двусторонний обмен контейнеров: state id,
  carried stack, component hash, подтверждения и исправление рассинхронизации.
- [ ] **V01.04** Сверить metadata сущностей, equipment, attributes, effects,
  passengers, entity events, sounds, particles и дистанции рассылки.
- [ ] **V01.05** Проверить переходы login/configuration/play, cookies,
  transfers, resource packs, registry sync и feature negotiation, где они
  предусмотрены сервером 26.3.
- [ ] **V01.06** Проверить чанки, light, heightmaps, biomes, block entities,
  chunk batches, ACK и unload/reload во время движения и телепортации.
- [ ] **V01.07** Подтвердить ограничения декодирования: размеры, глубина NBT,
  недопустимые enum/id, конечность координат, частичные пакеты и compression.

Приёмка: совместимые форматы подтверждены эталонными данными и пакетными тестами;
невалидный пакет не повреждает мир и не роняет сервер.

## V02 · Миры и сохранения

- [ ] **V02.01** Проверить format/version полей level, chunk и region,
  dimensions, world border, gamerules, время, погоду, spawn и difficulty.
- [ ] **V02.02** Проверить сохранение и восстановление игрока: позиция,
  измерение, инвентарь, компоненты, здоровье, голод, XP, effects, abilities,
  game mode, spawn point, ender chest, статистика и прогресс.
- [ ] **V02.03** Проверить сущности: UUID, движение, equipment, пассажиры,
  владельцы, поводки, projectiles, порталы, теги и специальные поля каждого типа.
- [ ] **V02.04** Проверить блок-сущности, loot table + seed, контейнеры,
  scheduled ticks, POI и worldgen state при выгрузке и перезапуске.
- [ ] **V02.05** Устранить потерю неподдержанных сущностей при чтении/сохранении:
  сначала определить политику совместимости и реализовать необходимые типы;
  не считать пропуск сущности корректной поддержкой мира.
- [ ] **V02.06** Проверить аварийное прерывание записи, атомарность сохранения,
  резервное восстановление и одновременные autosave/выгрузки.
- [ ] **V02.07** Явно определить, какие существующие vanilla worlds и DataVersion
  можно открывать. Поддержку старых миров не заявлять без конвертации и тестов.

Приёмка: один и тот же сценарий до/после выхода, unload и restart даёт тот же
результат, без потери компонентов, объектов и связанных сущностей.

## V03 · Игрок, движение, добыча и бой

- [ ] **V03.01** Сверить walking, sprinting, sneaking, jumping, step height,
  swimming, crawling, climbing, flying, gliding и соответствующие poses.
- [ ] **V03.02** Проверить collisions, suffocation, fall damage, fluids,
  knockback, velocity, friction, slime/honey, powder snow и portal movement.
- [ ] **V03.03** Проверить порядок teleport/ACK, смену измерения, respawn,
  sleep/leave bed и поведение наблюдателя, включая camera и контейнеры.
- [ ] **V03.04** Довести добычу: инструмент, hardness, harvest level, efficiency,
  haste/fatigue, вода/воздух, instant break, adventure predicates и отмена добычи.
- [ ] **V03.05** Довести атаку: cooldown, reach/ATTACK_RANGE, critical/sweep,
  shield/blocking, armor, resistance, enchantments, invulnerability и damage types.
- [ ] **V03.06** Проверить использование предмета при смене слота, руки,
  game mode, смерти, оглушении, teleport и замене stack/component.
- [ ] **V03.07** Проверить здоровье, голод, exhaustion, saturation, regeneration,
  starvation, oxygen, fire/freezing, totem и все эффекты.
- [ ] **V03.08** Проверить смерть: inventory/XP drops, keepInventory, messages,
  combat tracking, bed/anchor spawn, cooldown портала и повторный вход.

Приёмка: выживание, креатив, adventure и spectator; две руки; граничные
дистанции и повтор действий; отсутствие дюпов и потери предметов.

## V04 · Инвентарь, рецепты и меню

- [ ] **V04.01** Проверить pickup, quick move, swap hotbar/offhand,
  clone, throw, quick craft, pickup all, drag и двойной клик.
- [ ] **V04.02** Закончить container button/slot-state handlers, проверить
  закрытие меню, disconnect, смерть и удаление/разрушение контейнера.
- [ ] **V04.03** Сверить slot restrictions, максимумы stack/component,
  перенос остатков и состояние курсора; исключить dupe/ghost items.
- [ ] **V04.04** Проверить shaped/shapeless, все специальные рецепты,
  remaining items, repair, dyes, books/maps, fireworks и decorated pots.
- [ ] **V04.05** Закончить recipe book, unlock/limited crafting,
  выбор/размещение рецепта и статистику/advancement callbacks.
- [ ] **V04.06** Проверить уже имеющиеся inventory, crafting table, furnace,
  blast furnace, smoker, anvil, grindstone, beacon и shulker menus.
- [ ] **V04.07** Реализовать или довести enchanting, brewing, smithing,
  stonecutter, loom, cartography и lectern; сверить каждое действие с эталоном.
- [ ] **V04.08** Довести меню сундуков/двойных сундуков, hopper,
  dispenser/dropper/crafter и связанные редстоун-сигналы.
- [ ] **V04.09** После V08 проверить merchant, horse/vehicle inventories
  и остальные типы из `vecta-registry/build_assets/menutypes.json`.

Приёмка: все типы меню из данных 26.3 разобраны поимённо; несколько игроков
одновременно не дублируют и не теряют предметы.

## V05 · Предметы и специальные действия

Все имена и варианты находятся в `objects.csv`; здесь — группы работ.

- [ ] **V05.01** Общий Item.use: BLOCKS_ATTACKS, KINETIC_WEAPON,
  equippable, cooldown/remainder, active use, consumables и finish/release hooks.
- [ ] **V05.02** Arrow/SpectralArrow/Bow/Crossbow: выбор боеприпаса,
  charge/release, multishot/piercing, попадания, pickup, damage/effects и NBT.
- [ ] **V05.03** Trident: бросок, loyalty, channeling, riptide,
  equipment/spin pose, погода/вода, pickup и перезапуск во время полёта.
- [ ] **V05.04** Wind charge и остальные projectile items: использовать
  общую систему projectiles/взрывов; проверить все уже реализованные броски.
- [ ] **V05.05** Bundle: item-click override, вместимость компонентов,
  вложения, выбор/выдача содержимого, обе руки и синхронизация меню.
  Базовая реализация N02 и [исправления ревью](planning/reviews/N01-N02.md)
  находятся в `codex/night-vanilla-01`. Настоящий клиент и restart-сценарий
  остаются непроверенными; полный пункт пока не закрывается.
- [ ] **V05.06** Bucket/MobBucket: capture/release, NBT моба, вода,
  waterlogging, ultra-warm/environment attributes и отказы размещения.
- [ ] **V05.07** EmptyMap/Map: создание, tracking, exploration maps,
  banners/frames, decorations, cloning/scaling/locking и persistence.
- [ ] **V05.08** Writable/Written/KnowledgeBook: редактирование, подпись,
  resolution, validation, copy generation, открытие и рецепты.
  N03 добавляет редактирование, подпись, открытие и проверки копирования;
  [ревью N03/N19](planning/reviews/N03-N19.md) фиксирует исправления и ограничения.
  KnowledgeBook/recipe book и точный codec JSON для предела страниц остаются открытыми.
- [ ] **V05.09** ArmorStand, Painting, ItemFrame/GlowItemFrame,
  EndCrystal и Cushion: placement, collision, support, damage, drops и NBT.
  - [x] ItemFrame/GlowItemFrame: размещение, вставка и поворот предмета,
    проверка опоры, дроп рамки и содержимого, `Fixed`, save/load.
    [Ревью Codex и исправления](planning/reviews/V05.09a.md): ENTITY_DATA,
    имена дропа, NBT и world-border collision; добавлены граничные сценарии.
    N19 добавляет entity pick-block для рамок и других уже реализованных типов;
    [ревью и ограничения](planning/reviews/N03-N19.md).
    Не поддержано: отслеживание карт (зависит от V05.07).
    Painting и остальные типы группы не начаты.
- [ ] **V05.10** Boat/ChestBoat/Raft и Minecart items — только после
  рабочих сущностей, пассажиров, управления и контейнеров.
- [ ] **V05.11** Banner, SmithingTemplate, DiscFragment, DebugStick,
  GameMasterBlockItem: проверить реальные серверные overrides; отличия
  tooltip/клиента не переносить искусственно в серверную логику.
- [ ] **V05.12** Для всех остальных строк `items` проверить food, potions,
  spawn eggs, tools, dyes, brushes, seeds, fireworks, music, equipment,
  component-driven interactions и их создание через команды/loot/crafting.

Приёмка каждой группы: main/offhand, survival/creative, stack 1/N,
cooldown, durability, stats, events, отсутствие результата при отказе,
сохранение компонентов и повторная загрузка. Клиентские эффекты не дублировать.

## V06 · Блоки, блок-сущности и рабочие станции

В текущем снимке 27 типов не имеют зарегистрированной фабрики. Это проверяется
в `block_entities.csv`; NBT fallback не заменяет tick, inventory и menu.

- [ ] **V06.01** Chest/trapped chest и copper chest: single/double,
  facing/waterlogging, obstruction, openers, sounds, comparator, loot и oxidation.
- [ ] **V06.02** Hopper: suction, transfer directions, cooldown/order,
  sided slots, entities, comparator и chunk boundaries.
- [ ] **V06.03** Dispenser/dropper: inventory, facing, redstone edge,
  выбор слота и все item-specific dispensing behaviors по 26.3.
- [ ] **V06.04** Crafter: disabled slots, рецепт, remainder, pulse,
  output/blocking, comparator и menu synchronization.
- [ ] **V06.05** Brewing stand: ingredients, fuel, recipes, timers,
  sided transfer, interruptions, menu и statistics.
- [ ] **V06.06** Enchanting table, anvil, grindstone и smithing:
  offers/seed/costs, bookshelves, components, repair/rename, incompatibilities.
- [ ] **V06.07** Lectern, bookshelf, decorated pot, banner и shelf:
  содержимое, components, interaction, redstone, ломание и сохранение.
- [ ] **V06.08** Conduit и beacon: shape validation, range, effects,
  targets, power/costs, изменения окружающих блоков и loading.
- [ ] **V06.09** Sculk sensor/calibrated sensor/catalyst/shrieker:
  vibrations, occlusion, scheduling, frequencies, XP/death, warning и Warden.
- [ ] **V06.10** Mob spawner, trial spawner, ominous item spawner и vault:
  configuration, активация, difficulty, учёт игроков, waves/rewards, cooldowns.
- [ ] **V06.11** Copper golem statues и creaking heart:
  активация/связанные сущности, состояние среды, destruction и persistence.
- [ ] **V06.12** Command block, structure block, jigsaw, test block и
  test instance block: разрешения, packets, NBT и реальное поведение.
- [ ] **V06.13** Проверить уже зарегистрированные sign/hanging sign,
  barrel, furnaces, campfire, jukebox, beehive, bell, comparator, detector,
  moving piston, brushable, end gateway/portal, ender chest, sulfur, shulker и skull.
- [ ] **V06.14** Разобрать оставшиеся классы блоков из каталога:
  growth/bonemeal, nylium/moss, infested, eggs, light, glass,
  TNT, dried ghast, stations и остальные специальные состояния.
- [ ] **V06.15** Проверить у всех блоков placement/replacement,
  state transforms, support, waterlogging, drops, tool interactions,
  pathfinding, client update и neighbor notifications.

Приёмка: создание, использование, изменение соседей, break/explosion,
unload/restart; для контейнеров также automation и несколько игроков.

## V07 · Физика мира, редстоун, жидкости и взрывы

- [ ] **V07.01** Сверить порядок neighbor/shape updates, scheduled ticks,
  random ticks, block/fluid tick priorities и поведение на границе чанков.
- [ ] **V07.02** Проверить wire, torches, repeaters, comparators,
  observers, buttons/levers, pressure plates, sensors и все sources/consumers.
- [ ] **V07.03** Проверить pistons/sticky pistons: push limits,
  slime/honey, block entities, moving collisions и update order.
- [ ] **V07.04** Довести общую систему взрывов: entity/block damage,
  resistance, exposure, knockback, drops, fire, gamerules и events.
- [ ] **V07.05** Подключить TNT, crystals, creepers, fireballs, beds,
  anchors, wind charges и прочие источники после общего основания.
- [ ] **V07.06** Проверить воду/лаву: flowing/source conversion, mixing,
  waterlogging, containers, bubbles, entity push и dimension/environment rules.
- [ ] **V07.07** Довести fire spread/aging, flammability, weather/lightning,
  freezing/melting, gravity blocks, falling damage и extinction.

Приёмка: одинаковые схемы и последовательности тиков на VectaMC и Mojang,
включая выгрузку/загрузку и разные границы чанков.

## V08 · Сущности и искусственный интеллект

Каждая строка `entities` из каталога — отдельная проверка поведения и сохранений.
Уже зарегистрированные типы также проверяются полностью. `Player` создаётся
через сетевой lifecycle, а не обычную фабрику мобов.

- [ ] **V08.01** Общие foundations: navigation/pathfinding, goals/brain,
  memories/sensors, visibility, targeting, movement controls и AI scheduling.
- [ ] **V08.02** Lifecycle моба: spawn finalization, equipment/enchantments,
  drops/XP, despawn, persistence, pickup, breeding, age, taming и conversion.
- [ ] **V08.03** Проверить уже существующие cow/mooshroom/pig/sheep/chicken,
  bat/endermite и их варианты, движения, взаимодействия и загрузку.
- [ ] **V08.04** Базовые враждебные: zombie/husk/drowned/zombie villager,
  skeleton/stray/bogged/parched, spider/cave spider, creeper, slime/magma cube.
- [ ] **V08.05** Разведение/приручение/транспорт: horse/donkey/mule,
  camel/camel husk, llama/trader llama, wolf/cat, rabbit, fox, panda,
  polar bear, ocelot, armadillo, goat, turtle и другие из каталога.
- [ ] **V08.06** Вода и летающие: все fish, squid/glow squid, dolphin,
  axolotl, frog/tadpole, bee, parrot, allay, sniffer и nautilus variants.
- [ ] **V08.07** Nether/End: piglins/bartering/anger, hoglin/zoglin,
  blaze, ghast/happy ghast, strider, enderman, shulker и соответствующие variants.
- [ ] **V08.08** Villager/wandering trader: POI, professions, schedules,
  restock, trades, reputation, gossip, breeding, iron golems и raids.
- [ ] **V08.09** Illagers/witch/vex/ravager/phantom/guardians,
  breeze, Warden, creaking, copper golem, sulfur cube и остальные типы 26.3.
- [ ] **V08.10** Bosses: dragon fight, crystals, respawn, portals/XP,
  wither spawning, phases, attacks, damage и persistence.
- [ ] **V08.11** Boats/rafts/chest variants и все minecarts:
  movement, rails, passenger/input, collisions, damage, drops и inventories.
- [ ] **V08.12** Projectiles/clouds/fangs/lightning/TNT:
  owner, hit detection, fluid/block interaction, effects, damage и NBT.
- [ ] **V08.13** Armor stands, paintings, frames, displays, mannequin,
  interaction/marker и прочие технические сущности: серверные действия,
  command-visible state, metadata и persistence.
- [ ] **V08.14** Завершить общую систему езды: recursive passengers,
  control/jump/inventory, dismount search, teleport и сохранение дерева.

Приёмка каждого типа: spawn/interaction/AI/combat/drop/despawn, save/load,
spawn eggs и команды; отсутствие фабрики не обходить созданием пустого Entity.

## V09 · Живой мир и естественный спавн

- [ ] **V09.01** Естественный спавн по категориям, caps, player distances,
  biome/structure rules, light, difficulty, collision, pack size и despawn.
- [ ] **V09.02** Structure spawns, special spawners, patrols, raids,
  phantom/insomnia, wandering trader и остальные специальные циклы 26.3.
- [ ] **V09.03** Random-tick экология: crops, stems, saplings/trees,
  leaves, vines, fungi, moss, flowers, spread/decay и bonemeal.
- [ ] **V09.04** Время/погода/сезонные или позиционные environment attributes,
  lighting, sky/block exposure и dimension-specific behavior по эталону.
- [ ] **V09.05** Mob-effect hooks: bad/raid omen, infested, oozing,
  weaving, wind charged и прочие эффекты с entity spawn или изменением мира.
- [ ] **V09.06** POI management/persistence, villages, beds, workstations,
  portals, compass targets и взаимодействия соответствующих мобов.

Приёмка: контролируемые циклы spawn/despawn и естественное выживание
после длительной симуляции и перезапуска.

## V10 · Генерация, структуры и измерения

- [ ] **V10.01** Сверить Overworld/Nether/End: seed, RNG, noise,
  biomes, terrain, surfaces, carvers, aquifers и lighting boundaries.
- [ ] **V10.02** Поимённо проверить все configured/placed features и placement
  modifiers; неподдержанные enum/codec не заменять пустым результатом.
- [ ] **V10.03** Сверить structure sets, placement, starts/references,
  pieces, jigsaw pools/processors, transforms и clipping.
- [ ] **V10.04** Закончить family-specific DataMarkers, loot containers,
  block entities, structure mobs и финализацию каждого семейства структур.
- [ ] **V10.05** Завершить flat features/lakes и сверить остальные
  поддерживаемые generator settings и presets с 26.3.
- [ ] **V10.06** Проверить spawn position, spawn chunks, portal construction,
  coordinate scaling, gateways, End platforms и dragon-related generation.
- [ ] **V10.07** Golden-проверки по фиксированным seed/координатам:
  block states, biomes, heightmaps, structures и loot seeds, независимо от
  порядка/числа потоков, save/reload и порядка генерации соседних чанков.

Приёмка: конкретные расхождения с эталонными чанками устранены;
наличие структуры в реестре не считается её размещением в мире.

## V11 · Loot, зачарования и прогресс

- [ ] **V11.01** Проверить все loot tables/conditions/functions,
  contexts, luck, tools/equipment, enchantments и воспроизводимость seed.
- [ ] **V11.02** Довести каждый enchantment effect/trigger,
  costs/compatibility, attributes, location effects и durability/unbreaking.
- [ ] **V11.03** Проверить crafting/smelting/brewing/trading progression,
  XP, recipe unlocks, статистику и критерии достижений.
- [ ] **V11.04** Завершить advancement listeners/triggers/rewards,
  persistence, revoke/grant, recipe/function rewards и сетевую синхронизацию.
- [ ] **V11.05** Довести scoreboard objectives/criteria, stat links,
  teams, display slots, visibility, death count и persistence.
- [ ] **V11.06** Проверить bossbars, raid/dragon/wither progression,
  victory/death, portals и повторный запуск сохранённого мира.

Приёмка: последовательное прохождение от нового игрока до завершения
ванильного прогресса с корректными рецептами, достижениями и сохранениями.

## V12 · Команды и datapack-механики

- [ ] **V12.01** Получить список команд 26.3 и поимённо сопоставить с
  `vecta-core/src/command/`; проверить syntax, permissions, suggestions,
  selectors, contexts, result count и сообщения, значимые для командных цепей.
- [ ] **V12.02** Довести execute conditions/store, entity NBT,
  summon NBT, data, item/loot, locate biome/poi и недостающие ветки.
- [ ] **V12.03** Проверить command blocks, minecart command blocks,
  functions/tags, scheduling, predicates, return/macro behavior по 26.3.
- [ ] **V12.04** Сверить gamerules, difficulty/force-gamemode, world border,
  time/weather/spawn, teams/scoreboard/bossbars и persistency командного состояния.
- [ ] **V12.05** Согласовать поддержку внешних datapacks до реализации:
  текущий AGENTS запрещает проектировать runtime JSON loading. Сначала закрыть
  встроенные vanilla data/functions в разрешённой typed-архитектуре; изменение
  этой политики требует отдельного решения владельца.
- [ ] **V12.06** Сверить управление сервером: whitelist/ban/op, save/stop,
  RCON, сообщения/ошибки и корректную остановку с сохранением.

Приёмка: сценарии команд проверяются по observable результату, включая
селекторы, результаты execute и эффекты в мире, а не только parsing.

## V13 · Многопользовательский сервер

- [ ] **V13.01** Проверить join/rejoin, online identity/authentication,
  encrypted/compressed sessions, player list, permissions и смену состояний.
- [ ] **V13.02** Проверить chat/session signatures, ACK, filtering,
  командный чат, visibility, команды и правила отправки сообщений.
- [ ] **V13.03** Проверить entity tracking и game events с несколькими
  игроками на границах view/simulation distance и при смене измерений.
- [ ] **V13.04** Проверить порядок конкурентных действий: блоки, бой,
  containers, vehicles, teleport, disconnect, chunk unload и autosave.
- [ ] **V13.05** Проверить packet floods/invalid actions, resource bounds,
  cancellation и восстановление после сетевых ошибок без дюпов и потери мира.
- [ ] **V13.06** Проверить корректную остановку при активных игроках,
  генерации, записи мира и незавершённых сетевых запросах.

Приёмка: отсутствие расхождений между клиентами, потери состояния,
deadlocks и воспроизводимых способов дублировать предметы.

## V14 · Итоговая проверка ванильного поведения

- [ ] **V14.01** Для каждой задачи сохранить сравнимый сценарий на Mojang
  26.3 и VectaMC с одинаковыми условиями и seed, где это применимо.
- [ ] **V14.02** Пройти новый мир: ресурсы → еда → инструменты → дом →
  фермы → Nether → зачарования/зелья → End/dragon → поздняя игра.
- [ ] **V14.03** Отдельно проверить creative/adventure/spectator,
  command blocks, technical worlds и все специальные объекты 26.3 из каталога.
- [ ] **V14.04** Повторить ключевые сценарии с 2+ игроками,
  unload/reload, logout/rejoin и stop/restart.
- [ ] **V14.05** Проверить существующие ванильные сохранения в объявленных
  пределах поддержки и сравнить результат повторного сохранения.
- [ ] **V14.06** Вести открытые расхождения и известные ограничения;
  объявлять совместимость только в подтверждённых пределах.

## V15 · Устойчивость и производительность

- [ ] **V15.01** Провести длительные прогоны с игроками, мобами,
  генерацией, редстоуном, контейнерами и регулярным сохранением.
- [ ] **V15.02** Измерять MSPT/TPS, tail latency, CPU/RAM, I/O,
  размер очередей, время join/teleport/chunk delivery и утечки.
- [ ] **V15.03** Сравнивать с Mojang на одинаковом железе, мире,
  настройках, прогреве и нагрузке; публиковать команды и исходные результаты.
- [ ] **V15.04** Оптимизации проверять прежними vanilla-сценариями;
  не менять результаты RNG, порядок значимых тиков и сохранения ради benchmark.

## V16 · После ванильной основы

- [ ] **V16.01** Спроектировать поддержку других протоколов отдельно
  от воспроизведения механик каждой версии. Сначала подтвердить диапазон
  реально опубликованных версий и их данные; не предполагать будущие форматы.
- [ ] **V16.02** Для каждой следующей версии отдельно обновлять эталон,
  extraction, protocol, registries, mechanics, saves и regression suite.
- [ ] **V16.03** После этого определить plugin API и требования к
  совместимости с Paper/Purpur/Folia. Загрузка JVM-плагина на Rust-сервере
  требует отдельной архитектуры и не следует из наличия Rust plugin API.

## Как закрывать задачу и отдавать на ревью

В отчёте: ID задачи, исходный/итоговый коммит, изменённое поведение,
использованный vanilla-класс/метод, зависимости, тесты с результатом,
проверка настоящим клиентом и оставшиеся ограничения.

Минимум для каждого изменения: подходящие регрессионные тесты,
`cargo fmt --all --check` и проверка затронутых crates. Перед передачей этапа:

```sh
cargo check --locked --workspace
cargo test --locked --workspace
cargo clippy --locked --all-targets --all-features -- -D warnings
python -m unittest discover -s scripts -p "test_*.py" -v
```

Не ослаблять AGENTS, тесты, линтеры, лимиты или проверку данных ради зелёного CI.
Не принимать «готово», если фактически сделаны только реестр, пустой класс,
fallback, комментарий или тест, повторяющий константу.
