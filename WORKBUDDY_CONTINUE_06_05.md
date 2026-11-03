# Большой сеанс 05: VectaMC, доведение ванильных систем

Это задание на выполнение кода в течение доступного сеанса, а не просьба написать
ещё один план. Рабочая очередь ниже охватывает более 150 карточек существующего
`NIGHT_TASK_06.md`. Выполняй последовательно, не заканчивай сеанс после первой
группы. Объём очереди не означает обещание закончить её за одну ночь.

## Начало и сохранность работы

Работай в `C:\Users\My comp\Desktop\Vecta`, на actual tip ветки
`codex/night-vanilla-03`. Проверь status/log/remote. Исторические SHA в отчётах
не являются указанием откатить ветку. Не начинай новый fork SteelMC.

Один раз прочитай `AGENTS.md`, регламент `NIGHT_TASK_06.md`, актуальный
`planning/night-06/checkpoint.md` и
`planning/reviews/night-06-g022-codex.md`. Затем читай только очередную группу,
её Vecta/Java callers, извлечённые данные и тесты. Не перечитывай 600 карточек
перед каждым изменением. Нужные группы и их IDs уже определены ниже.

Проверенный workspace после ревью `cd11aa81a`: **4279 passed /0 failed /18 ignored**,
fmt/Clippy exit0, utils228, paths31. Cargo warning mimalloc при all-features —
существующий cfg с dhat-heap, не повод удалять default allocator. Подробности в review.

Чужие изменения не изменять и не индексировать:

- `planning/night-01/report.md`, SHA256
  `06680CBC971245B70C990D95C906F0F1E3685EFFDE61C13D562664B4B6B05222`;
- `WORKBUDDY_START_06.md`, SHA256
  `48430DC1D56AF1E679E0A81411518002B8C406B3F27648AD1A197DF174AFAD5A`.

Если фактическое состояние отличается, сначала выясни происхождение изменений;
не применяй reset/clean/stash и не перетирай другую работу. Не запускай других
агентов. Коммить и пушь только проверенные собственные изменения в текущую ветку.

Target определяет Cargo.toml: сейчас `0.16.1+mc26.3`. Java находится в
`minecraft-src/minecraft/src/net/minecraft`, manifest — рядом с исходниками.
Перед gameplay-работой проверь совпадение версии, а не верь номеру в этом тексте.

## Проверенные факты, которые нельзя снова объявить блокерами

1. `/data` уже зарегистрирован, block/storage поддерживают get/merge/remove/modify,
   источники from/string/value. SNBT arguments и общие NBT paths существуют.
   Их не нужно писать заново. Наше ревью исправило wildcard counts, обработку
   несовместимых значений в числовых массивах, обход неоднородных ветвей,
   глубину merge-source, Java float formatting и UTF-16 substring.
   Тесты путей вынесены в `vecta-utils/src/nbt/path/tests.rs`.
   Исправлен и общий float formatter в `vecta-utils/src/java.rs`: удаление
   trailing zeroes должно менять decimal exponent, сохраняя значение числа.
2. Контейнерный comparator input УЖЕ есть. Читать
   `behavior/blocks/redstone/diode/comparator.rs`, `container/barrel_block.rs`
   и `world/block_updates/mod.rs`. `world/redstone.rs` — лишь torch bookkeeping.
   Не добавлять параллельный расчёт fullness в общий get_signal.
3. `Player::server()` уже возвращает сервер. Для placement не нужна новая
   ссылка World→Server. `BlockEntityLoadContext` — ранее согласованный явный
   read-only контекст. Передача такого контекста через существующий player caller
   не требует заново выбирать всю архитектуру. Playerless callers проверять отдельно.
4. `BlockEntity::load_with_owned_components` сначала кодирует/декодирует compound,
   затем вызывает loader без Result. Ошибка bridge происходит ДО мутации BE;
   не добавлять фиктивный rollback с проигнорированной ошибкой восстановления.
5. onlyOpCanSetNbt уже получен extractor-ом для 49 типов. Гейт и тесты есть;
   B-06-05 закрыт. Не писать ручной список шести типов и не повторять extraction.
6. Root CustomName/lock на обычном disk reload соответствуют Java. Не удалять их
   и не вызывать item applyComponents на обычной загрузке. B-06-04 закрыт.
