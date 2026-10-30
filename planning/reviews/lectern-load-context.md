# Кафедра: явный контекст чтения при загрузке NBT

Дата: 2026-10-30. Ветка `codex/night-vanilla-03`, база `2f7e3980e`.
Версия workspace и локальных vanilla sources: Minecraft Java 26.3.

Финальная runtime-проверка закрыта на `015152051`: пять новых тестов прошли,
полный workspace — **3890 passed / 0 failed / 18 ignored**, exit 0.
Код реализации — `1f114ffb8`; после него менялась только документация.

Владелец выбрал отдельный контекст чтения при загрузке блок-сущностей.
Новая ссылка из `World` на `Server` не добавлялась.

## Уточнение поведения ваниллы

Предыдущее описание B-W03-06 слишком широко трактовало «загрузку книг».
В `LecternBlockEntity.loadAdditional` вызов `resolveBook(book, null)` действительно
есть, но разрешение выполняется только при уже установленном `ServerLevel`.

- `BlockEntity.loadStatic` создаёт сущность и вызывает `loadWithComponents`
  **до** `setLevel`. В `LevelChunk` привязка происходит позже, при
  `setBlockEntity`; повторного разрешения книги там нет. Поэтому обычное
  чтение с диска/worldgen оставляет неразрешённую книгу неразрешённой.
- `BlockInput.place` (`/setblock`, `/fill`) обновляет NBT уже существующей
  в мире блок-сущности. Здесь книга разрешается из центра кафедры, в её мире,
  **без игрока**, даже если команду выполнил игрок.
- `BlockDataAccessor.setData`, используемый `/execute store block`, также
  загружает NBT в уже привязанную блок-сущность.

Нельзя исправлять это автоматическим разрешением каждой книги при чтении чанка:
оно изменило бы ванильное поведение.

## Изменение

`BlockEntityLoadContext` — ограниченный по времени объект чтения. Его серверный
source закрыт внутри; наружу не выдаются методы управления сервером. При
живой загрузке он создаётся из эффективного мира команды, с нулевым поворотом
и без source entity. Позиция для разрешения страниц — центр целевого блока.
`detached()` представляет стадию до привязки к уровню и не разрешает страницы.

В `BlockEntity` добавлен `load_additional_with_context`; обычные поля используют
существующий decoder. `load_with_components` и owned-NBT bridge принимают
контекст явно. `/setblock`, `/fill` и callback `/execute store block` передают
его в загрузчик. Первоначальные factory/disk/worldgen пути остаются без живых
серверных запросов.

Кафедра разбирает книгу во временный `ItemStack`, разрешает страницы и вычисляет
page count до захвата container mutex, затем записывает книгу и ограниченную
страницу. Это сохраняет порядок RHS-before-assignment ваниллы: NBT запроса к
самой кафедре видит прежнюю книгу и не пытается повторно захватить её mutex.
Уже разрешённые книги не разрешаются снова. Ошибки разрешения сохраняют
исходную страницу и отмечают содержимое resolved существующим vanilla codec.

## Проверки

Добавлены пять регрессий в `server/tests/book_resolution/lectern_load.rs`:

- Живая загрузка в другом мире: `@s` пуст, ближайший игрок выбирается от центра
  кафедры, wildcard score не наследует игрока команды, storage и NBT самой
  кафедры читаются правильно; `Page = 999` ограничивается числом страниц.
- Borrowed/owned factory loads оставляют исходные страницы неразрешёнными;
  последующая загрузка с живым контекстом разрешает их.
- Повторная загрузка resolved-книги не читает изменившийся storage.
- Валидный score selector с двумя игроками вызывает runtime-ошибку разрешения:
  исходная страница сохраняется и получает resolved, как в
  `ScoreContents.findTargetName` / `WrittenBookContent.resolvePage`.
- Настоящий command dispatch `/setblock`, `/fill` и `/execute store block`
  проводит контекст до кафедры.

Окончательные исходники:

