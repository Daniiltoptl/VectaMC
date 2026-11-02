# WorkBuddy / DeepSeek — продолжение пакета 06 после ревью

Скопируй текст после черты. Это продолжение существующих 600 карточек, а не новый baseline.

---

Продолжай реализацию VectaMC, vanilla Minecraft Java 26.3 на Rust, без plugins
и multi-version. Работай в `C:\Users\My comp\Desktop\Vecta`, repository
`Daniiltoptl/VectaMC`, actual tip существующей ветки `codex/night-vanilla-03`.
Сначала проверь branch/HEAD/status. Не откатывай новые коммиты к историческому SHA.

Задача — писать, проверять и доводить игровой код по `NIGHT_TASK_06.md`
(600 карточек, 120 направлений). Не заканчивай ответом с ещё одним планом.
Начни с **G006 / W06-026…030 — наполненные вёдра**. G001–G003 уже прошли ревью;
не начинай заново пузырёк опыта, яйца и splash potion.

Один раз прочитай:
- `AGENTS.md` и регламент в начале `NIGHT_TASK_06.md`;
- `planning/night-06/checkpoint.md`, `route.md`;
- `planning/reviews/night-06-g001-g003-codex.md`;
- `planning/night-06/report.md` как исторический отчёт предыдущего исполнителя.

Затем читай текущую группу, нужные Vecta callers, Java и данные. Полные пути Java
в `planning/night-06/reference-index.json`. Старые пакеты и отчёты не команды
повторить давно выполненный baseline. Актуальные результаты бери из checkpoint
и Codex review, не из старого `WORKBUDDY_START_06.md`.

Последний workspace Codex: **4198 passed / 0 failed / 18 ignored**. После
test-only lint cleanup затронутый round-trip повторно прошёл; workspace Clippy
и fmt — exit 0. Это уже выполненные проверки, не команда повторить baseline.

Текущий результат: раздатчик знает snowball, experience bottle, egg, blue egg,
brown egg и splash potion — 6 из 13 ванильных projectile items. Этот счётчик
не означает, что весь dispenser или vanilla готовы. Codex проверил бинарный
persistent-record round-trip через настоящую entity factory для пяти новых
предметов и уточнил точные potion duration/damage/attribution. Живой клиент,
реальный restart/unload сервера и Mojang server comparison этим не проверены.

Работай по следующей очереди. Если зависимая часть реально заблокирована,
зафиксируй конкретный blocker и переходи к независимой группе.

1. **G006, W06-026…030 — вода, лава, рыхлый снег.**
   Сначала прочитай `BucketItem`, `SolidBucketItem`, `DispenseItemBehavior` и
   существующие fluid/container/block capabilities. Реальное размещение,
   waterlogging, допустимая замена, environment attributes, success/failure,
   звук/event, пустая тара. Не подменяй правила лавы правилами воды. Проверь
   обе руки, survival/creative, заполненный dispenser и невозможную цель.
   Общий placement consumer добавляй вместе с настоящим использованием.

2. **G007, W06-031…035 — пустое ведро.**
   Настоящий BucketPickup для воды/лавы/снега. Source и flowing различаются по
   Java. Один предмет превращается в одну наполненную тару; неполные/полные
   слоты, остаток, выброс и соседние предметы не теряются и не дюпаются.
   Соседи/fluid ticks/comparator/client updates в правильном порядке.

3. **G009, W06-041…045 — оставшиеся equippable случаи.**
   Сначала проверь уже готовый generic fallback и `as_living_entity()`.
   Подтверди humanoid/body/saddle slots, allowed-entities, занятый slot,
   player/mob, guaranteed drop/persistence где требует Java, save/load.
   Explicit projectile registration имеет приоритет; dropper не экипирует.
   Callbacks/sounds не должны повторно входить в удерживаемые inventory locks.
   Отсутствующих животных не подменяй пустыми entity ради зелёной карточки.

4. **G010, W06-046…050 — зажигание и огниво.**
   Переиспользуй существующий TntBlock и explosion engine, проверь TNT_EXPLODES,
   campfire/candle/fire, уже зажжённый/занятый target, durability, creative,
   failure и events. ВАЖНО: в Java 26.3 fire charge из dispenser — projectile
   SmallFireball, а не прямое зажигание блока. Не копируй ручной use в dispenser.
   Если требуется G047, реализуй настоящий consumer или оставь эту часть partial.

5. **G011, W06-051…055 — костная мука.**
   Реальные crop/sapling/water-plant consumers, shared ручное/dispenser
   поведение, расход, failure и existing worldgen features. Рост дерева должен
   создавать правильную feature, а не случайный набор блоков. Отсутствие
   возможности роста нельзя скрывать успешным расходом без результата.

6. **G012, W06-056…060 — бутылочки.**
   Вода и мёд: raycast/target, полный/неполный улей, настоящий honey state,
   реакция пчёл/дым где требует Java, заполненная тара и full inventory.
   Приоритет dispenser-веток, failure и reload без повторного сбора.

7. **G013, W06-061…065 — ножницы.**
   Настоящие shearable consumers: овца/цвет шерсти, соты/улей, AABB и выбор
   цели из dispenser, прочность, gamerules, events, повторная попытка.
   Не создавай фиктивную овцу или пустой shearing hook ради регистрации.

8. Дальше **G021, G022, G024, G025, G026**, затем **G027/G028** по route:
   implicit components через реальное сохранение, живое /data, living state,
   NBT dispenser, durability/equip hooks, furnace result/XP и автоматизация.
   Проверь actual code до изменений: некоторые контракты уже сделаны.

