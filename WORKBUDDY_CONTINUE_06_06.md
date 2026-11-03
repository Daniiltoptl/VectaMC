# Большой сеанс 06: закончить entity foundation, затем продолжать весь пакет

Рабочий каталог `C:\Users\My comp\Desktop\Vecta`, ветка `codex/night-vanilla-03`.
Продолжай actual tip, исторические SHA не являются указанием откатиться.
Выполняй код до доступного лимита сеанса, переходя между проверенными срезами.

Код ревью `1a8bb2457`; продолжать actual tip вместе с docs, не откатываться.
Последний проверенный workspace после ревью: **4288 passed /0 failed /18 ignored**,
Clippy/fmt exit0, повторные entity tests7, Python4, audit3105/49/231.
Clippy сохранил прежний Cargo warning mimalloc/dhat. Фактические проверки и
ограничения брать из review; baseline повторно до первого изменения не запускать.

## Контекст и объём

Прочитай AGENTS.md, `planning/reviews/night-06-session05-codex.md`, актуальный
checkpoint и **полностью `WORKBUDDY_CONTINUE_06_05.md`**. Его очередь из12
направлений и более150 существующих карточек остаётся действующей. Этот файл
уточняет факты после сеанса05 и меняет первый приоритет; он не сокращает
очередь до одной задачи. Затем читай нужные группы `NIGHT_TASK_06.md`, код,
Java26.3, callers, produced data и tests по мере исполнения.

Сначала status/log/remote и два foreign hashes. Не трогай и не stage:

- `planning/night-01/report.md`:
  `06680CBC971245B70C990D95C906F0F1E3685EFFDE61C13D562664B4B6B05222`;
- `WORKBUDDY_START_06.md`:
  `48430DC1D56AF1E679E0A81411518002B8C406B3F27648AD1A197DF174AFAD5A`.

Не запускать дополнительных агентов. Не менять систему, pagefile, toolchain,
Cargo profiles, VM и чужие процессы. EULA не принимать.

## Что уже сделано и сохраняется

- Placement передаёт BlockEntityLoadContext и разрешает книгу авторизованной
  кафедры. Тип, OP+creative gate, порядок custom data→components→setPlacedBy,
  snapshot предмета вне inventory lock и detached первоначальный disk load
  сохраняются. Не писать этот путь заново.
- RegionManager действительно пишет/читает `.mca`. Исправлены оба места потери
  explicit components: save_without_metadata и load_with_components. Имя/lock
  в корневом NBT соответствуют Java; applyComponents на обычном reload не нужен.
- `/data entity` и `/execute store entity` зарегистрированы, общий base loader
  существует. Это НЕ завершённый LivingEntity.load: см. обязательный этап ниже.
- Ревью перенесло запрет записи в игроков в общий load_command_data. Нельзя
  вернуть обход через execute store или другой caller.
- `execute store entity` принимает одного target, как DataCommands provider.
  Player rejection внутри callback не проваливает результат вложенной команды.
- Numeric NBT читается по TagValueInput: boolean — NumericTag.byteValue !=0,
  int/short имеют свои сужения. Codec.BOOL не заменяет getBooleanOr.
- Vec3/Vec2 допускают numeric lists и primitive arrays. Учти ValueInput partial
  results: Util.fixedSize для слишком длинного списка отдаёт допустимый prefix;
  TagValueInput.read принимает error.partialValue. Короткий список даёт default.
- При command Motion load выставляется velocity sync. invulnerable_time включён
  в command snapshot и загружается с reset/default semantics.
- Entity command tests теперь в `server/tests/data_entity.rs`, shared command
  fixture — в data_command.rs. Не возвращать всё в один растущий файл.

## Этап A — полный supported living load и реальное сохранение: G024/P0

Закрой этот фундамент перед новыми consumers, которые рассчитывают на полный
load_command_data. Отсутствие Rust loader — задача реализации, не внешний блокер.

1. Сделай точную матрицу Java Entity.load + LivingEntity.readAdditionalSaveData
   → Vecta state → export → command load → disk save/load → dirty/network sync.
   Это рабочая матрица реализации, не повод завершить сеанс отчётом вместо кода.
2. Сейчас save_command_nbt экспортирует Health/HurtTime/DeathTime/absorption,
   attributes, active_effects, equipment и прочее, но load_command_data вызывает
   только subtype loader. Sheep.load_additional→load_mob не загружает Health
   и общий living state. `/data merge … {Health:1.0f}` нельзя объявлять рабочим,
   пока значение только попадает в compound, но не меняет живую сущность.
