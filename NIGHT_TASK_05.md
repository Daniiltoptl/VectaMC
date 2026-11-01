# Длинный сеанс 05 — ванильный gameplay после обновлений SteelMC

> Архивная очередь из 100 карточек. Расширенное актуальное ТЗ:
> [NIGHT_TASK_06.md](NIGHT_TASK_06.md) — 600 карточек в 120 направлениях.

Это задание **писать и проверять код**, не составлять ещё один план. Ниже 100 карточек
для нескольких длинных сеансов. Не обещай выполнить всё за ночь. Закрыл проверяемый
slice → узкие tests → review staged diff → commit/push → следующий независимый slice.
Не заканчивай после первой задачи или часа, пока доступны факты и время сеанса.

## Старт

- Каталог `C:\Users\My comp\Desktop\Vecta`, репозиторий `Daniiltoptl/VectaMC`.
- Продолжать **actual tip `codex/night-vanilla-03`**. Не reset к SHA из старых документов.
- Прочитать AGENTS.md, `planning/night-05/checkpoint.md`,
  `planning/reviews/upstream-2026-11-01.md`, `server-random-sequences.md`,
  `W04-019-034-codex.md`, предыдущие `W04-001-021.md` и `W04-016-029.md`.
- Target берётся из Cargo.toml: `0.16.1+mc26.3`. Local manifest: 26.3, 5037 Java,
  server SHA1 `33680f5f2ac32864d6d7cf5e56a705fdb3e05f4c`.
- Пока без плагинов и без поддержки клиентов 1.21–26.2. Следующие версии отдельно.
- Старые NIGHT_TASK_04/WORKBUDDY_START_04 исторические. Не выполнять их устаревшее
  указание начать заново с W04-001..006. Подробные старые acceptance criteria полезны.
- Чужой `planning/night-01/report.md` не редактировать, не stage, не stash/reset.
  SHA256 `06680CBC971245B70C990D95C906F0F1E3685EFFDE61C13D562664B4B6B05222`.
- Не менять AGENTS, не делать main/merge/force push, общий clean/reset, не запускать
  дополнительные агенты/Gemini/Mistral CLI. Stage только собственные paths/hunks.

## Что сохранять

Рамки обоих видов: placement, взаимодействие, предмет/поворот, опора, дроп и pick.
Мешки: inventory clicks, selection packet, содержимое и использование. Рабочие
меню камнереза, loom, кафедры, smithing; initial stonecutter recipes и контекст книг.
Crafting result/remainders/stats; ПКМ и Q забирают весь результат, crafted count
считается по ванильному requested/min contract. Не возвращать ошибочную «половину стака».
Implicit BE components, lock/name/lazy loot, одинарные/двойные/trapped/copper сундуки,
viewers, closing menus, hopper/sided automation/dropper и основа dispenser уже сделаны
в описанном в reviews объёме. Наличие этой очереди не доказывает отсутствие механики.

Сохранять component replacement при pick, отсутствие hopper/container доступа к книге
кафедры и equipment callbacks вне inventory locks. `as_living_entity()` уже есть;
GameEventListener позволяет наблюдать события без нового mock bus.

Named RNG общий на Server, seed у server default world, в том числе для разных domains.
Владелец явно согласовал последовательные игровые тики миров. WorldMap сохраняет
порядок вставки и после восьмого world. Не возвращать parallel dispatch/отдельные банки.
Состояние RNG хранится SavedDataManager в TOML, не в Mojang random_sequences.dat.
Snowball и generic equippable dispenser route уже работают; остальные special actions
проверять по конкретным consumers. Перенесённые TNT/explosion, Mth, hurt/splash и tab-list
читать по свежему upstream review, не реализовывать заново и не считать full vanilla.

## Порядок работы

Начать с **W05-001**, затем 002, 003, 006, 007, 009–013 по готовности consumers.
004 требует 005, 008 — настоящих существ 079, 016 — лодок 059, 017 — стойки 052.
Не регистрировать поведение на пустую/NBT-only сущность. Если foundation отсутствует,
закончить её с первым реальным consumer либо оставить точный blocker и взять независимую
карточку. Метод, которого пока нет в Rust, сам по себе не блокер, если его контракт ясен.

