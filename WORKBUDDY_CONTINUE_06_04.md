# WorkBuddy / DeepSeek — продолжение после ревью G021

---

Продолжай разработку VectaMC, vanilla Minecraft Java 26.3 на Rust.
Каталог `C:\Users\My comp\Desktop\Vecta`, actual tip `codex/night-vanilla-03`.
Сначала branch/HEAD/status и чужие dirty hashes. Новые изменения не откатывать.
Исполняй существующий `NIGHT_TASK_06.md` (600 карточек / 120 направлений),
не составляй ещё один общий план. Один раз прочитай AGENTS.md, регламент ТЗ,
`planning/night-06/checkpoint.md`, `planning/reviews/night-06-g021-codex.md`,
затем current group, нужные Vecta/Java/data callers и tests.

Свежие проверки после authorization slice: workspace 4254 passed / 0 failed /
18 ignored; gate tests 2 passed, ordinary/type-mismatch 1 passed; Clippy/fmt exit 0.
Gate code — 673c7a558, но продолжать actual tip вместе с docs. Baseline до первого
изменения не повторять.

G021 НЕ полностью closed: protected-NBT gate уже реализован после успешной
extraction владельцем, но сохранение проверялось только через decoder без region
I/O/unload/restart; attached lectern item text context остаётся для G022.
B-06-04 снят: сохранять root CustomName/lock и отдельный обычный reload.

Независимая основная очередь начинается с **G022/W06-106…110**.
Parallel agents не запускать; очереди ниже — приоритеты одного исполнителя.

1. Сохрани выполненный G021/W06-103 authorization slice. Produced metadata
   26.3/49 типов уже получена, B-06-05 СНЯТ. Generator требует реальный
   only_op_can_set_nbt и проверяет version; не писать ручной список/default false.
   Player::can_use_game_master_blocks использует creative/instabuild плюс
   существующий published OP-group contract Vecta, как privileged entity item data.
   Gate проверяется при BlockItem payload load и prefill HAS_BOOK кафедры.
   Cases operator/non-operator × creative/survival, playerless placement,
   ordinary barrel и mismatched payload покрываются реальными placement tests.
   Не вводить вместо этого новую модель permissions, OP levels или guessed nodes.
   Extraction уже выполнена владельцем в обычном CMD на JDK 25, не повторять
   прежние неудачные tool-environment loopback attempts. Provenance/command —
   scripts/steel-extractor/block-entity-types.md; prepared patch теперь проверен
   выполнением task владельцем. На машине JAVA_HOME может указывать на 17 при
   PATH java=25: для будущих extractor tasks явно выбирать установленную Java 25.

2. G021/W06-105: дополнить настоящим region/chunk storage write/read и unload/load
   при доступном текущем harness. Сохранять name/lock/contents и explicit components,
   проверять functional container после восстановления. Decoder/factory — уже
   проверены, не выдавать их за новый I/O сценарий или живой restart сервера.

3. G022: настоящий builtin /data get/merge/modify/remove для block/entity/storage.
   Java DataCommands и реальные NBT path/context consumers. Использовать готовые
   command registration, execution/NBT/storage и BlockEntityLoadContext. Размер
   задачи не blocker. Если конкретная общая foundation требует неясного решения,
   проверить и спросить; не коммитить no-op общий интерфейс или половину команды.
   Независимые группы при этом продолжать.

4. G022 integration: открытое menu, cursor/storage, data slots, comparator/ticker,
   dirty state, rejected NBT, player entity restrictions, storage scope по source.
   Server state авторитетен. Не вводить новый World→Server reference.
   Для lectern сохранить явный read context; disk load до setLevel не разрешает
   книгу повторно при activation. Дополнительно проверить ITEM custom-data load:
   текущий load_item_custom_data использует обычный detached load_additional,
   тогда как Java attached lectern при loadCustomOnly разрешает книгу. Нужен
   сквозной контекст для реального caller, не фиктивный сервер у standalone fixture.