3. Реализуй один living read path для существующих сущностей. Порядок Java:
   absorption → attributes apply → active effects → clamped health → hurt/death
   timers → FallFlying → sleeping state → поддержанный brain/reference state →
   combat attribution → equipment → locator/impulse state. Сверь весь метод,
   включая default/absent/invalid значения, а не копируй этот перечень вслепую.
4. Attributes: typed registry refs, base и permanent modifiers, dirty calculation
   и CUpdateAttributes. Непостоянные modifiers, equipment modifiers и active
   effect modifiers имеют разный жизненный цикл. Не собирать вторую AttributeMap
   из ручных vanilla констант и не терять существующие transient modifiers.
5. Effects: разобрать настоящий MobEffectInstance codec, duration/amplifier,
   ambient/particles/icon, hidden_effect chain, infinite duration и flags.
   Не подменять load вызовами gameplay add/removeEffect без сверки: их hooks,
   modifiers и network side effects отличаются от readAdditionalSaveData.
6. Equipment: отсутствие compound сбрасывает slots по Java. Сейчас load_equipment
   вызывается из load_mob; проверь порядок Living→Mob→Ageable→Animal→subtype и
   убери двойную загрузку, если общий living consumer становится владельцем.
7. Timers/death: HurtTime/DeathTime/invulnerable_time, здоровая/умирающая сущность,
   повторный load, отсутствие повторного death loot и damage callbacks. Новая
   жизнь игрока не сохраняет старые death guards. Player command mutation запрещена.
8. Sleeping/rotation/impulse: восстановить state и позицию/pose в правильном
   порядке; отдельные saved grace/impact fields не заменять gameplay max/grace
   helper-ом, который изменит значение вместо восстановления.
9. Combat attribution: UUID reference и unloaded target, memory timers и kill
   credit. Сначала прочитать текущие EntityReference/WeakEntity/history consumers.
   Если ownership действительно неясен — точный вопрос с кодом и вариантами,
   не fake entity и не отбрасывание reference при отсутствии online target.
10. Disk pipeline: `chunk_saver/storage/entities.rs::entity_to_persistent`
    сериализует дополнительные поля отдельно от typed base. Сейчас living data
    из command snapshot не становятся автоматически disk payload. Пройти
    EntityRegistry load factories, реальный RegionManager и cross-world recreation.
    Сохранять health/effects/attributes/equipment/timers, где Java их сохраняет.
11. Сверить command dump с saveWithoutId, не выдавать его за полную serialisation
    foundation. Для разных consumers применять нужный набор полей без дублирования.
12. Тесты должны вызывать команду на живой сущности, наблюдать state, combat или
    movement outcome, а затем save/load. Проверить Health до/после max-health
    attribute, смену effects и hidden chains, remove с default, armor equipment,
    cooldown, смерть/respawn и два последовательных восстановления.

Если для реально поддержанной семантики отсутствует обязательная foundation,
назови точный незавершённый scope. Не объявляй B-06-06 закрытым на основании
одной овцы с CustomName/NoGravity и не расширяй downstream на ложном контракте.
Необходимое архитектурное решение спрашивать после проверки, независимые срезы
очереди при этом выполнять. Размер задачи сам по себе не требует согласования.

## Этап B — закончить entity accessor, а не только зарегистрировать ветки

- Team существует как domain-scoped Scoreboard state в `scoreboard.rs`:
  team/add_holder_to_team/holder_team_name доступны. Отсутствие `/team` или
  scoreboard UX-команд не делает NBT Team невозможным. Сервер уже есть у
  CommandSource; новой World→Server ссылки не нужно.
- Сверь Team load order и UUID behavior с Entity.load и EntityDataAccessor.setData.
  В Java UUID загружается до Team, затем accessor восстанавливает исходный UUID.
  Проверь payload, меняющий оба поля; простое игнорирование UUID может менять
  побочные результаты Team. Не менять live UUID index без проверки ownership.
- Entity.saveWithoutId не обязан сериализовать Team обратно: прочитать фактический
  save method/NbtPredicate. Не добавлять поле в dump только ради симметрии load.
- Разделение доменов и существующая scoreboard persistence сохраняются. Нужен
  общий настоящий consumer для data и execute store, не ещё один no-op hook.
