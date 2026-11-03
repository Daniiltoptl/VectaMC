# Сеанс 07: единый loot context, compute и большая очередь vanilla

> Актуализация: срезA (единая typed ContextMap, migration всех текущих callers,
> metadata16keys/31sets) реализован, см. planning/night-06/report-next-07.md.
> Не делай его повторно. Следующий consumer — providers/compute. Новый вопрос
> о владении общим domain read context решён: пользователь одобрил shared Arc
> scoreboard/storage и отдельный read context без World→Server ссылки.
> Оба архитектурных решения согласованы, повторно не спрашивать.
> Actual catalog:28float+23int=51тип. Первый код `2a0ef1daf`, workspace4304/0/18.
> Исходный объём задания ниже сохраняется для оставшейся очереди.

Работай в `C:\Users\My comp\Desktop\Vecta`, ветка `codex/night-vanilla-03`.
Продолжай actual tip вместе с ревью Codex, не откатывайся к историческим SHA.
Задача — писать и проверять код до доступного лимита сеанса. Завершение одного
helper или одной карточки не является завершением всего задания.

Код ревью `a91820a33`; продолжать actual tip вместе с docs. Проверенный workspace
**4299 passed /0 failed /18 ignored**, Clippy/fmt exit0, Python4,
audit3105/49/230. Cargo manifest warning mimalloc/dhat сохранён; см. review.

## Начало и объём

Прочитай AGENTS.md, `planning/reviews/night-06-session06-codex.md`, checkpoint,
progress, blockers, route, **полностью `WORKBUDDY_CONTINUE_06_05.md`** и файл06.
Файл05 задаёт подробные 12 направлений; `NIGHT_TASK_06.md` содержит 600 карточек.
Этот запрос уточняет первую очередь и факты ревью, а не сокращает задание.
Далее читай Java, generated/extracted данные и callers по текущему срезу.

Проверь branch/status/remote и неизменность чужих файлов. Не трогай и не stage:

- `planning/night-01/report.md`, SHA256
  `06680CBC971245B70C990D95C906F0F1E3685EFFDE61C13D562664B4B6B05222`;
- `WORKBUDDY_START_06.md`, SHA256
  `48430DC1D56AF1E679E0A81411518002B8C406B3F27648AD1A197DF174AFAD5A`.

Не запускай дополнительных агентов. Не меняй систему, pagefile, toolchain,
Cargo profiles, VM и чужие процессы. Не принимай EULA. Не делай force push,
reset/clean/stash чужой работы, merge в main или массовую смену зависимостей.

Текущий authoritative target — suffix workspace Cargo.toml; сейчас26.3.
Сверь `.vecta-reference.json`, а не версию из старого отчёта. Известный checkout
SteelExtractor: `D:\CodexBuilds\SteelExtractor`, output внутри
`run\steel_extractor_output`. Java25 выбирай явно через JAVA_HOME, если Gradle
берёт Java17. После повторных однотипных ошибок не перебирай бесконечно варианты:
покажи точную задачу, лог и недостающий produced файл. Ручных registry IDs не надо.

## Уже согласованное архитектурное решение — выполнять

Пользователь поручил Codex выбрать лучший путь. Выбран **единый типизированный
ContextMap внутри существующего LootContext с миграцией всех текущих loot
callers**. Это разрешённый следующий архитектурный срез. Не спрашивай повторно,
можно ли переходить на param map. Не делай вторую параллельную контекст-модель
специально для compute. Сохрани общий seeded RandomSource и Vecta domains.

Сначала спроектируй безопасный по lifetime и типам интерфейс по реальным
потребителям. Не предрешай внутреннее хранение до чтения кода. Нельзя создавать
зависимость registry→core, обходить lifetime через unsafe, Any/TypeId или clone
ради borrow checker. Используй существующие capability/read contexts и keyed
downcasting, если он действительно нужен. Ключи extension-параметров должны
иметь путь расширения, а значения — проверяемый тип и время жизни.

## A. ContextMap и миграция существующего loot — первый обязательный срез

1. Составь краткую матрицу текущих LootContext полей, Java LootContextParams,
   обязательных/допустимых param sets и реальных callers. Это рабочая матрица,
   а не замена реализации отчётом.
2. Проверь ContextMap/Builder, LootParams/Builder, LootContext/Builder и
   LootContextParamSets в Java26.3. Различай отсутствующий параметр и нулевое
   значение; повтори validation обязательных и недопустимых параметров.
3. Введи одну типизированную модель параметров, пригодную для всех текущих
   consumers. Сначала перенеси имеющиеся поля без потери семантики.
4. Мигрируй все constructors/callers, conditions, entries, functions и readers.
   Старый flat context не оставляй вторым источником истины.
5. Проверь block loot, entity loot, container fill, command loot, инструменты,
   enchantment callers и прочие реально найденные пути. Не считай список
   исчерпывающим без поиска всех сборок контекста.
6. Seeded RNG остаётся тем же mutable потоком; не копируй его состояние и не
   создавай новый генератор при чтении параметра или построении derived context.
