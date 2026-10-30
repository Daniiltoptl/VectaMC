# Стартовый запрос для WorkBuddy / DeepSeek — пакет 04

Скопируй весь текст после черты. Основное задание — `NIGHT_TASK_04.md` в репозитории.

---

Продолжай разработку VectaMC — Minecraft Java 26.3 сервера на Rust.
Рабочий каталог: `C:\Users\My comp\Desktop\Vecta`.
Репозиторий: `https://github.com/Daniiltoptl/VectaMC`.
Рабочая ветка уже существует: **`codex/night-vanilla-03`**, продолжать actual tip.
Не создавать её повторно, не возвращаться к старому Night 02/03 baseline.

**Выполняй `NIGHT_TASK_04.md`: 120 подробных задач с эталонами и приёмкой.**
Это разрешение писать и проверять код по очереди, не запрос составить ещё один план.
Прочитай пакет целиком один раз; потом обращайся к текущей карточке и её источникам.
Не заканчивай после одной карточки или часа работы: закончил проверяемый slice →
узкие тесты → staged diff review → commit → push в origin своей ветки → следующий slice.
Продолжай до реального предела сеанса или подтверждённого общего блокера.
Не обещай выполнить все 120 за ночь; качество и vanilla semantics не снижать.

Сначала прочитай `AGENTS.md`, `planning/reviews/lectern-load-context.md`,
`planning/reviews/night-03-follow-up.md`, `planning/night-03/progress.md`
и `planning/night-03/blockers.md`. Сохрани фактические branch/HEAD/status.

Актуальная исходная точка перед созданием пакета:
`9e27a42f0e2b92f531715b4de5371d2176abc474`; после неё будет docs-коммит пакета.
Начинай с tip, не откатывай более новые изменения. Workspace `0.16.1+mc26.3`;
эталон `minecraft-src/.vecta-reference.json` — 26.3, 5037 Java-файлов.
В `planning/night-04/reference-index.md` — полные пути Java-точек входа нового пакета.

Последний полный baseline **3890 passed, 0 failed, 18 ignored**, включая doctests.
Финальные пять `book_resolution::lectern_load` тестов прошли отдельно.
Clippy workspace/all-targets/all-features с `-D warnings`, fmt, Python — exit 0
для того же Rust-кода. Не пересобирать baseline до первого изменения без причины.
Исторические OOM и B-W03-07 закрыты успешным runtime прогоном.
Настоящий клиент, Mojang comparison, Linux и два сетевых клиента не проверялись.

**Начни с W04-001 → W04-002 → W04-003 → W04-004 → W04-005 → W04-006.**
Это оставшийся W03-06: реальное количество созданных предметов, ITEM_CRAFTED,
callbacks, remainders и правильные result-menu consumers. В ResultSlot и
CraftingHandler есть подтверждённые TODO. Не добавляй счётчик без вызовов consumers.
Различай `onCraftedBy` и `onCraftedBySystem/onCraftedPostProcess` в 26.3;
map post-processing ждёт настоящих map foundations, а не пустого callback.
После ближайших закрытых контрактов переходи к block-entity components 011–020,
сундукам/воронкам 021–030 либо recipe book 007–010 по фактическим зависимостям.
Большие зависимые карточки держи частичными; не выдавай только stats за весь W03-06.

Не переделывать уже сделанное: W03-01..05, menu button dispatch, stonecutter recipe
packet, extracted slot-display registry, overflow drop камнереза, lectern contexts.
Loom не зависит от recipe book; update-recipes packet не требует полного RecipeDisplay.
При размещении книги source — поставивший игрок в центре кафедры; при live NBT
load — без source entity через отдельный BlockEntityLoadContext.
Первоначальный disk/worldgen load идёт до setLevel и не разрешает книгу.
Никакого дополнительного World→Server reference или resolve на chunk activation.