Перед изменением: Vecta callers → локальная Java → extracted data → tests → код.
Если контракт/архитектура всё ещё неясны, вопрос владельцу по AGENTS. Молчание не согласие.
При повторных сбоях одной категории не перебирать flags и обходы бесконечно.
Никаких stubs, fake vanilla data, пустых shared hooks и speculative fallbacks.
Карточка закрывается действительным путём action/packet → dispatch → state → sync →
save/load, где это предусмотрено Java. Preview/codec/registration/stat counter недостаточны.

Эталоны карточек перечислены по именам классов. Проверенные полные пути находятся в
`planning/night-05/reference-index.json`, относительно minecraft-src/minecraft/src/net/minecraft/.
В 26.3 есть BoneMealItem/BottleItem, BrewingRecipe вместо старого PotionBrewing,
Enderman/EnderDragonFight/RecipeCraftedTrigger. Старые package paths из памяти не брать.

## Данные, сборки и handoff

SteelExtractor: `D:\CodexBuilds\SteelExtractor`, обычно output в run/steel_extractor_output/.
Сначала проверить checkout/version/output, затем нужный extractor. Workflow:
`scripts/steel-extractor/README.md`. Не считать старый checkout свежим для нового extractor.
Копировать только нужные produced assets с provenance. Generated Rust/extracted JSON
вручную не менять, protocol/registry IDs не вписывать из памяти. Seeded системы сохраняют
ванильный RNG и порядок draws. Rust self-consistency не равна совпадению с Java.

Каждая тяжёлая Cargo-команда **-j 1**. Env:
`CARGO_TARGET_DIR=D:\CodexBuilds\VectaMC`, `CARGO_INCREMENTAL=0`,
`TMP=TEMP=D:\CodexBuilds\tmp`; logs в `D:\CodexBuilds\logs`.
Проверить свободное место и активные сборки. Не закрывать VM/приложения, не убивать все
rustc/Java, не менять ОС, зависимости/toolchain/flags/profiles ради прохождения сборки.
Новый обоснованный tooling slice не разрешает обходить правила проекта.

После slice — meaningful tests, не tests констант ради счётчика. Нулевой найденный test
count не pass. При старте не пересобирать уже проверенный baseline без новой причины.
Перед передачей группы: fmt, locked workspace tests, workspace Clippy
all-targets/all-features с -D warnings, Python unittest и audit. Не полный suite после
каждой строки; после нового изменения — relevant перепроверка. Не suppress lint для обхода.
Keyed DowncastType с уникальным owner key и SAFETY разрешён AGENTS; другой unsafe,
production unwrap/expect и Any/TypeId не импортировать под видом upstream доверия.
Игровой tick не ждёт disk/network async. EULA без явного согласия не принимать.

Вести `planning/night-05/progress.md` и blockers.md. На slice: W05/W04 ID, commit,
результат для игрока, прочитанные Java методы, команды/exit/count, оставшаяся зависимость.
README/CHANGELOG обновлять кратко и по факту. Перед лимитом оставить actual HEAD/status,
dirty paths, незавершённый diff, active build/session/log и конкретный следующий ID.
Не коммитить непроверенную foundation ради окончания лимита. В конце обязательно обычный
текстовый ответ, не только файлы: сделано/частично/tests/branch/push/report/next.
Отдельно честно указать настоящий клиент, Linux, Java golden и два сетевых игрока.

## A. Раздатчики — первые готовые consumers

### W05-001. Пузырёк опыта

Эталон: ExperienceBottleItem, ThrownExperienceBottle. Связь: W04-032.
Подключить готовую сущность с её config, копией стака и spawn hooks. Импульс создаёт
летящий пузырёк, расходует один предмет, даёт верные events; rejected insertion не отменяет расход.

### W05-002. Три варианта яиц

Эталон: EggItem, ThrownEgg. Связь: W04-032.
Проверить EGG/BLUE_EGG/BROWN_EGG по Java 26.3, вариант и настоящее вылупление.
Приёмка: обе руки при use, dispenser, попадание, расход и NBT; один alias без variant behavior не готов.

### W05-003. Взрывное зелье

Эталон: SplashPotionItem, ThrownSplashPotion. Связь: W04-032.
Замкнуть launch/hit с potion components. Проверить две цели на разных расстояниях,
duration/instant effects, отсутствие owner у раздатчика и расход ровно одного стака unit.

### W05-004. Туманное зелье

Эталон: LingeringPotionItem, ThrownLingeringPotion. Зависимость: 005.
Попадание создаёт настоящее работающее облако с компонентами. Проверить повторное
наложение и owner attribution; визуальный explosion или NBT-only cloud недостаточны.