7. Сохрани server-global persisted named RandomSequences и стабильный порядок
   последовательных игровых тиков миров. Domains не смешивай.
8. Добавь значимые проверки required/allowed/missing, несовместимого типа,
   контекста без entity, shared RNG call order и существующего seeded loot.
9. Миграция должна компилироваться и иметь реальные consumers. Не коммить
   foundation с фиктивными значениями, отключёнными условиями и пропущенными
   существующими callers. Срез можно разделить на несколько reviewable commits,
   но промежуточные shared контракты должны оставаться рабочими.

## B. ContextFloatProvider / ContextIntProvider и реестры

10. Проверь фактический каталог Java26.3: incoming report перечисляет27float и
    22int реализации. Подтверди список исходниками, а не принимай число за spec.
11. Важное исправление старого ТЗ: существуют реальные реестры
    `CONTEXT_FLOAT_PROVIDER_TYPE` / `CONTEXT_INT_PROVIDER_TYPE` и configured
    `CONTEXT_FLOAT_PROVIDER` / `CONTEXT_INT_PROVIDER`. Inline constant alternative
    не отменяет registry dispatch и resource references.
12. Проверь текущие extracted assets/codegen, затем добавь недостающее через
    SteelExtractor. Копируй только produced нужные файлы. Не переписывай JSON
    и не транскрибируй ID вручную из Java.
13. Сделай typed Rust providers и codec dispatch с понятным extension path.
    Vanilla configured данные компилируй build scripts, не вводи runtime
    datapack JSON loader. Не используй worldgen IntProvider вместо loot provider.
14. Повтори inline/resource формы, defaults, required parameters и validation.
    Отсутствующая registry reference должна давать реальную ошибку.
15. Арифметика: unary/binary/n-ary операции, min/max/average, rounding, modulus,
    quotient, power, sqrt, trig, conversions — по точным Java implementations.
16. Проверь отдельно NaN/Infinity, overflow, отрицательные значения, пустые
    списки, деление на0 и границы преобразований. Не добавляй свои clamps или
    «безопасные defaults», если Java делает иначе.
17. Случайные providers: диапазоны, веса, порядок RNG calls и seeded результат.
    Недостаточно проверить только допустимый диапазон.
18. Contextual providers: entity/position/target, storage NBT, score,
    enchantment level, environment attributes, conditions. Для каждой зависимости
    сначала найди реальный существующий read API; не объявляй систему отсутствующей
    только из-за другого имени метода.
19. Недостающую foundation реализуй вместе с настоящим consumer. Если после
    проверки остаётся неизвестный контракт/архитектура, задай узкий вопрос с
    Java/кодом/логом. Не регистрируй no-op provider под vanilla именем.
20. Если весь каталог требует нескольких завершённых срезов, фиксируй границы
    честно. Не публикуй общий codec, принимающий варианты, которые не вычисляет.

## C. Настоящий `/data modify … compute`

21. Сверь дерево `DataCommands.register`, `ResourceOrIdArgument` и
    `LootContextSources` целиком для этого consumer.
22. Реализуй float/integer argument parsing и resource/inline forms, command
    graph serialization, cursor/errors и completion, где Java это предоставляет.
23. Context sources строят общие param sets COMMAND_COMPUTE_DEFAULT/POSITION/
    ENTITY; это не отдельный ad hoc compute context.
24. Передавай command source read context к storage/score/world/entity через
    разрешённые API, не создавая World→Server pointer и не раскрывая write access.
25. Проверь весь путь parsing→context→provider→NBT result→setData→dirty/sync.
    Сохрани existing rollback, запрет записи в players и single entity target.
26. Проверяй отсутствующие параметры, неправильные targets, неизвестный provider,
    неверный SNBT и арифметические ошибки по Java messages/result contracts.
27. Тестируй реальные команды для storage, block и supported entity, не только
    прямые вызовы evaluate. Include unchanged-result/no-target/error cases.
28. Свяжи command context и seeded random semantics по Java. Не используй
    runtime RNG вместо заданного deterministic потока.

## D. Остальная очередь — продолжать после проверенных A–C

Это полноценная следующая работа, а не список пожеланий. Подробные сценарии
находятся в файле05 и соответствующих группах NIGHT_TASK_06.md.

29. G025/G026: NBT раздатчика, equipment hooks, durability, остатки/remainders,
    busy slots, инвентарь при полном контейнере, gamerules и сохранение.
30. G027/G028: печи, результат/XP/recipes used, hopper automation, fuel/remainder,
    comparator, lit state и unload/reload фактическим I/O.
31. G031…G037: RecipeDisplay foundation, wire payload, recipe book sync,
    crafting/auto placement, stonecutter/loom реальные clients paths. Не считать
    CUpdateRecipes заменой recipe book и не возвращать старый ложный blocker loom.
32. Растения: TreeGrower/саженцы и bonemeal, вода/кораллы, реальные configured
    features и их данные. Случайность/порядок worldgen — строго seeded vanilla.