- `cargo clippy --locked --workspace --all-targets --all-features -j 1 -- -D warnings`
  — **exit 0**, включая type-check всех пяти новых тестов.
  Лог: `codex-lectern-load-clippy-verified.log`. Сохраняется прежнее отдельное
  предупреждение Cargo о mimalloc при одновременном dhat-heap; оно не подавлялось.
- `cargo fmt --all --check` — exit 0.
- `python -m unittest discover -s scripts` — 4 passed.
- `python scripts/audit_vanilla.py` — 3105 objects, 49 block entity types,
  253 source markers.
- `cargo test --locked -p vecta-core --lib book_resolution::lectern_load -j 1`
  — **5 passed / 0 failed**, exit 0; сборка 2m30s.
  Лог: `codex-lectern-load-tests-resumed.log`.
- `cargo test --locked --workspace -j 1` — **3890 passed / 0 failed / 18 ignored**,
  exit 0, включая doctests; сборка 4m12s, core 2983 passed / 1 ignored.
  Лог: `codex-lectern-load-workspace-resumed.log`.
- Предыдущая попытка `cargo test --locked --workspace -j 1` не завершилась:
  во время LLVM codegen `memory allocation of 278544 bytes failed`; с OOM
  завершился и PowerShell (exit -532462766). Лог: `codex-lectern-load-workspace.log`.

Подтверждённая причина: после падения GlobalMemoryStatusEx показал 31.9 ГБ RAM,
16.61 ГБ свободной физической памяти, commit limit 33.23 ГБ и свободный commit
11.21 ГБ. `D:\pagefile.sys` имеет размер 1.33 ГБ; диски C:/D: свободны примерно
на 29/103 ГБ. VM не обнаружена. На момент падения настройки не менялись.

Позже 2026-10-30 владелец явно разрешил увеличить подкачку на D: до 16 ГБ.
Прежний диапазон `1361 16384` заменён на `16384 16384` в `PagingFiles`;
чтение реестра подтвердило сохранение. Прежнее значение сохранено вне репозитория
в `D:\CodexBuilds\logs\pagefile-before-2026-10-30.json`.
После изменения активный файл всё ещё 1.33 ГБ, commit limit всё ещё 33.23 ГБ:
для применения настройки нужна перезагрузка Windows. Перезагрузка не выполнялась,
приложения владельца не закрывались. Затем владелец сообщил об освобождении
ресурсов и явно попросил продолжить без перезагрузки. Перед запуском лимит
оставался 33.23 ГБ, свободный commit — 9.27 ГБ, свободная RAM — 16.38 ГБ.
При этих условиях сначала узкие тесты, затем полный workspace завершились
успешно без изменения toolchain, rustflags, profiles или зависимостей.

Промежуточный узкий прогон: 4 passed / 1 failed из-за продвижения часов зависимого
мира тестовой fixture; ошибка подготовки исправлена. Ранее исправлена также
установка позиции после регистрации игрока. После этих прогонов отрицательный
case уточнён по CompilableString/NbtContents: неверная синтаксически NBT path
не служит vanilla-примером runtime-ошибки, поэтому используется валидный score
selector с двумя игроками. Промежуточные результаты не подтверждали
окончательный diff; последующие успешные прогоны выше проверяют его полностью
в пределах unit/integration/doctest suite.

Предыдущий baseline 3885 passed / 0 failed / 18 ignored относится к `2f7e3980e`.
Новый подтверждённый baseline — 3890 passed / 0 failed / 18 ignored на
`015152051`. Использованы прежние `-j 1`, target/temp на D: и
`CARGO_INCREMENTAL=0`, без перебора toolchains, rustflags и profiles.

Логи: `D:\CodexBuilds\logs\codex-lectern-load-*`.

## Границы

Это не завершение всей механики книг, кафедры или NBT-команд. Существующие
ограничения canonical JSON длины страниц и общих block-entity component
round-trips остаются отдельными задачами. `/execute store` сохраняет прежнюю
границу custom-NBT загрузки; этот diff добавляет туда контекст чтения, не
объявляя полную совместимость `BlockDataAccessor`/components.

Настоящий клиент, Linux и EULA не затрагивались. Чужой
`planning/night-01/report.md` не изменялся и не включается в коммит.