### W05-005. Облако эффектов

Эталон: AreaEffectCloud. Связь: W04-064.
Реализовать wait/duration/radius/reapplication и save/load. Две цели входят и выходят:
effects не накладываются каждый tick вместо cooldown, истёкшее облако действительно удаляется.

### W05-006. Наполненные вёдра

Эталон: BucketItem, DispenseItemBehavior, LiquidBlock. Связь: W04-033.
Подключить воду, лаву и рыхлый снег, waterlogging и остаток тары. Проверить допустимую
и заблокированную цель, среду, полный dispenser и обычное использование предмета.

### W05-007. Подбор пустым ведром

Эталон: BucketPickup, DispenseItemBehavior. Зависимость: 006.
Связать pickup_block и заполненную тару; flow не подменять source fluid. Проверить
успех/отказ, полный inventory и выдачу лишней тары, без изменения блока до успешной операции.

### W05-008. Вёдра с существами

Эталон: MobBucketItem, Bucketable. Зависимости: 006, 079.
Выпускать настоящих существ с entity data/from-bucket состоянием. Проверить round-trip
подбора/выпуска, содержимое ведра и остаток тары; отсутствующий aquatic AI не скрывать.

### W05-009. Оставшиеся equippable случаи

Эталон: EquipmentDispenseItemBehavior, LivingEntity. Связь: W04-034.
Сохранить готовый fallback; проверить saddle/body slots/allowed entities/занятый slot.
Player и mob получают нужное снаряжение, mob — guaranteed drop/persistence; dropper экипировку не надевает.

### W05-010. Огниво и fire charge

Эталон: DispenseItemBehavior, TntBlock. Связь: W04-035.
Подключить TNT, campfire/candle/fire и success/failure. Проверить прочность/расход,
TNT_EXPLODES, blocked target и events; второй explosion engine не создавать.

## B. Специальные действия и автоматизация

### W05-011. Костная мука

Эталон: BoneMealItem, DispenseItemBehavior. Связь: W04-035.
Настоящие growable consumers для item/dispenser. Проверить crop, sapling и water plant,
расход, failure и existing worldgen features; hardcoded случайный блок не считается ростом.

### W05-012. Бутылочки

Эталон: BottleItem, BeehiveBlock, DispenseItemBehavior.
Набирать воду/мёд, изменять блок и выдавать тару. Проверить неподходящий блок,
полный улей и полный inventory без потери результата или повторной выдачи.

### W05-013. Ножницы

Эталон: ShearsItem, Sheep, BeehiveBlock.
Использовать общие реальные shear consumers. Проверить wool/honeycomb, прочность,
дроп и недопустимое повторное срезание; gamerules/events совпадают с Java.

### W05-014. Головы и тыквы

Эталон: CarvedPumpkinBlock, WitherSkullBlock. Зависимость: 077.
Специальные dispenser branches замкнуть на настоящие конструкции и spawning.
Неверная конструкция цела, успех удаляет ровно нужные блоки; NBT-маркер вместо моба запрещён.

### W05-015. Default-dispatch 26.3

Эталон: DispenserBlock, DispenseItemBehavior.
Проверить explicit behavior/entity-data spawn egg/equippable/sulfur-cube swallowing
и их приоритет. ItemStack с несколькими подходящими компонентами выбирает ровно ванильную ветку.

### W05-016. Лодки из раздатчика

Эталон: BoatDispenseItemBehavior, AbstractBoat. Зависимость: 059.
Подключить настоящие boat/raft/chest варианты. Проверить source water, воздух над
водой, сушу, положение/rotation и components; rejected placement не дублирует предмет.

### W05-017. Стойка из раздатчика

Эталон: ArmorStandItem, ArmorStand. Зависимость: 052.
Использовать общую установку живой стойки. Проверить координаты, направление,
entity components и отказ; placeholder ArmorStand factory не закрывает ветку.

### W05-018. Автокрафтер

Эталон: CrafterBlock, CrafterBlockEntity, CrafterMenu. Связь: W04-030, 037.
Блок/BE/menu, togglable slots, redstone craft, output/remainders. Проверить вставку
результата или выброс, comparator, NBT и повторный импульс без дюпа.

### W05-019. Зельеварка

Эталон: BrewingStandBlockEntity, BrewingStandMenu, BrewingRecipe. Связь: W04-036.
Использовать рецепты 26.3, топливо и brewing cycle. Три бутылки, смена ingredient,
hopper sides и reload сохраняют progress; старый PotionBrewing из другой версии не копировать.

