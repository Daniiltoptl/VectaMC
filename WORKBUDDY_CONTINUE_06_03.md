# WorkBuddy / DeepSeek — продолжение после ревью вёдер и инструментов

---

Продолжай реализацию VectaMC, vanilla Minecraft Java 26.3 на Rust.
Каталог `C:\Users\My comp\Desktop\Vecta`, actual tip `codex/night-vanilla-03`.
Не reset к историческим SHA. Сначала branch/HEAD/status и чужие dirty hashes.

Выполняй `NIGHT_TASK_06.md` — существующие 600 карточек / 120 направлений.
Нужен код и проверяемые игровые результаты, а не ещё один общий план.
Прочитай AGENTS.md, регламент ТЗ, `planning/night-06/checkpoint.md`, route.md
и `planning/reviews/night-06-g006-g013-codex.md`. Затем текущую группу и её
реальные Vecta/Java/data callers. Старые START/CONTINUE и исторические reports
не команды повторно делать уже выполненные этапы. Результаты проверок бери
из актуального Codex review/checkpoint; baseline без причины не пересобирать.

Свежие проверки Codex: workspace 4245 passed / 0 failed / 18 ignored,
dispenser 83 passed, normal_nbt_reload 1 passed, Clippy/fmt exit 0.

Начни с **G021 / W06-101…105**, но **НЕ реализуй старое решение B-06-04**.
Этот blocker снят как фактически неверный. Java 26.3 сохраняет CustomName/lock
отдельными NBT-полями, `loadWithComponents` не вызывает `applyComponents`.
`collectComponents`/`removeComponentsFromTag` — item/pick-путь. Применение пустых
item components при обычном reload может стереть имя, lock и inventory.
Сохрани regression `normal_nbt_reload_preserves_identity_and_inventory_without_applying_item_components`.
Не удаляй root NBT поля и не добавляй hook на reload ради старого отчёта.

Очередь работы:

1. G021/W06-101…105: разделить и проверить реальные disk/owned/borrowed/live
   и item placement/pick boundaries. Не переписывать готовое. Подтвердить, что
   сохранение пишет только stored explicit map плюс специализированные NBT-поля;
   item pick удаляет соответствующие дубли из BLOCK_ENTITY_DATA; placement
   применяет implicit components к настоящей BE. Проверить name/lock/contents,
   оставшиеся зарегистрированные explicit components и удалённый component,
   реальный chunk storage save/load и functional container после восстановления.
   Не объявлять record codec test живым restart/unload сервера.

2. G022/W06-106…110: живое /data, сохранённый BlockEntityLoadContext, menu sync,
   comparator/ticker/dirty state, ошибочный NBT и книги кафедры. Не добавлять
   World→Server reference. Первоначальный load до setLevel не имитирует live
   command context; activation не разрешает книгу повторно.

3. G024/W06-116…120, G025/W06-121…125, G026/W06-126…130: persistent living state,
   реальное NBT раздатчика, прочность и equip hooks. Проверить actual code до
   изменений, включая owner/no-owner, reentrant callbacks и removal/recreation.

4. G027/G028: печи, результаты и XP, recipe tracking, automation/sided rules,
   два viewers, break/close/reload без дюпа и stale preview.

5. G031…G037: RecipeDisplay, recipe book, initial/wire sync, unlocks,
   placement/max craft и callbacks. Typed generation и реальные consumers,
   не ручные IDs, preview-only или stats вместо самой механики.

6. Возвращайся к открытым P0 зависимостям, а не откладывай их как внешние blocker:
   - G011/W06-052: настоящий TreeGrower + sapling Bonemealable одним срезом.
     Java hardcoded transform отражать typed кодом, не выдуманным registry.
   - W06-053: underwater biome/tag/coral selection и clicked face по Java;
     seagrass test сам по себе не закрывает всю ветку.
   - W06-026/G119: позиционный WATER_EVAPORATES, vanilla fizz sound/large smoke;
     не сохранять упрощённый dimension bool как полную parity.
   - G114: настоящие Bee/hive consumers, harvest, release, smoke и priority.
     Honeycomb table — HARVEST_BEEHIVE, не block-break blocks/beehive.
     Typed table и interaction-loot helper существуют: проверить его context,
     не считать нужный Rust метод отсутствующим внешним output.
   - G047: SmallFireball/fire charge; G048: wind charge.
   - G005 → G004: AreaEffectCloud → lingering potion.
   Затем дальше route, включая стрелы/оружие, AI и остальные направления.

