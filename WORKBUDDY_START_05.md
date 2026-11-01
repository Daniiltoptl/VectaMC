# Стартовый запрос для WorkBuddy / DeepSeek — сеанс 05

Скопировать весь текст после черты. Полное задание уже лежит в репозитории.

---

Продолжай разработку VectaMC, ванильного Minecraft Java 26.3 сервера на Rust.
Каталог: `C:\Users\My comp\Desktop\Vecta`.
Репозиторий: `https://github.com/Daniiltoptl/VectaMC`.
Продолжать actual tip существующей ветки `codex/night-vanilla-03`.

**Выполняй `NIGHT_TASK_05.md`: 100 подробных карточек, зависимости и приёмка.**
Это запрос реализовывать и проверять код, не писать ещё один план.
Сначала AGENTS.md, `planning/night-05/checkpoint.md` и перечисленные там свежие reviews.
Старый WORKBUDDY_START_04.md устарел: baseline и указание заново делать crafting игнорировать.
Не откатывай новые commits и не переписывай уже сделанные TNT/RNG/menus.
Прочитай итоговый baseline в checkpoint; без новой причины до первого изменения не пересобирай его заново.

Начни с **W05-001: пузырёк опыта из раздатчика**, потом 002, 003, 006, 007, 009–013
по реальным зависимостям. Закрыл проверяемый slice → узкие meaningful tests → staged
diff review → commit → push текущей feature branch → следующая независимая карточка.
Не заканчивай после одной задачи или часа. Продолжай до реального лимита сеанса;
не обещай все 100 за ночь и не снижай vanilla semantics ради количества.

Нужны action/packet/use/tick → dispatch → state → sync → save/load.
Регистрация, preview, NBT-only entity и пустой hook не равны готовой механике.
Локальная Java 26.3 и extractor output — эталон, не имена/цифры из памяти.
В `planning/night-05/reference-index.json` проверены полные пути Java классов.

Не переделывать мешки, рамки, меню, ПКМ/Q crafting contracts, контекст книг кафедры,
контейнеры/hopper/dropper, generic equippable dispenser и потоковый seeded loot.
Living capability уже есть: `as_living_entity()`; события наблюдаемы GameEventListener.
Владелец выбрал один persistent RNG owner на Server и последовательные игровые тики
миров. WorldMap стабилен и после восьми миров. Parallel world gameplay не возвращать.
Свежие обновления читать по `planning/reviews/upstream-2026-11-01.md`.

При неясном контракте сначала код/Java/data/tests. Реальный blocker записать точно;
зависимое остановить, независимое продолжить. Архитектуру и missing extractor data
согласовывать по AGENTS. Отсутствие Rust метода не блокер, если можно закончить helper
с реальным consumer по ясному эталону. Молчание владельца не согласие.

Каждая тяжёлая Cargo-команда **-j 1**:
`CARGO_TARGET_DIR=D:\CodexBuilds\VectaMC`, `CARGO_INCREMENTAL=0`,
`TMP=TEMP=D:\CodexBuilds\tmp`. Logs в `D:\CodexBuilds\logs` с exit codes.
Не менять flags/profiles/toolchain/deps ради сборки, не закрывать VM/приложения,
не убивать все rustc/Java, не менять ОС и не принимать EULA.
SteelExtractor: `D:\CodexBuilds\SteelExtractor`; проверить actual version/output.

Не трогать чужой `planning/night-01/report.md`, AGENTS, main и чужие изменения.
Никаких общих reset/clean/stash, force push, дополнительных агентов/CLI делегатов.
Generated Rust/extracted JSON руками не править; no-op foundations, fake data,
production unwrap/expect, Any/TypeId и unapproved unsafe не импортировать из upstream.

Вести `planning/night-05/progress.md` и blockers.md. Перед передачей игровой группы:
fmt, locked workspace tests, workspace Clippy all-targets/all-features `-D warnings`,
Python tests/audit. Не полный suite после каждой строки, а после готовой группы.
README/CHANGELOG кратко по факту, attribution сохранять.

Перед лимитом точный handoff: HEAD/push, dirty paths, незавершённый diff,
active build/session/log, следующий ID. Непроверенную foundation ради лимита не коммитить.
**В конце обычный текстовый ответ, не только файлы:** что заработало/частично,
tests/exit/counts, branch/HEAD/push, report и next ID. Отдельно client/Linux/Java golden/
два сетевых игрока — что действительно проверено.