### W05-020. Цепочка автоматизации

Эталон: HopperBlockEntity, DispenserBlock, TntBlock. Зависимости: готовые 001–019.
Собрать сундук→hopper→dropper/dispenser→действие. Несколько настоящих ticks,
reload и полный output сохраняют баланс предметов; direct helper call не заменяет цепочку.

## C. NBT, lifecycle и рабочие станции

### W05-021. Implicit components round-trip

Эталон: BlockEntity. Связь: W04-014.
Довести disk/owned/borrowed/live base components. Save/load, item placement/pick
не теряют custom data и не возвращают удалённый компонент из старого NBT.

### W05-022. Живое /data

Эталон: DataCommands, BlockDataAccessor, BlockEntity. Зависимость: 021.
Обновить live BE и menu sync с существующим BlockEntityLoadContext. Проверить
comparator/ticker; первоначальный disk load не должен повторно разрешать книгу кафедры.

### W05-023. NBT рамок

Эталон: ItemFrame, GlowItemFrame. Зависимость карт: 082.
Проверить Fixed/Invisible/Rotation/drop chance/components через все NBT пути.
Сохранить готовые placement/pick и explosion veto обеих рамок; карты не подменять пустым tracking hook.

### W05-024. Постоянное living state

Эталон: LivingEntity, ServerPlayer.
Health/HurtTime/DeathTime/effects/equipment сверить на save/load/respawn. Новая жизнь
не сохраняет старый hurt timer/death guard; command NBT и игровой state согласованы.

### W05-025. NBT раздатчика

Эталон: DispenserBlockEntity, DropperBlockEntity. Зависимость: 021.
Проверить loot/name/lock/base components через open/hopper/explosion. Lock до открытия,
seed и содержимое не сбрасываются, повторный consumer не генерирует лут второй раз.

### W05-026. Прочность и equip hooks

Эталон: LivingEntity, ItemStack.
Замкнуть armor/tool damage на настоящие use/hurt paths. Проверить break/components,
creative/unbreaking/обе руки; callbacks не вызываются под inventory mutex.

### W05-027. Печь: результаты и XP

Эталон: AbstractFurnaceBlockEntity, AbstractFurnaceMenu. Связь: W04-044.
Recipe usage/output callbacks/stats и накопленный XP. Take/shift/hopper/reload
не удваивают опыт; выдача обязана быть связана с реальным smelting consumer.

### W05-028. Печь: автоматизация

Эталон: HopperBlockEntity, AbstractFurnaceBlockEntity. Зависимость: 027.
Сохранить сделанные sided slots. Длинный fuel/remainder цикл, смена рецепта и
chunk unload не теряют progress и не пропускают предмет в запрещённый слот.

### W05-029. Украшенный горшок

Эталон: DecoratedPotBlock, DecoratedPotBlockEntity.
Довести insertion/wobble/break modes/components. Полный горшок, projectile/tool
break, NBT и comparator; содержимое падает ровно один раз.

### W05-030. Shelf 26.3

Эталон: ShelfBlock, ShelfBlockEntity.
Сверить swap/interaction и хранение. Полный/пустой slot, обе руки, save/load,
два игрока и предусмотренные Java redstone/comparator consumers работают сквозным путём.

## D. Книга рецептов и меню

### W05-031. RecipeDisplay

Эталон: RecipeDisplay, SlotDisplay, RecipeManager.
Добавить варианты с настоящими recipe-book consumers, extracted IDs и codec tests.
Stonecutter update-recipes не переписывать: полный RecipeDisplay ему не нужен.

### W05-032. ServerRecipeBook

Эталон: ServerRecipeBook. Зависимость: 031.
Known/highlight recipes и settings сохраняются на игроке. Login/rejoin/respawn,
серверное удаление рецепта и повторное открытие не оставляют недействительные записи.

### W05-033. Recipe-book packets

Эталон: ClientboundRecipeBookAddPacket, ServerGamePacketListenerImpl. Зависимость: 032.
Связать add/remove/settings с lifecycle. Unknown IDs, initial order относительно
tags/recipes и повторный login проверяются реальными пакетами, без ручных packet IDs.

### W05-034. Place recipe

Эталон: ServerPlaceRecipe, RecipeBookMenu. Зависимости: 031–033.
Serverbound placement работает с авторитетным inventory и настоящей сеткой.
Shaped/shapeless, разные menu sizes и нехватка ingredients не создают дюп.