33. G005→G004: AreaEffectCloud foundation и lingering potion consumer, owner,
    радиус/длительность/повторное применение, сохранение и события.
34. G041…G045: AbstractArrow/стрелы, подбор, collisions, урон, эффекты, лук,
    арбалет, charging/loading/shooting и dispenser consumers.
35. G046…G048: fireworks, SmallFireball/fire charge и WindCharge. Прямое
    зажигание не заменяет SmallFireball из раздатчика.
36. Bee/ульи/мёд/соты/ножницы: настоящие entity/hive foundations и consumers,
    а не флаги готовности. Проверяй release, occupancy, anger и persistence.
37. G081…G083→G023: maps saved data, tracking, frame linkage, removeFramedMap,
    лимиты и пакеты; не переписывай уже готовые рамки без найденного дефекта.
38. G018…G020: hopper/dropper/dispenser/crafter automation на общих меню,
    containers и redstone, реальные inventory conservation и multi-viewer paths.
39. Оставшийся G024 Brain/locator-bar icon — реальные открытые поля, а не
    «Java их не сохраняет». Перед dependent consumers нужны полные соответствующие
    foundations. Не добавляй сырые фиктивные поля ради round-trip теста.
40. Criteria ITEM_DURABILITY_CHANGED — отдельная остающаяся зависимость:
    проверь имеющийся advancement state/commands и недостающий criteria trigger.
    Наличие ручной команды advancement не означает готовность trigger engine.

Если один срез зависит от отсутствующего внешнего факта, зафиксируй проверенные
источники и узкий blocker, затем делай независимый следующий срез. Не объявляй
весь пакет заблокированным одной карточкой. Не обходи реальный stop rule AGENTS.

## Сохранять исправления ревью

- Living loader имеет Reset/Merge, общий disk/command путь, Living→subtype order.
- Codec numeric conversion отличается от TagValueInput: INT truncates, BOOL
  nonzero; HurtTime/DeathTime — short. Partial list results сохраняют valid entries.
- Optional codec malformed fields могут давать defaults в partial result.
- UUID/BlockPos numeric streams принимают valid prefix при избыточной длине только если
  весь stream numeric. Live entity UUID accessor сохраняет прежним.
- Packed attributes имеют независимые ID/operation/permanent индексы. Старый
  operation entry при смене operation того же ID может продолжать влиять на
  значение; это проверено настоящим Java AttributeInstance, не «почистить».
- Raw NBT effect load не запускает gameplay effect hooks и update/remove packets.
  Нельзя вызывать startSleeping вместо setPosToBed при загрузке состояния сна.
- Combat references resolve living targets во всех мирах текущего domain,
  соблюдая expiry; не возвращать world-local lookup и nonliving attackers.
- Team использует существующий domain scoreboard, payload UUID holder и
  postDataManipulated. Не вводить новую scoreboard/permissions архитектуру.
- Placement context, produced onlyOp metadata, OP+creative gate, custom data→
  components→setPlacedBy, detached initial disk load сохраняются.
- Item break callback выполняется вне inventory lock; player guard общий.

## Проверки, commits и передача

Среда сборки: CARGO_TARGET_DIR=`D:\CodexBuilds\VectaMC`, CARGO_INCREMENTAL=0,
TMP/TEMP=`D:\CodexBuilds\tmp`, тяжёлые команды serial `-j 1`, логи в
`D:\CodexBuilds\logs`. Не запускать параллельно два cargo процесса.

После изменений — narrow meaningful tests, fmt check. На завершённом крупном
срезе: `cargo test --locked --workspace -j 1` и
`cargo clippy --locked --workspace --all-targets --all-features -j 1 -- -D warnings`.
Не повторяй baseline до первого изменения без причины. Проверки окончательного
кода обязательны; type-check не заменяет runtime tests.

Сверяй спорные codecs с local Java; при необходимости допустим standalone
oracle классов без старта MinecraftServer/EULA. Это не полное server comparison.
Сохраняй исходник/вывод oracle вне repo или в осмысленном test fixture и укажи
границы доказательства. Не называй test fixtures настоящим client/restart.

После code commit обнови audit штатным scripts/audit_vanilla.py и выполни
`python -m unittest discover -s scripts`. Старые known Cargo manifest warnings
не скрывай lint suppressions и не лечи сменой allocator/toolchain вне задачи.

Делай reviewable commits и обычный push существующей ветки по проверенным
законченным срезам. Stage только свои файлы. Проверяй оба foreign hashes.
Обновляй существующие progress/checkpoint/blockers; исторические report bodies
не переписывай. Следующий отчёт: `planning/night-06/report-next-07.md`.

Перед исчерпанием лимита оставь checkpoint: actual HEAD/status, незаконченный
код, точная следующая функция/consumer, выполненные команды и логи. Не оставляй
только файлы: **обязательно ответь текстом** — поведение игрока, commits/push,
точные проверки, partial/blockers и следующий шаг. Не называй ядро полной
ваниллой и не считай число тестов процентом готовности.