7. Отсутствие Rust consumer или большой объём задачи — работа реализации,
   не внешний блокер. Настоящие блокеры — отсутствующий эталон/output либо
   неясный ownership/error/architecture contract после проверки.

## Порядок выполнения

Завершай небольшие проверяемые сквозные срезы внутри большой очереди. После
готового среза — узкие проверки, review diff, commit/push и сразу следующая задача.
Не останавливайся ради формального отчёта после одной карточки. Если одна ветка
зависит от отсутствующего вывода extractor или решения, запиши точный вопрос и
продолжай независимые ветки. Молчание пользователя не разрешает угадывать.

### 1. Закончить живую загрузку block data: G021/G022

- Провести явный BlockEntityLoadContext в item custom-data placement. Изучить
  `BlockItem::place_with`, `Player::server`, CommandSource, Lectern.loadAdditional
  и TypedEntityData.loadInto. Соблюсти custom data → components → setPlacedBy.
- Авторизованная установка кафедры с unresolved written book должна разрешать
  storage/score/selector текст с позиции блока, без source entity, как Java.
- Не держать inventory lock во время resolver/callbacks. Сохранить snapshot
  предмета до callbacks, гейт OP+creative, type matching и playerless denial.
- Disk load до setLevel оставляет книгу unresolved; activation сама по себе
  не должна повторно разрешать уже сохранённые страницы.
- Протестировать размещение, изменение storage, сохранение/загрузку книги,
  неавторизованного игрока, отсутствие BLOCK_ENTITY_DATA и несовпадение типа BE.
- G021/W06-105: настоящий region/chunk write/read и unload/load через существующий
  I/O harness. Имя, lock, items, explicit/unknown components и функциональное меню
  после восстановления. Factory decoder не выдавать за region I/O или restart.
- Дополнить rejected `/data` cases, NBT arrays, неоднородные wildcard paths и
  source/target одного storage. Не переписывать уже проверенные методы.

### 2. Общий entity NBT load/save и living state: G024/W06-116…120 + G022 entity

- Прочитать Entity.load/saveWithoutId, LivingEntity.loadAdditionalSaveData,
  EntityDataAccessor.setData, существующие typed PersistentEntity и factory.
- `Entity::apply_spawn_data` уже читает много base полей. Сначала сверить его:
  переиспользовать подходящие readers, добавить full-load reset semantics и live
  tracking вместо вызова spawn-only partial update на уже добавленной entity.
  Отсутствие всего base loader с нуля — неверная постановка задачи.
- Определить границу общего base loader и subtype loader по текущей архитектуре.
  Если ownership/lifecycle действительно неясны, задать вопрос с проверенными
  вызовами и вариантами; не коммитить интерфейс, который молча пропускает поля.
- Реализовать необходимые base поля: положение/движение/поворот, разрешённые
  флаги и timers, tags, имя/видимость, gravity/silent/invulnerable и другие поля
  фактического Java26.3-контракта. Не копировать список из памяти.
- Обновление живой entity согласовать с chunk/entity spatial indices и dirty
  sync. Простое изменение EntityBase.position в обход world tracking недопустимо.
- Health/HurtTime/DeathTime/effects/equipment: persistence, damage cooldown,
  respawn и cross-world recreation, отсутствие повторной смерти/дропа.
- `data entity`: get/merge/modify/remove через общий полный loader; vanilla запрет
  изменения player data, сохранение исходного UUID при setData. Проверить
  passengers/attachments и subtype данные перед объявлением provider готовым.
- Подключить `/execute store … entity` к той же реализации, убрать старый TODO
  только после working consumer. Не дублировать декодирование между командами.
- Tests: реальная entity в World, command mutation → lookup/broadcast → save/load,
  health/effects/equipment, попытка изменения игрока и UUID, rejected payload.
- G022 `modify … compute`: сверить ContextFloatProvider/ContextIntProvider и
  LootContextSources с текущими loot number providers. Реализовать необходимые
  типизированные providers и реальный command consumer в одном срезе; worldgen
  IntProvider не подменяет loot context. Если нужны новые registry данные,
  обновить extractor и получить produced output. Проверить float/integer result,
  source context, seeded RNG и ошибки, а не только регистрацию узла команды.