Не ломай исправления review: неудачная стрижка оставляет инструмент в слоте,
поводки проверяются до wool и для non-Shearable entities, spectators исключены,
dispenser shear sound — Blocks, ручной — Players. Сохраняй точное число и порядок
1000/2000: bucket success — одна пара, rejected filled bucket — вложенная и внешняя,
empty-bucket failure — одна; full-container remainder добавляет свою пару.
Лава не может использовать WATER в canPlaceLiquid и заменять oak slab.

Закрыл slice → узкие meaningful tests → diff review → commit/push текущей ветки
→ следующий consumer. Не заканчивать после одной группы/часа/коммита, пока есть
время и независимая проверяемая работа. Все 600 за ночь не обещать. Число тестов,
регистраций или commits не выдавать за процент готовности vanilla.

AGENTS обязателен: сначала verify Vecta/Java/extracted data/tests. Missing Rust
method — реализация, если contract ясен. Если действительно неясны architecture,
ownership/error handling или нет нужного эталона/produced data, останови зависимую
часть, задай узкий вопрос с проверенными фактами и продолжай независимую работу.
Не выдумывать fallback и не принимать молчание за решение. Этот запрос не
разрешает новые threading/async решения или scope scoreboard.

Без fake foundations/stubs/manual registry IDs/правок generated Rust или extracted
JSON. SteelExtractor `D:\CodexBuilds\SteelExtractor`: проверить version/output,
изменить extractor при необходимости, копировать только нужные produced files.
Не менять runtime datapack модель проекта. Shared hierarchy — capabilities;
keyed DowncastType с уникальным key/SAFETY — разрешённое исключение AGENTS.

Сохрани frame/bundle/crafting/menu/container/hopper/dropper, lectern context,
seeded loot, server-global persistent named RNG, default-world seed и стабильный
последовательный world gameplay dispatch. Сохрани TNT/explosion и прежние fixes.

Cargo последовательно с `-j 1`, `CARGO_INCREMENTAL=0`:
target `D:\CodexBuilds\VectaMC`, TMP/TEMP `D:\CodexBuilds\tmp`.
Logs в `D:\CodexBuilds\logs`, actual exit/count. Не запускать параллельные builds.
После slice — узкий relevant test; до передачи группы — fmt check, locked workspace
tests, workspace Clippy all-targets/all-features с -D warnings. Relevant Python
tests/audit при tooling/data изменениях и финальной передаче. Не повторять полный
suite после каждой строки. Не менять flags/profiles/toolchain/deps ради обхода,
не убивать чужие процессы, не закрывать VM, не менять Windows и не принимать EULA.

Foreign dirty `planning/night-01/report.md` и `WORKBUDDY_START_06.md` не трогать,
не stage/stash/reset; hashes в checkpoint. Stage только свои paths/hunks.
Без main/merge/force push/reset/clean, новых агентов/Gemini/Mistral CLI.
README/CHANGELOG — кратко по реально готовому, attribution сохранять.

Продолжай progress.md/blockers.md пакета 06; новый итог —
`planning/night-06/report-next-03.md`. Исторические reports не переписывать.
На slice: IDs/status, before/after, Java methods, tests/exit/count, commit/push,
границы и next consumer. Перед лимитом — actual HEAD/status, unfinished diff,
active build/session/log и точный next ID. Не коммитить непроверенную foundation.

В конце ОБЯЗАТЕЛЬНО обычный текстовый ответ: реально готовое, partial/blocked,
tests/count/exit, branch/HEAD/push, report и next ID. Одних файлов недостаточно.
Клиент 26.3, Linux, Mojang comparison, два сетевых игрока и живой restart/unload
указывать отдельно только по фактическому выполнению.
