# Запрос для WorkBuddy / DeepSeek — расширенный пакет 06

Скопируй весь текст после черты. Само ТЗ уже находится в репозитории.

---

Продолжай разработку VectaMC — vanilla Minecraft Java 26.3 сервер на Rust.
Каталог `C:\Users\My comp\Desktop\Vecta`, repository `Daniiltoptl/VectaMC`.
Продолжать **actual tip существующей ветки `codex/night-vanilla-03`**.
Сначала зафиксировать git branch/HEAD/status. Более новые изменения не откатывать.

**Выполняй `NIGHT_TASK_06.md`: 600 карточек в 120 направлениях.**
Это запрос реализовывать и проверять код, не составлять новый план.
Один раз прочитай AGENTS.md, регламент в начале ТЗ, `planning/night-06/checkpoint.md`
и `planning/night-06/route.md`. Затем читай текущую группу Gxxx и её карточки,
соответствующие Vecta callers/Java/extracted data/tests. Не загружай весь большой
каталог на каждом turn. Полные Java paths — `planning/night-06/reference-index.json`.
Старые пакеты 04/05 — history, их baseline и указание заново делать crafting не выполнять.

Начни с **G001 / W06-001…005 — experience bottle из раздатчика**.
Потом G002, G003, G006, G007, G009…013 по настоящим зависимостям.
Функциональный slice может закрыть несколько карточек вместе: shared foundation
и первый реальный consumer передавать вместе. Отдельный пустой hook не коммитить.

Закрыл проверяемый slice → узкие meaningful tests → staged diff review → commit →
push текущей feature branch → следующая независимая карточка. Не останавливаться
после первой карточки, одного коммита или часа, если есть проверяемая работа и время.
Продолжать до реального лимита сеанса или подтверждённого общего блокера.
Не обещать выполнить все 600 за ночь и не снижать vanilla semantics ради количества.

Наличие карточки не доказывает отсутствие функции. Готовый код не переписывать:
проверить значимый сценарий, отметить already verified и двигаться дальше.
Нужен action/packet/use/tick → dispatch → state → sync → save/load, где это требует Java.
Registration/preview/codec/NBT-only entity/stat counter не равны рабочей механике.

Сохранять сделанные frame/bundle/crafting/menu/container/hopper/dropper/dispenser
contracts, контекст книг кафедры, потоковый seeded loot и общий persistent RNG.
Владелец явно выбрал один RNG owner на Server и последовательные world game ticks.
WorldMap insertion order стабилен после восьми миров; parallel gameplay не возвращать.
`as_living_entity()` и реальные game-event listeners уже есть, новых fake seams не нужно.
Сохранить перенесённые TNT/explosion/Mth/hurt/splash/tab-list и наши subtype fixes.

Прочитай reviews:
`planning/reviews/upstream-2026-11-01.md`,
`planning/reviews/server-random-sequences.md`,
`planning/reviews/W04-019-034-codex.md`,
`planning/reviews/W04-001-021.md`, `planning/reviews/W04-016-029.md`.
Последний игровой baseline — **4185 passed / 0 failed / 18 ignored**, Clippy/fmt прошли.
Это уже выполненные проверки, не повод снова строить baseline без нового изменения.
Текущий пакет 06 меняет документацию, не игровой код.

При неясном contract сначала проверить Vecta/Java/data/tests, потом узкий вопрос по AGENTS.
Отсутствие готового Rust метода не блокер, если его можно закончить с настоящим consumer.
Missing architecture/ownership/extractor output — реальный gate; не угадывать и не писать fallback.
Зависимую карточку остановить, независимую продолжать. Молчание не согласие.
G086 runtime scoreboard access и любые новые threading/async решения не согласованы этим текстом.

Target определяется Cargo.toml: `0.16.1+mc26.3`, source manifest должен совпадать.
SteelExtractor `D:\CodexBuilds\SteelExtractor`: проверить actual checkout/output перед использованием.
Generated Rust/extracted JSON руками не править; IDs из памяти не вписывать.
Не импортировать broad lint allows/production unwrap/expect/Any/TypeId/unsafe из upstream.
Keyed DowncastType с уникальным owner key и SAFETY — по разрешённому исключению AGENTS.

Каждая тяжёлая Cargo-команда **-j 1**, `CARGO_INCREMENTAL=0`,
`CARGO_TARGET_DIR=D:\CodexBuilds\VectaMC`, `TMP=TEMP=D:\CodexBuilds\tmp`.
Logs в `D:\CodexBuilds\logs` с actual exit/count. Не запускать несколько Cargo builds одновременно.
Не менять flags/profiles/toolchain/deps ради сборки, не закрывать VM/приложения,
не убивать все rustc/Java, не менять ОС и не принимать EULA.
До передачи группы: fmt, locked workspace tests, workspace Clippy all-targets/all-features
с -D warnings, relevant Python tests/audit. Не полный suite после каждой строки.

Чужой `planning/night-01/report.md` не трогать, не stage и не stash/reset.
Не менять AGENTS, main, чужие files, не force push/clean/reset всей папки.
Не запускать дополнительные агенты, Gemini/Mistral CLI и стороннюю рассылку.
README/CHANGELOG — кратко по реальным результатам, attribution сохранять.

Журнал продолжать в `planning/night-06/progress.md` и blockers.md, итог — report.md.
На карточку: W06 ID/группа/старый W05 ID, before/after, source methods, tests/exit/count,
commit/push, partial/blocker и следующий consumer. Сотни пустых отчётов не создавать.
Перед лимитом оставить actual HEAD/status, dirty paths, unfinished diff, active build/session/log
и конкретный next ID. Непроверенную общую foundation ради лимита не коммитить.

**В конце обязательно обычный текстовый ответ, не только файлы:** что заработало,
что partial/blocked, tests/exit/count, branch/HEAD/push, report и следующий ID.
Отдельно client/Linux/Java run/два сетевых игрока — что реально проверено.