### 3. Контейнеры раздатчика и экипировка: G025/G026/W06-121…130

- Имя, lock, contents, loot table/seed, components: item → placement → disk → load.
- Сохранить различия dropper/dispenser, правильный порядок dispense dispatch,
  остатки вёдер/бутылок, полный контейнер и отказ без расхода/дюпа.
- Damageable/Unbreakable, durability break, slot callbacks и equip hooks сверить
  с EnchantmentHelper/ItemStack, не ограничиваться уменьшением damage field.
- Equip на игрока и mobs через capability, занятый слот и несоответствующий тег,
  drop chances/persistence. Отсутствие wolf/horse не скрывать fake entity.
- Проверить сохранённый механизм выбора слота: seeded и runtime randomness
  применять согласно AGENTS и Java, не разрушить общий server named RNG.

### 4. Печи от результата до автоматизации: G027/G028/W06-131…140

- Furnace/blast furnace/smoker: recipe selection, fuel/remainders, cook/burn timers,
  recipe change и reset, отсутствие рецепта, input/output limits.
- Result slot: XP и recipe-used bookkeeping, extraction рукой/shift-click/hopper,
  закрытие меню, разлом, сохранение/загрузка и недопущение повторного начисления XP.
- Sided slots и canPlace/canTakeThroughFace с реальными предметами, а не тестом
  возвращаемых номеров. Проверить автоматизацию сверху/сбоку/снизу и blocked output.
- Два локальных server-backed зрителя видят один container; изменения одного
  и автоматики доходят второму. Устаревший пакет не оживляет уже выданный результат.
- NBT ItemStack components и имя/lock сохраняются. Comparator fullness и dirty
  notifications использовать существующие, не добавлять вторую систему.

### 5. RecipeDisplay → recipe book → крафт: G031…G037/W06-151…185

- Разделить CUpdateRecipes (уже сделан для stonecutter) и recipe-book/display
  пакеты по фактическим Java codecs. Не возвращаться к прежнему ложному
  блокеру «для stonecutter нужны все RecipeDisplay».
- Typed RecipeDisplay/SlotDisplay, holder sets, property sets и регистрационный
  порядок. Не вписывать IDs руками и не парсить vanilla JSON в runtime.
- ServerRecipeBook: known/highlight/settings, initial sync, add/remove recipe,
  сохранение игрока, disconnect/rejoin и dimension change.
- Serverbound recipe selection/settings/place handling: ownership меню, ID,
  устаревшее/закрытое окно, отрицательные/неизвестные IDs без panic.
- Placement: вернуть предметы из grid, учесть inventory capacity, подобрать
  ingredients, place/max craft, компоненты/stackability, ghost response при нехватке.
- Реальный crafting result и remainder, ITEM_CRAFTED/used recipe callbacks,
  shift-click batches и разбитый/закрытый контейнер без дюпа.
- Unlock triggers по supported actions. Не объявлять весь recipe book готовым
  только по регистрации пакетов или preview без перемещения предметов.
- Tests: packet→handler→menu→inventory→reply; multicraft, partial inventory,
  component-distinct ingredients, reconnect и отсутствие лишнего повторного sync.

### 6. Рост растений и мир: G011 partial + соответствующие worldgen callers

- Sapling Bonemealable + TreeGrower: существующие extracted features, biomes,
  stage transition, 2×2 где положено, выбор варианта, rollback при неуспехе.
- Не создавать отдельные деревья вручную и не дублировать feature placements.
- Underwater bonemeal: face/biome/tag selection, coral/wall coral/seagrass,
  water source/survival и RNG call order по Java.
- Water evaporation: позиционный WATER_EVAPORATES, fizz/large smoke, отказ или
  расход ведра по правильной ветке. Проверить ручной и dispenser consumers.
- Отдельные tests на успех/отказ/границу высоты/неподходящую опору. Seeded worldgen
  сравнивать с доступным Java эталоном; не заменять детерминизм случайным RNG.

### 7. Облако и туманное зелье: G005 затем G004/W06-016…025

- AreaEffectCloud entity: radius/wait/duration/reapplication delay, radius-on-use,
  radius-per-tick, owner, effects и entity metadata из current generated schema.