5. Затем G024/G025/G026: living persistence, NBT dispenser, durability/equip hooks.
6. Затем G027/G028: печи, результат/XP и sided automation, viewers/break/reload.
7. Затем G031…G037: RecipeDisplay, recipe book, packets, placement/max craft,
   unlocks/callbacks. Typed generation и реальный item movement, не preview-only.
8. Возвращаться к P0: TreeGrower/saplings; underwater biome/tag/coral selection;
   позиционный WATER_EVAPORATES и полные fizz/smoke; Bee/hive harvest/release,
   SmallFireball/fire charge, AreaEffectCloud/lingering. Дальше existing route.

Не начинай заново готовые рамки/мешки/крафт/меню/containers/dispenser consumers.
Сохрани fixes вёдер, ножниц, поводков, звуков/событий, TNT, общий persistent named
RNG на Server, default-world seed и стабильный последовательный world gameplay
dispatch. Сохрани snapshot item до callbacks загрузки и один saveCustomOnly на merge.

Готовый код сначала проверить, а не переписывать. Отсутствующий Rust consumer —
задача реализации при ясном contract, не внешний blocker. Закрыл slice → узкие
meaningful tests → staged diff review → commit/push текущей ветки → следующий.
Не останавливаться после одной группы/часа/коммита, если есть время и независимая
проверяемая работа. Все 600 за ночь не обещать, registrations/tests не процент vanilla.

AGENTS обязателен: verify Vecta/Java/extracted data/tests до gameplay changes.
Неясные architecture/ownership/error handling или отсутствующий produced output
требуют узкого вопроса после проверки, без guessed fallback. Молчание не решение.
Без stubs/fake foundations/manual IDs, direct generated Rust/extracted JSON edits,
runtime datapack JSON loader и production unwrap/expect/borrow-checker clones.
Keyed DowncastType с уникальным owner key/SAFETY — разрешённое исключение AGENTS.
Shared hierarchy — capabilities. Новые threading/async/scope scoreboard решения
этим запросом не согласованы.

Cargo последовательно с `-j 1`, `CARGO_INCREMENTAL=0`, target
`D:\CodexBuilds\VectaMC`, TMP/TEMP `D:\CodexBuilds\tmp`. Logs в
`D:\CodexBuilds\logs` с actual exit/count; 0 selected tests не pass.
Baseline без причины до первого изменения не пересобирать. После slice — узкий
test, перед передачей группы — fmt check, locked workspace tests и workspace
Clippy all-targets/all-features -D warnings. Relevant Python tests/audit после
tooling/data изменений и финальной передачи. Не полный suite после каждой строки.
Не менять flags/profiles/toolchain/deps ради обхода, не убивать чужие процессы,
не закрывать VM, не менять Windows/pagefile, не принимать EULA.

Foreign dirty `planning/night-01/report.md` и `WORKBUDDY_START_06.md` не трогать,
не stage/stash/reset; hashes в checkpoint. В extractor заранее были чужие
SlotDisplayRegistry и build.gradle changes: сохранить их. Stage только своё.
Без main/merge/force push/reset/clean, дополнительных агентов/Gemini/Mistral CLI.
README/CHANGELOG — кратко по реальному готовому, attribution сохранять.

Продолжать progress.md/blockers.md пакета 06; новый итог — report-next-04.md.
Исторические reports не переписывать под новый сеанс. На slice: IDs/status,
before/after, Java methods, commands/exit/count, commit/push, limits и next ID.
Перед лимитом — HEAD/status, unfinished diff, active build/session/log, конкретный
next consumer. Непроверенную общую foundation ради лимита не коммитить.

В конце ОБЯЗАТЕЛЬНО обычный текст: реально готовое, partial/blocked, checks,
branch/HEAD/push, report и next ID. Одних файлов недостаточно. Отдельно указывать
реальные client/Linux/Mojang comparison/two-player/restart checks, без подмены fixtures.