### W05-035. Max craft и компоненты

Эталон: StackedItemContents, RecipeBookMenu. Зависимость: 034.
Подбор ingredients и repeated craft учитывают stack limits/components.
Нестакаемые items, повреждённые tools и полный inventory не смешивают несовместимые stacks.

### W05-036. Открытие рецептов

Эталон: KnowledgeBookItem, RecipeCommand, ServerRecipeBook. Зависимость: 032.
Awards/limited crafting/knowledge book и команды give/take работают сквозным путём.
Проверить права, неверный key и повторную выдачу; stat counter не равен advancement.

### W05-037. Crafting callbacks

Эталон: ItemStack, ResultSlot, CrafterBlock. Зависимости: 018, 081 для maps.
Различить onCraftedBy/System/postprocess в 26.3, сохранив исправленный ПКМ/Q.
Preview не меняет saved map, take и crafter не выполняют callback дважды.

### W05-038. Наковальня

Эталон: AnvilMenu.
Repair/combine/rename/enchantment limits и XP cost. Проверить too expensive,
creative, оба входа и пустое имя; XP списывается только за действительный take.

### W05-039. Точило

Эталон: GrindstoneMenu.
Repair, curses и XP выдаются по Java. Curse-only item, два повреждённых предмета
и разные способы take не позволяют получать опыт за preview или повторно.

### W05-040. Зачаровательный стол

Эталон: EnchantmentMenu.
Offers от player seed/bookshelf power/components. Одинаковый seed даёт те же offers,
lapis/levels списываются атомарно, новый seed появляется после успеха, не preview.

## E. Снаряды и оружие

### W05-041. AbstractArrow

Эталон: AbstractArrow, BowItem.
Физика/collision/in-ground/owner с первым настоящим bow consumer. Стена, moving
entity и unload/reload; регистрировать пустую Arrow ради dispenser запрещено.

### W05-042. Arrow и SpectralArrow

Эталон: Arrow, SpectralArrow. Зависимость: 041.
Damage/pickup/critical/piercing/glow и dispenser. Проверить creative-only pickup,
shield и застрявшую стрелу; damage идёт через общий hurt path.

### W05-043. Стрелы с эффектами

Эталон: Arrow, PotionContents. Зависимости: 041, 042.
Potion/custom effects/color переживают flight/hit/pickup. Подобранный предмет
содержит допустимые компоненты; длительность эффектов совпадает с Java.

### W05-044. Лук

Эталон: BowItem. Зависимость: 041.
Draw/release/ammo/enchants и статистика. Короткое натяжение, infinity/creative,
обе руки и отсутствие ammo; прямой hurt вместо летящей стрелы запрещён.

### W05-045. Арбалет

Эталон: CrossbowItem. Зависимости: 041, 046.
Charging/charged projectiles/multishot/piercing/rockets. Прерывание и смена слота
не дублируют ammo; charged item сохраняется в NBT и стреляет один раз.

### W05-046. Фейерверк

Эталон: FireworkRocketEntity, FireworkRocketItem.
Lifetime/elytra attachment/payload и dispenser. Проверить обычный запуск, полёт
игрока, damage через преграды и NBT; NBT-only ракета не закрывает задачу.

### W05-047. Small fireball

Эталон: SmallFireball, FireChargeItem. Зависимость: 010.
Настоящий снаряд, dispenser config, hit и ignition. MobGriefing, горючая/негорючая
цель и owner attribution; обычный выброс fire charge предметом недостаточен.

### W05-048. Wind charge

Эталон: WindCharge, AbstractWindCharge, ServerExplosion.
Добавить настоящий TRIGGER_BLOCK consumer в общую explosion систему. Knockback,
fall reset и активация блоков проверяются отдельно; TNT-разрушением не подменять.

### W05-049. Жемчуг эндера

Эталон: ThrownEnderpearl, EnderpearlItem.
Owner/teleport/damage/tickets и сохранение. Смена мира/домена, смерть владельца,
restart; ссылка на владельца не удерживает disconnected player бесконечно.

### W05-050. Рыбалка

Эталон: FishingHook, FishingRodItem.
Throw/retract/bobber/bite/seeded loot/tool effects. Вода/суша, пойманная entity,
обрыв owner и durability проходят реальные paths; лут берётся из общей системы.

## F. Декор, respawn и транспорт