Все остальные карточки — проверить текущее поведение и закрыть реальные разрывы,
а не считать автоматически отсутствующим всё перечисленное. Если уже работало,
сохрани и отметь проверенный сценарий, затем продолжай следующий пункт.
Нужны рабочие действия игрока: packet→dispatch→handler→server state→sync→save/load.
Registration, preview, NBT-only entity или пустой hook не равны игровой реализации.

При неясном contract сначала проверь Vecta, локальную Java, extractor data и tests.
Если фактов всё ещё нет — точный blocker/question по AGENTS: что проверено,
чего не хватает, почему догадка изменит vanilla, какой path/output/decision нужен.
Молчание владельца не согласие; зависимую работу остановить, независимую продолжать.
Не объявлять простой отсутствующий локальный метод блокером, когда контракт и данные
уже ясны и можно закончить implementation вместе с реальным consumer.

Нельзя: стабы/fake vanilla data, no-op foundations, ручные protocol/registry IDs,
правка generated Rust/extracted JSON, обходы borrow checker, Any/TypeId вместо
keyed downcast, unapproved unsafe/unwrap, async waits в tick, отключение проверок.
Детерминированные системы используют seeded vanilla RNG по правилам AGENTS.
Не меняй зависимости/toolchain/rustflags/profiles ради успешной сборки.

SteelExtractor существует: `D:\CodexBuilds\SteelExtractor`, base 26.3
`40e72f994d54edd619e3338fe431d7e938bf0db0`.
Существующий slot-display workflow — `scripts/steel-extractor/README.md`.
Новые данные извлекать и копировать только нужные produced files с provenance.

На этой машине **каждая тяжёлая Cargo-команда с `-j 1`**:
`CARGO_TARGET_DIR=D:\CodexBuilds\VectaMC`, `CARGO_INCREMENTAL=0`,
`TMP/TEMP=D:\CodexBuilds\tmp`; сначала проверить пути и свободное место.
Логи в `D:\CodexBuilds\logs\` с корректно сохранённым exit code.
Настройка подкачки D: 16 ГБ сохранена с согласия владельца; при последнем запуске
активный размер ещё 1.33 ГБ, limit 33.23 ГБ. Тесты всё равно прошли по его запросу.
При новом OOM проверь реальные ресурсы и следуй AGENTS; не перебирай flags,
не перезагружай ПК, не закрывай приложения, не убивай все rustc/Java по имени.

После изменения — узкие реальные tests; 0 найденных tests не pass.
Shared-contract change проверяет затронутых consumers. Перед передачей игровой группы:
fmt, workspace check/test/Clippy, Python tests/audit по командам пакета.
Не запускать полный suite после каждой мелкой правки, не менять код после final
checks без relevant перепроверки. Проверь audit diff, не выдавай регистрацию за parity.

Существующий чужой `planning/night-01/report.md` не редактировать, не stage,
не отменять и не убирать общим stash. Известный SHA256 есть в большом пакете.
Не трогай AGENTS, не запускай дополнительные агенты/Gemini/Mistral CLI.
Не принимай EULA, не делай merge/main push/force-push/reset/clean всей папки.
Stage только свои paths/hunks. Проверенные commits push в origin текущей feature branch.
README/CHANGELOG обновляй кратко по реальным игровым результатам.

Журнал продолжать в `planning/night-03/`, указывать W04 IDs и связь с W03.
Не создавать сотни пустых отчётов. Итог продолжения — `report-next.md` с actual
commits, commands, counts, blockers, dirty paths и конкретным следующим ID.
Исторические отчёты сохраняй. Перед лимитом оставь точный handoff по actual state.

**В конце обязательно ответь обычным текстом, а не только файлами.**
Напиши: что заработало для игрока, какие задачи закрыты/частичны, tests с результатом,
ветка/commit/push, ссылка на report и ближайший next ID. Отдельно укажи клиент/Linux/
Mojang comparison/2+ players: выполнено или нет. Не пиши «всё готово» о Minecraft.

Начинай с проверки actual git state, затем W04-001/002. Нового согласования
на уже ясные и разрешённые этапы не требуется; реальные неясности — по AGENTS.
