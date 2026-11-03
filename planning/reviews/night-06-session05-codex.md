# Ревью сеанса05: placement, region I/O и entity data

Дата: 2026-11-03. Вход `049b831da`, branch `codex/night-vanilla-03`.
Проверены изменения `77bd4142c..049b831da`, локальная Java26.3 и callers.
Код исправлений ревью: `1a8bb2457`.
Cargo target и manifest совпадают:26.3/5037 Java.

## Принятые изменения

Placement использует существующие Player/Server и явный BlockEntityLoadContext.
World→Server ссылка не добавлена. Snapshot вне inventory lock, type/OP gates и
порядок custom data→components→setPlacedBy сохранены. RegionManager test пишет
настоящий `.mca`; save_without_metadata и load_with_components исправляют обе
потери explicit components. Detached disk decoding не разрешает текст книги.
Root CustomName/lock и отсутствие item applyComponents при reload сохранены.

## Исправленные дефекты

1. **P1: execute store обходил запрет player NBT.** Проверка находилась только
   в DataAccessor. Теперь она в общем load_command_data, typed EntityNbtLoadError
   передаётся обоим callers. `/data` сохраняет translation; отказ store сохраняет
   результат вложенной команды. Ожидаемый отказ пишется на debug с причиной.
2. **P2: execute store принимал множество entities.** Java использует
   DataCommands.TARGET_PROVIDERS/EntityArgument.entity. Теперь single target
   argument и context.entity. Проверены runtime player rejection и parse
   rejection selector-а, допускающего несколько целей.
3. **P1: numeric vectors читались только из строго типизированных lists.**
   `[35,66,30]` не считался Pos, `[1,0,0]` — Motion, integer Rotation не читался.
   Vec3/Vec2 codecs принимают numeric values, NbtOps.getStream — primitive arrays.
   Reader поддерживает эти representations. Сохранена partial-value семантика:
   overlong list отдаёт prefix через Util.fixedSize и TagValueInput.read;
   short list даёт default. Проверяется смена chunk и реальный поиск по AABB.
4. **P2: NumericTag conversions были неполными.** Boolean читал только Byte,
   int — Byte/Short/Int. Добавлены остальные числовые tags, byte narrowing для
   bool и short narrowing для Fire. `fall_distance` принимает numeric double;
   legacy FallDistance не является полем Entity.load26.3. Проверены int boolean,
   low-byte boolean, long→int и фактический fire timer, не повторный NBT cast.
5. **P2: Motion load не выставлял needsSync.** Теперь используется существующий
   velocity-sync marker. Проверен marker после команды, клиент не запускался.
6. **P2: invulnerable_time отсутствовал в command export/load.** Добавлены
   snapshot, запись и reset. Проверены сохранение при посторонней мутации,
   запись7 и сброс remove. Disk living persistence остаётся задачей G024.

Entity suite вынесен из data_command.rs в data_entity.rs; добавлены три
server-backed regression tests. Generated Rust/extracted JSON не правились.

## Что осталось незавершённым

**B-06-06 не закрыт полностью.** По чтению кода load_command_data вызывает base
reader и subtype loader. Sheep.load_additional→load_mob загружает equipment/mob
state, но общего LivingEntity.readAdditionalSaveData нет. Health, HurtTime,
DeathTime, AbsorptionAmount, attributes, active_effects и остальные living поля
экспортируются save_command_nbt, но не восстанавливаются этим путём. Команда
может вернуть успех без изменения нужного поля. Это вывод по call graph;
не выдан за выполненный клиентский сценарий или runtime проверку Health.

`chunk_saver/storage/entities.rs::entity_to_persistent` пишет subtype additional
NBT отдельно от typed base; command dump не обеспечивает living disk persistence.
Сначала G024/shared living load/save, затем зависимые consumers. Регистрация
провайдера не является завершением полной NBT foundation.

**Team — задача реализации.** Scoreboard уже имеет teams, holder membership и
domain persistence; CommandSource предоставляет Server. Отсутствие `/team`
не блокирует NBT Team. Java Entity.load читает UUID до Team, затем accessor
восстанавливает UUID. Игнорирование UUID сохраняет identity, но не доказывает
побочные результаты payload с UUID+Team. Live UUID index не менять наугад.

postDataManipulated, partial/invalid codecs, nonfinite position/rotation, sleeping,
combat references и living sync требуют дальнейшей проверки. По семи command
tests нельзя утверждать их совпадение. Compute также не реализован; inline
dispatch не является доказанной потребностью в новой extractor registry.

## Проверки

Узкий `cargo test --locked -p vecta-core --lib data_entity -j 1`:7 passed.
Поздние numeric-array/partial-list assertions входят в итоговый workspace.
Первый regression запуск выявил неверное использование test helper для ожидаемого
parse error; исправлен test caller на parser API.
`cargo test --locked --workspace -j 1`: **4288 passed /0 failed /18 ignored**,
exit0, сборка3m18s. Последний Clippy all-targets/all-features с `-D warnings`:
exit0,1m14s после исправления explicit enum arm; lint suppression не добавлялся.
Cargo выдал прежний manifest warning mimalloc при одновременном dhat-heap;
allocator/manifest не менялись. Python scripts:4 tests OK.
После lint-only правки: data_entity **7 passed**, exit0; fmt **exit0**.
Audit после code commit:3105 objects/49 BE/231 markers, provenance `1a8bb2457`.
Логи: `D:\CodexBuilds\logs\codex-g022-next05-*`.
Входящие4285/0/18 — отчёт исполнителя, не свежий прогон Codex.

Клиент26.3, Linux, Mojang differential, два сетевых клиента и live restart/unload
не запускались. Region fixture не является вторым сервером или ticket lifecycle
прогоном. EULA не принималась.

## Передача

Новый запрос — WORKBUDDY_CONTINUE_06_06.md вместе с полной очередью файла05.
Первым закончить living/entity foundation. Итог — report-next-06.md и ответ текстом.
Foreign hashes сохранены; файлы не менять/не stage:

- planning/night-01/report.md:
  `06680CBC971245B70C990D95C906F0F1E3685EFFDE61C13D562664B4B6B05222`;
- WORKBUDDY_START_06.md:
  `48430DC1D56AF1E679E0A81411518002B8C406B3F27648AD1A197DF174AFAD5A`.