### W05-051. Картины

Эталон: Painting, PaintingVariant, HangingEntity.
Variant/размер/место/опора/drop/pick/NBT. Проверить пересечение и creative/survival;
placeholder factory не равна установленной картине.

### W05-052. Установка стойки

Эталон: ArmorStand, ArmorStandItem.
Размеры/marker/small/pose/hands и equipment access. Предмет ставит живую стойку,
обе руки работают со слотами; creative и permissions соответствуют Java.

### W05-053. Урон стойке

Эталон: ArmorStand. Зависимость: 052.
Damage exceptions, marker/invisible, break stages и equipment drops. Проверить
melee/projectile/fire/explosion/doEntityDrops; обычный mob death не заменяет контракт.

### W05-054. Cushion

Эталон: Cushion, CushionItem.
Placement/seating/passenger positions/опора. Mount/dismount двух игроков,
collision/save/load/drop; наличие сущности не считается реализацией посадки.

### W05-055. End crystal

Эталон: EndCrystal, EndCrystalItem. Fight callbacks зависят от 078.
Замкнуть placement на настоящий explosion consumer и damage attribution.
Проверить требуемый блок, свободное место, chain explosion; fight hook не делать пустым.

### W05-056. Двери и люки

Эталон: DoorBlock, TrapDoorBlock.
Player/redstone use, половины, waterlogging, neighbors. Слом половины не оставляет
дюп, закрывание меняет collision и настоящий путь AI.

### W05-057. Кровати

Эталон: BedBlock, ServerPlayer.
Sleep/night skip/respawn и dimension explosion. Занятое/сломанное ложе,
безопасный respawn и отсутствие двойного взрыва двух половин.

### W05-058. Якорь возрождения

Эталон: RespawnAnchorBlock, ServerPlayer.
Charge/use/respawn и explosion вне допустимого измерения. Off-hand glowstone,
max charge, расход и заблокированный respawn не теряют/дублируют заряд.

### W05-059. Лодки и плоты

Эталон: AbstractBoat, Boat.
Fluid states/buoyancy/rowing/passengers/collision с первым вариантом. Вода/суша,
падение, два пассажира и NBT; затем типизированно подключить остальные и chest variants.

### W05-060. Вагонетки и рельсы

Эталон: AbstractMinecart, BaseRailBlock.
Rails/slopes/collision и существующий chest-container. Powered/activator rails,
unload/NBT; TNT/hopper/furnace variants требуют собственных consumers.

## G. Survival и действия игрока

### W05-061. Голод

Эталон: FoodData, Player.
Exhaustion/saturation/food/regen/starvation/sprint. Проверить difficulty,
freeze и сохранение; sync не отправляет отрицательные/переполненные значения.

### W05-062. Еда и consumables

Эталон: Consumable, ItemStack.
Start/finish/cancel и реальные consume effects. Обе руки, тара, creative,
full hunger и смена слота; preview не применяет effects.

### W05-063. Эффекты

Эталон: MobEffectInstance, LivingEntity.
Instant/stacking/duration/milk/honey и повторное наложение. Weaker/stronger,
immunity/death/reload и particles; custom effects не теряются в components.

### W05-064. Щит

Эталон: ShieldItem, LivingEntity, ItemStack.
BLOCKS_ATTACKS текущей версии, угол, damage tags, disable cooldown, обе руки,
durability и порядок reductions; старые константы 1.21 не переносить из памяти.

### W05-065. Ближний бой

Эталон: Player, LivingEntity. Зависимость: 064.
Cooldown/crit/sweep/knockback/armor/enchants/source. Sprint attack, creative,
spectator, teammate policy и два игрока; общий hurt_server не обходить.

### W05-066. Урон среды

Эталон: LivingEntity, Entity.
Fall/fire/lava/drown/freeze/suffocate и порядок ticks. Иммунитеты/totem/armor,
вход в воду; upstream splash/hurt timers сохраняются, прямой periodic hurt недостаточен.

### W05-067. Смерть и respawn

Эталон: ServerPlayer, PlayerList. Зависимости: 024, 049.
KeepInventory/XP/drop/messages/cleanup menus/pearls. Две смерти и смена мира
не дублируют loot; новая жизнь не сохраняет death guard или старые callbacks.

### W05-068. Опыт и mending

Эталон: ExperienceOrb, Player.
Merge/pickup delay/mending/level conversion/orb values. Несколько игроков,
полный ремонт и остаток XP; сохранить проверенный разброс опыта бутылки.