- Targets, immunity, distance, cooldown между повторными наложениями, instant
  effects и периодические effects по Java. Один living capability для игроков/mobs.
- Persistence и восстановление owner attribution, исключить второй burst на reload.
- Lingering projectile: создание настоящего облака при hit, transfer potion
  components/color, level events и удаление projectile ровно один раз.
- Рука и dispenser используют один consumer. Tests на попадание/радиус/время,
  несколько целей/повторное наложение/save-load; spawn-only не означает closed.

### 8. Стрелы, лук и арбалет: G041…G045/W06-201…225

- Сначала AbstractArrow foundation: flight/drag/gravity, block/entity collision,
  in-ground/life, pickup rules, owner, damage, critical/piercing/knockback и save/load.
- Arrow/SpectralArrow/tipped arrow: компонентный payload, эффекты/длительность,
  извлечение из земли, potion data и отказ добавления entity без расхода боеприпаса.
- Bow release/charge/ammo/infinity/durability; creative/survival, offhand ammo.
- Crossbow charge/loaded projectiles/multishot/piercing, firework после его foundation.
  Не подключать регистрацию оружия к заглушке projectile.
- Dispenser projectile consumers используют эти же сущности/launch contract.
- Tests через реальные use/release/dispense и tick/hit, затем save-load полёта,
  armour/enchantments, pickup и повторный hit без двойного урона/дропа.

### 9. Фейерверк, огненный шар и wind charge: G046…G048/W06-226…240

- Firework: lifetime, flight/explosion components, owner/attached entity,
  collision/damage, persistence и клиентское событие взрыва.
- SmallFireball: shoot/hit/ignite, owner/mobGriefing/damage immunity; затем
  FireCharge dispenser registration и реальные sound/events/расход.
- WindCharge: доступный explosion/knockback engine и взаимодействия с блоками
  по 26.3; общая physics foundation, не локальный push всем целям без правил.
- Сохранить существующий TNT engine и attributed damage; не подменять
  отсутствующую общую взрывную семантику только визуальным событием.
- Игрок/раздатчик и failed insertion, block/entity hit и unload/reload.

### 10. Пчёлы, ульи, мёд и ножницы: G012/G013 partial + реальные Bee foundations

- Bee lifecycle и beehive occupant data до объявления hive release рабочим.
  Соблюсти существующие entity factories и extracted типы/metadata.
- Shears: полный hive имеет приоритет над entity AABB, honey reset, smoke/anger,
  release rules, durability и правильный HARVEST_BEEHIVE typed loot context.
- Bottle consumer: honey и release, стак/остаток/полный dispenser. Сохранить
  уже сделанные вода source/flowing и отдельный feedback при eject remainder.
- Поводок снимается с первой подходящей цели даже без Shearable; failure не
  выбрасывает ножницы. Player/Blocks sound source остаётся различным.
- Persistence занятого улья, blocked exit, день/ночь/дождь по Java, harvesting
  вручную и автоматикой с реальным состоянием BE/сущностей.

### 11. Карты целиком: G081…G083/W06-401…415, затем G023/W06-111…115

- Map saved data/ID allocation и реальное persisted storage, dimension/center/
  scale/locked/tracking, holders/decorations и update packet deltas.
- Карта в руке обновляет цвета/декорации по фактическому миру; exploration и
  clone/scale/lock через существующие crafting/cartography paths.
- Frame map linkage: add/remove framed map, ограничение markers, снятие предмета,
  destruction/unload/load, смена карты и отсутствие висящих decorations.
- Только затем закрывать W06-115. Прочие frame NBT/support/rotation/Fixed/Invisible
  уже существуют — подтвердить и сохранить, не писать рамки заново.
- DropChance/gamerules/creative на заполненной и пустой рамке сверить с Java;
  не считать mock manager подтверждением реального persisted map linkage.
- Cartography menu/result/remainder, второй зритель, shift-click, invalid map ID.

### 12. Сквозная автоматизация: G018…G020/W06-086…100

- Crafter foundation с disabled slots, recipe matching, trigger/tick и настоящим
  результатом/remainders; не регистрировать пустой hook вместо craft.
- Brewing stand: fuel/ingredient/recipe timer, bottle components, прерывание,
  output changes, sided access и persistence.