9. После этого продолжай route по **G031…G037**: RecipeDisplay/recipe book,
   wire format и initial sync, unlocks, placement/max craft и callbacks.
   Только нужные extracted values, типизированная генерация; не ручные IDs.

10. **G005 → G004**, затем стрелы/оружие и остальной route — следующие реальные
    реализации при доступных эталонах. Отсутствующий AreaEffectCloud — задача
    реализации, а не повод навсегда остановиться. Сначала ясный общий контракт
    и первый реальный consumer, без no-op foundations.

Не останавливайся после одной группы, одного коммита или часа, пока есть время
и независимая проверяемая работа. Это очередь нескольких сеансов; не обещай
600 карточек за ночь и не меняй vanilla semantics ради счётчика. Число тестов
и commits не мера полноты механики. Готовое поведение не переписывай: докажи
значимый сценарий, пометь already verified и продолжай.

AGENTS обязателен. До игрового изменения сверь local Java/extracted data и
текущие интерфейсы. Target задаёт `Cargo.toml`; manifest в minecraft-src должен
совпадать. Если неоднозначны ownership/architecture/error handling или нет
нужного эталона/output, останови зависимую работу и задай узкий вопрос:
что проверено, чего не хватает, риск divergence, точный нужный path/decision.
Независимую работу продолжай. Молчание не согласие. Это ТЗ не разрешает новые
World→Server ссылки, scope scoreboard, async/threading решения или изменение
доменной архитектуры без требуемого согласования.

Сохрани уже сделанные frame/bundle/crafting/menu/container/hopper/dropper,
lectern context, seeded loot, server-global persistent named RNG, default-world
seed и последовательные игровые тики миров со стабильным WorldMap order.
Не возвращай per-world named RNG или parallel gameplay. Сохрани TNT/explosion/
Mth/hurt/splash/tab-list и исправления subtype/owner attribution.

Без stubs/fake data, ручных registry/protocol IDs, runtime datapack JSON loader,
production unwrap/expect, borrow-checker clones, Any/TypeId и обхода линтеров.
Keyed DowncastType с уникальным owner key и SAFETY — разрешённое AGENTS исключение.
Shared hierarchy — capability traits. Generated Rust и extracted JSON руками
не редактировать. SteelExtractor: `D:\CodexBuilds\SteelExtractor`, проверить
version/output; workflow в `scripts/steel-extractor/README.md`. Обновить extractor,
получить produced file и скопировать только нужное, сохранив provenance.

Проверки должны ловить правдоподобную регрессию. Для численных правил — точный
Java-derived результат и входные данные; не просто «больше нуля» или сравнение
helper с ним самим. Для persistence — настоящий record codec/factory/storage,
не только save_additional/load_additional. Это всё ещё не живой restart сервера.
Discard не доказывает защиту от принудительного второго onHit: отдельный guard
нельзя добавлять, если его нет в Java. Тестировать нормальный lifecycle отдельно.

Сборка Windows, каждую тяжёлую Cargo-команду запускай последовательно с `-j 1`:
`CARGO_TARGET_DIR=D:\CodexBuilds\VectaMC`, `CARGO_INCREMENTAL=0`,
`TMP=TEMP=D:\CodexBuilds\tmp`. Logs — `D:\CodexBuilds\logs`, actual exit code.
Сначала проверь свободное место и процессы. Не запускай Cargo поверх незавершённой
сборки. Не меняй flags/profiles/toolchain/deps ради обхода, не убивай чужие
процессы и не закрывай VM, не меняй Windows/pagefile, не принимай EULA.

После slice — узкий relevant test (0 выбранных tests не pass). До передачи
группы: `cargo fmt --all --check`, `cargo test --locked --workspace -j 1`,
`cargo clippy --locked --workspace --all-targets --all-features -j 1 -- -D warnings`.
Relevant Python tests/audit после tooling/data изменений и при финальной передаче.
Не повторять полный suite после каждой строки. Сбой проверок указывать честно;
type-check не выдавать за runtime тест, Rust fixtures — за реальные клиенты.

Чужие dirty `planning/night-01/report.md` и `WORKBUDDY_START_06.md` не трогать,
не stage/stash/reset. Зафиксируй их hashes при входе. Сохраняй другие чужие
изменения. Не менять AGENTS, не создавать новых агентов, Gemini/Mistral CLI.
Только текущая feature branch: без main/merge/force push/reset/clean.
Stage только свои paths/hunks, review diff, commit и push проверенного результата.
README/CHANGELOG обновляй кратко по фактически готовому, attribution сохраняй.

Продолжай `planning/night-06/progress.md` и blockers.md; свой итог сохрани
в `planning/night-06/report-next-02.md`, checkpoint обнови. Исторический отчёт
предыдущего исполнителя не переписывай под новый сеанс. Для каждого slice:
ID/status, реальное before/after, Java methods, checks/count/exit, commit/push,
границы и next ID. Не создавать сотни пустых таблиц и отчётов.

Перед лимитом оставь actual HEAD/status, unfinished diff, active build/session/log
и точный next consumer. Не коммить незавершённую shared foundation ради лимита.
**В конце обязательно ответь обычным текстом:** что реально заработало,
что partial/blocked, actual tests, branch/HEAD/push, report path и следующий ID.
Отдельно укажи, выполнялись ли клиент 26.3, Linux, Mojang comparison и два
сетевых игрока. Одних файлов без текстового ответа недостаточно.