### W05-069. Natural spawning

Эталон: NaturalSpawner. Зависимости: реальные mobs 071–075.
Caps/distance/light/biome/structure rules. Passive/hostile группы в загруженных
чанках; отсутствие AI не скрывать успешным spawn count.

### W05-070. Despawn и persistence

Эталон: Mob.
Distance/no-action/named/tamed/from-bucket. Два игрока, spectator, unload/reload
и mob с equipment из раздатчика; persistent mob не исчезает из-за общего despawn helper.

## H. Мобы и AI

### W05-071. Зомби

Эталон: Zombie. Зависимость: 065.
Movement/target/melee/sun/baby/NBT. Реальное преследование, преграда,
daylight и death loot; constructor/TODO-only factory не готова.

### W05-072. Скелет

Эталон: Skeleton, AbstractSkeleton. Зависимости: 041, 044.
Ranged AI с настоящими arrows/equipment/sensing. Line of sight, дистанция,
смена оружия, owner/source и NBT; стрелу не имитировать прямым hurt.

### W05-073. Крипер

Эталон: Creeper.
Swelling/cancel/ignition/charged/explosion. MobGriefing, дистанция, смерть до
detonation и special drops; TNT tick не заменяет creeper AI.

### W05-074. Паук

Эталон: Spider.
Climbing/jump/target/daylight/sizes. Стена, потолок, узкий проход и прекращение
преследования; движение через общий collision, без teleport к игроку.

### W05-075. Эндермен

Эталон: Enderman.
Взгляд/агрессия/teleport/вода/held block. MobGriefing, запрещённые блоки,
safe destination, damage и NBT; не ставить произвольный блок вместо extracted rules.

### W05-076. Жители

Эталон: Villager, MerchantMenu.
Profession/POI/trades/demand/restock и merchant inventory. Два игрока,
исчерпание сделки, репутация/NBT; сначала закончить нужные POI/Brain foundations.

### W05-077. Големы

Эталон: IronGolem, SnowGolem. Связь: 014.
Создание конструкцией и полноценный первый gameplay slice каждого вида.
Player-created flag, targets, снег/среда и death loot; constructor-only mob недостаточен.

### W05-078. Дракон и End fight

Эталон: EnderDragonFight, EnderDragon.
Manager/crystals/ritual/bossbar/gateways с настоящими consumers. Death/rejoin/
restart/repeat ritual; отсутствующая фаза не имитируется пустым hook ради EndCrystal.

### W05-079. Водные существа

Эталон: AbstractFish, Squid. Связь: 008, 069.
Bucket species и aquatic movement/spawn/air. Выпуск/подбор, суша,
player distance/NBT; видимая неподвижная сущность не равна работающему AI.

### W05-080. Приручаемые существа

Эталон: TamableAnimal, Wolf, Cat.
Tame/owner/sit/follow/breed/species equipment. Offline owner, смена мира,
friendly damage и NBT; UUID owner не превращать в сильную циклическую ссылку.

## I. Карты, прогресс и команды

### W05-081. Хранилище карт

Эталон: MapItemSavedData, MapItem.
Manager/IDs/dimensions/colors/dirty regions и реальные map-data packets.
Два items одного ID, отдельные карты и reload сохраняют правильное общее состояние.

### W05-082. Карты в руках и рамках

Эталон: ItemFrame, MapItem. Зависимость: 081; связь N18.
Decorations/tracking/rotation/remove/лимит рамок. Обычная/glow рамка,
удар/разрушение, несколько игроков и restart; пустой tracking hook запрещён.

### W05-083. Картографический стол

Эталон: CartographyTableMenu, MapItem. Зависимости: 081, 037.
Scale/lock/clone/postprocess на настоящих данных. Output/shift/remainders,
общее ID у копий; preview не изменяет saved map.

### W05-084. Знамёна

Эталон: BannerBlockEntity, BannerItem, LoomMenu.
Placement/components/patterns/washing/NBT. Item→block→pick/drop и pattern limits;
сохранить loom selection/tags, не придумывать зависимость loom от recipe book.

### W05-085. Sculk

Эталон: SculkSensorBlockEntity, CalibratedSculkSensorBlock, VibrationSystem.
Delay/occlusion/frequency поверх существующих game events. Wool, расстояние,
calibrated side/cooldown; listener без настоящего tick/delay не закрывает систему.