- Hopper → furnace/brewing/crafter → hopper/container: transfer cooldown,
  slot rules, full output и unload/load. Суммарные предметы не меняются без
  реального recipe/remainder. Меню и comparator видят актуальное состояние.
- Уничтожение контейнера и pending tick не повторяют craft/loot/drops.
- Фиксировать проверяемые цепочки реальных блоков, не набор независимых helpers.

Если вся эта очередь реально завершена, продолжай actual route существующего
`NIGHT_TASK_06.md`: G029/G030, G038…G040 и следующие. Завершение перечисленного
не равняется полной vanilla; отчёт должен показывать точный поддержанный scope.

## Качество и ограничения

- AGENTS обязателен. Java/data до gameplay-кода. Не угадывать registry values,
  IDs, error handling и архитектуру. Неясность проверять по callers и задавать
  узкий вопрос с exact source/path/log. Не переносить ограничения vanilla на
  Vecta domains/permissions/admin architecture без отдельного основания.
- Общий foundation и его первый настоящий consumer коммитить вместе. Не
  добавлять todo!/паники/no-op capabilities, fake data, mocked vanilla manager.
- Не править generated Rust или extracted JSON руками. SteelExtractor checkout
  `D:\CodexBuilds\SteelExtractor`, output `run\steel_extractor_output`.
  Нужны только реально произведённые файлы. JAVA_HOME может указывать на17 при
  PATH Java25; выбирать установленный JDK25 явно. После повторного сбоя одной
  категории остановить зависимую работу, не перебирать бесконечные flags.
- Сохранять server-global persisted RandomSequences, default-world seed,
  стабильный последовательный порядок игровых тиков миров; фоновые I/O/worldgen
  не превращать в синхронное ожидание на game tick.
- Не вводить World→Server pointer. Использовать уже согласованные явные контексты.
- Не добавлять production unwrap/expect, borrow-checker clones, unsafe вне
  разрешённого keyed Downcast, отключения линтеров и игнорирование ошибок.
- Модуль под растущую систему и отдельные tests; не наращивать один гигантский файл.
- Не менять pagefile, VM, toolchain, профили Cargo или чужие процессы. EULA не
  принимать; настоящий сервер/клиент не выдавать за запущенный без доказательства.

## Проверки и журнал

Все тяжёлые Cargo команды последовательно с `-j 1`:

```powershell
$env:CARGO_TARGET_DIR='D:\CodexBuilds\VectaMC'
$env:CARGO_INCREMENTAL='0'
$env:TMP='D:\CodexBuilds\tmp'
$env:TEMP='D:\CodexBuilds\tmp'
```

Baseline уже задокументирован в review. До первого изменения повторно весь
workspace не гонять. После изменения — минимальные содержательные regression
tests; не тесты ради счётчика. Для общего foundation — связанные consumers.
На готовых крупных этапах и перед передачей:

```text
cargo fmt --all --check
cargo test --locked --workspace -j 1
cargo clippy --locked --workspace --all-targets --all-features -j 1 -- -D warnings
python -m unittest discover -s scripts
python scripts/audit_vanilla.py
```

Логи `D:\CodexBuilds\logs`, отдельный файл на прогон. OOM/нехватку диска/ошибку
записать с exact log; type-check не выдавать за runtime tests. Не запускать
параллельные компиляции. Не повторять полную зелёную проверку без новой причины.

Обновляй `planning/night-06/progress.md`, `checkpoint.md`, `blockers.md` после
существенных срезов. Ложные блокеры снимать с конкретным caller/test, а не
переносить в следующий отчёт. Открытый срез не называть полностью closed.

Итог сеанса — `planning/night-06/report-next-05.md` и ОБЯЗАТЕЛЬНО текстовый ответ:
ветка/HEAD/push, коммиты, что изменилось для игрока, actual проверки с числами и
exit codes, ограничения, незавершённая работа и точное следующее действие.
Клиент/Linux/два сетевых игрока/restart/Mojang differential перечислять отдельно
как выполненные или не выполненные. Указать хэши двух чужих файлов.

Не останавливаться после «готово G023» или одного тестового коммита. Продолжать
код из независимой очереди, пока позволяет сеанс; под конец оставить проверенный
checkpoint. Не обещать всю ваниллу за одну ночь и не считать число тестов её процентом.