- postDataManipulated: учесть Java default и существующие overrides. До появления
  Villager не писать фиктивного Villager; подготовленный общий hook не должен
  скрывать неполный load текущих поддержанных сущностей.
- Vec/list codecs: проверить partial decode, numeric arrays, invalid entries,
  короткие/длинные списки и numeric type conversion по реальным codecs. Не путать
  Codec.INT с NumericTag.intValue или Codec.BOOL с NumericTag.byteValue.
- Сверить NaN/Infinity, normalize/clamp rotation, valid/invalid position и движение
  в неактивный destination chunk с существующим EntityMoveError contract. При
  неясной политике ошибки/владения спрашивать, не добавлять silent fallback.
- Tests: player guard через оба caller, одиночный target и selector graph,
  Team existing/missing, UUID+Team, live spatial query после смены chunk,
  dirty velocity/metadata, rejected payload и повторное применение данных.

## Этап C — compute, затем весь оставшийся большой маршрут

`modify … compute` реализуется одним законченным срезом: typed
ContextFloatProvider/ContextIntProvider, LootContextSources, SNBT/codec parsing
и реальные from-context command consumers. Это inline dispatch из Java, не
доказанная потребность в новой extractor registry. Проверять конкретные types.
Не копировать worldgen IntProvider вместо loot context provider.

После проверенных A/B/C продолжать направления3–12 `WORKBUDDY_CONTINUE_06_05.md`:

1. G025/G026 — NBT dispenser, damage/durability/equip callbacks и persistence.
2. G027/G028 — печи, result/XP/recipes_used, side slots и hopper transfers.
3. G031…G037 — displays, recipe book, packets, place/max craft, unlock/callbacks.
4. G011 partial — TreeGrower, saplings, underwater coral/biome/tag/face selection.
5. G005→G004 — AreaEffectCloud и lingering potion с working effect lifetime.
6. G041…G045 — AbstractArrow, обычные/спектральные/tipped arrows, bow/crossbow.
7. G046…G048 — firework, SmallFireball/fire charge и wind charge.
8. Bee/hive — release, honeycomb HARVEST_BEEHIVE context, bottle/shears priority.
9. G081…G083→G023 — persisted maps, packets, frame linkage и cartography.
10. G018…G020 — настоящий crafter, brewing stand и сквозная автоматизация.

Подробные acceptance cases и dependencies уже раскрыты в327-строчном файле05
и `NIGHT_TASK_06.md`. После завершения продолжай существующий route. Закрыть
одну группу и остановиться с доступным временем — не исполнение большой очереди.

## Исполнение, проверки и передача

Сохранять: исправления `/data` paths/UTF-16/float formatter, OP gates, persisted
server named RNG/default-world seed, стабильный последовательный world game tick,
существующие permissions/domains, dispenser bucket/shears/leash/sound fixes.

AGENTS обязателен: без stubs/manual IDs/generated Rust edits/ручного extracted
JSON/runtime datapack loader, production unwrap/expect, borrow-checker clones,
необоснованного unsafe и подавления настоящих ошибок. При необходимости output
обновить local SteelExtractor и получить produced файл. JDK25 для Gradle выбирать
явно; повторные loopback failures не обходить бесконечным перебором flags.

Все тяжёлые Cargo проверки последовательно с `-j 1` и прежним окружением:
`CARGO_TARGET_DIR=D:\CodexBuilds\VectaMC`, `CARGO_INCREMENTAL=0`,
`TMP=TEMP=D:\CodexBuilds\tmp`. Последние проверенные числа брать из review,
baseline до первого изменения не повторять. Узкие tests после среза, полный
workspace и Clippy/fmt на крупных этапах и перед передачей; scripts tests/audit.
Фиксировать реальную команду/exit/log, type-check не называть runtime tests.
Cargo warning mimalloc при all-features связан с dhat cfg; default allocator
не удалять и не подавлять линтер ради бесшумного отчёта.

Commit/push проверенные срезы текущей ветки и продолжать. Актуализировать progress,
checkpoint, blockers и audit с честными статусами. Не плодить отчёты вместо кода.
Итог — `planning/night-06/report-next-06.md` И текстовый ответ: HEAD/коммиты/push,
изменение поведения, проверки, ограничения и точное следующее действие. Клиент,
Linux, два сетевых игрока, live restart и Mojang comparison перечислять отдельно.
Не выдавать region fixture или внутренний packet capture за клиентскую сессию.