### W05-086. Teams и friendly fire

Эталон: Scoreboard, ServerScoreboard, Entity.
Довести team properties и runtime gameplay read access к доменной scoreboard.
FriendlyFire=false защищает союзника от blast/melee, knockback идёт по Java; domains не смешиваются.
Архитектуру доступа согласовать по AGENTS после проверки существующих command contexts.

### W05-087. Advancements

Эталон: PlayerAdvancements, InventoryChangeTrigger, RecipeCraftedTrigger.
Настоящие progress/listeners для inventory_changed/crafting/unlock по extracted
data. Award once/revoke/rejoin; stat counter и no-op hook не считаются критериями.

### W05-088. Statistics

Эталон: StatsCounter, ServerStatsCounter.
Сверить реальные item/block/entity actions и persistent stats. Обе руки,
creative exceptions/reload/relogin; регистрация stat без consumer не готова.

### W05-089. /loot, /item, /random

Эталон: LootCommand, ItemCommands, RandomCommand.
Подключить к настоящим loot/components/server RandomSequences. Reset/defaults,
seed flags/permissions/domain/save; второй RNG bank для команды не создавать.

### W05-090. Gamerules

Эталон: GameRules, MinecraftServer.
Закрыть подтверждённые отсутствующие gameplay consumers и persistence.
Live change/rejoin/restart меняют поведение; сохранённый boolean без consumer не готов.

## J. Паритет, генерация и инструменты

### W05-091. Расширенный Java golden взрывов

Эталон: ServerExplosion, ExplosionDamageCalculator.
Сохранить перенесённые ray/cache/TNT tests, расширить produced Java fixtures для
boundaries/resistance/fluid/blocklike/callbacks. Уже совпадающий engine заново не переписывать.

### W05-092. Собственный worldgen parity прогон

Эталон: ChunkGenerator.
Воспроизводимый набор чанков трёх измерений с Java seed/coordinate manifest.
Сохранять первый diff и hashes; upstream 7500 chunks не выдавать за собственную проверку Vecta.

### W05-093. Биомы, noise, heightmaps

Эталон: NoiseBasedChunkGenerator, NoiseChunk. Зависимость: 092.
Сложные координаты/seeds после Mth/section переноса. Сравнивать block/biome/
heightmap outputs, не один hash; fixtures генерировать, JSON руками не исправлять.

### W05-094. Структуры

Эталон: StructureTemplate, StructureTemplatePool.
Processors/block entities/structure entities с lazy loot. Одинаковый seed
даёт те же structure contents после загрузки; entity NBT не теряется в template placement.

### W05-095. Порталы и border

Эталон: NetherPortalBlock, EndPortalBlock, PortalForcer.
Search/create/scaling/safe placement/border. Linked worlds/domains, unload,
passengers/owner/возврат; сохранить согласованные domain defaults.

### W05-096. Chunk/entity lifecycle

Эталон: PersistentEntitySectionManager, ServerChunkCache.
Длинный unload/load/save с entities/containers/RNG. Нет duplicate UUID/повторного
loot; потоки продолжаются в стабильном порядке миров, как в принятой архитектуре.

### W05-097. Измеренная производительность

Эталон: ServerExplosion, ChunkMap.
Замерить TNT/containers/entities на одинаковом workload с сырыми результатами.
Parity до/после; не ускорять удалением gameplay, сменой profiles/flags или отключением проверок.

### W05-098. Flint отдельным tooling slice

Эталон: upstream steel-flint на проверенном SHA; Java entry point не применим.
Адаптировать bridge на реальные World/Player APIs и pinned dependency. Не копировать
глобальные lint allow/production unwrap. Specs/filtering проверить; PR auto-comments без запроса не включать.
Связь: upstream 01f29bb86, 2f4d13e88, d4655730d, 1a74818e0. Это не gameplay feature.

### W05-099. Полный Java golden лута

Эталон: LootTable, LootPool, LootItemFunction.
Получить produced {table, seed, slots, nextDraw} и сверить inventory fill/nested
tables. Версия/команда/provenance обязательны; повторяемость Rust сама по себе не доказывает vanilla parity.

### W05-100. Длинный survival сценарий

Эталон: MinecraftServer, ServerPlayer. Зависимости: выбранные готовые slices.
Добыча→крафт→контейнеры→бой→смерть→respawn→save/rejoin. Два настоящих клиента при
доступности; иначе client validation остаётся открытой, EULA самовольно не принимается.
