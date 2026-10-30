# Night 03 — повторное ревью Codex

Дата: 2026-10-30. База: `15f73baf9eb004836fa5602c812fe4bbf14fc6f9`.
Ветка: `codex/night-vanilla-03`. Minecraft Java 26.3; локальный manifest
`minecraft-src/.vecta-reference.json` соответствует версии workspace, 5037 Java-файлов.

Реестр/кодеки: `3138657e32bcf409f312b200f0ffba590a5381a3`.
Рабочие станции и отчёт — следующий коммит этой ветки, содержащий этот файл.
Проверки ниже относятся к совокупному diff обоих коммитов от указанной базы,
выполнены перед фиксацией без последующих изменений Rust-кода.

## Исправления

### Реестр slot_display и кодеки

- Убраны ручные protocol ID из `SlotDisplayKind`. Значения прочитаны из
  `BuiltInRegistries.SLOT_DISPLAY` отдельной задачей SteelExtractor 26.3.
- Output `slot_displays.json` скопирован без ручного редактирования. Build script
  проверяет версию, непрерывность ID и уникальность ключей и генерирует typed
  definitions/регистрацию. Сгенерированный Rust не коммитится.
- Network read/write используют ID зарегистрированных типов. Встроенный kind
  выбирает payload codec; это больше не protocol enum.
- Для расширений сохранён путь регистрации собственных typed codecs с keyed
  downcasting по образцу существующего `PositionSource`. Проверяются конкретная
  зарегистрированная запись и тип payload, без `Any`/`TypeId` и raw-byte заглушек.
  Это основа сериализации, не загрузчик плагинов.
- Регрессии для предыдущего исправления: reference/inline trim pattern с точными
  байтами, named/direct/empty/singleton holder sets, runtime multi-item ingredients,
  nested displays, неверные ID/размеры/обрезанные inline values. Дополнительно:
  соответствие регистрации реальному output и проверка custom codec ownership.
- Inline trim holder хранится через `Box`, чтобы редкий большой payload не
  увеличивал каждый элемент enum. Сетевой формат неизменен и проверяется байтами.
- Убраны три замечания Clippy в прежнем recipe-sync коде: лишнее замыкание и
  два абсолютных пути в месте отправки пакета. Новые lint suppressions не добавлены.

Извлечение: `extractSlotDisplays --no-daemon --max-workers=1`, exit 0.
Java 25.0.4, SteelExtractor base `40e72f994d54edd619e3338fe431d7e938bf0db0`.
Patch/команды воспроизведения: [scripts/steel-extractor](../../scripts/steel-extractor/README.md).
Не запускались сервер, мир или клиент, EULA не принималась.

### Камнерез: частично вмещающийся результат

При Shift-клике `StonecutterKind::quick_move` переносил в инвентарь столько,
сколько влезало, и потреблял вход. Невместившийся остаток виртуального результата
терялся. В `StonecutterMenu.quickMoveStack` 26.3 он передаётся `player.drop`.
Добавлен тот же drop после `onTake`, с `Prediction::Predicted`, вне container locks.
Регрессия заполняет инвентарь, оставляет место для одного предмета из результата,
проверяет оставшийся вход, новый preview и ровно один корректный ItemEntity.

### Кафедра: контекст размещаемой книги

Прежний console source находился у спавна overworld и не имел source entity.
`with_entity_override` задавал только имя для `score "*"`, не селектор `@s`.
Теперь source использует мир и центр кафедры и фактического поставившего игрока,
сохраняя доступ к text queries и разрешение страниц вне inventory locks.
Регрессия проверяет NBT `SelectedItem.id` через `@s` и `Book.id` соседней кафедры
по относительным координатам, в мире, отличном от primary/console world,
в окружении существующего test server harness. Событие перелистывания использует
уже извлечённую `level_events::SOUND_PAGE_TURN` вместо локального числа.

Эталон: `LecternBlockEntity.resolveBook/createCommandSourceStack` 26.3.

## Проверки

Для изменений до load-context follow-up на `2f7e3980e`:
`cargo test --locked --workspace -j 1` — **exit 0,
3885 passed / 0 failed / 18 ignored**, включая doctests. Сборка — 7m13s.
В составе этого прогона: core 2978 passed / 0 failed / 1 ignored,
registry 424 passed, protocol 81 passed. Все 9 новых тестов прошли, включая
кафедру в отдельном мире. Лог: `codex-night03-workspace-final.log`.

`cargo clippy --locked --workspace --all-targets --all-features -j 1 -- -D warnings`
— **exit 0**. Лог: `codex-night03-clippy-verified.log`.
Это не полностью пустой от предупреждений запуск: сам Cargo выдаёт
`cargo::unused_dependencies` для `mimalloc` в `vecta/Cargo.toml`. Причина видна
в `vecta/src/main.rs`: `--all-features` включает `dhat-heap`, который выбирает
свой allocator и исключает использование mimalloc. Предупреждение Cargo не
превращается в ошибку через переданный rustc/Clippy `-D warnings`.
Зависимость, её обычное использование и настройки allocator не менялись;
предупреждение не подавлялось.

`cargo fmt --all --check` — exit 0; Python unittest — 4 passed;
audit — 3105 objects / 49 block entity types / 253 source markers.
`typos` и `prek` не запускались: команды отсутствуют в PATH.

Ранние ошибки компиляции новых тестов и замечания Clippy исправлены; их старые
логи не выдаются за прошедшие проверки. Настройки сборки и toolchain не менялись.

Логи текущих проверок: `D:\CodexBuilds\logs\codex-*`.
`planning/night-01/report.md` — чужой незакоммиченный файл; не изменён и не staged.

## Оставшееся

- Вопрос NBT load уточнён в [последующем изменении](lectern-load-context.md).
  Оно прошло пять новых тестов и полный workspace: 3890 passed / 0 failed /
  18 ignored на `015152051`; блокер проверки B-W03-07 закрыт.
  Владелец выбрал отдельный контекст чтения. При обычном disk load vanilla
  ещё не привязывает сущность к ServerLevel и не разрешает книгу; контекст
  нужен при обновлении уже живой блок-сущности командами.
- `ResultSlot` crafted count / ITEM_CRAFTED / item callbacks и recipe awards
  остаются W03-06/W03-65. Это касается и новых рабочих станций.
- Block-entity implicit components / includeData pick-block остаются B-W03-04.
- Recipe book, карты и остальная очередь Night 03 не объявляются готовыми.
- Нет проверки настоящим клиентом 26.3, Linux или двумя сетевыми клиентами.

Ревью локальных исходников и автоматические тесты не доказывают полную
ванильную совместимость всех пяти стадий WorkBuddy или всего ядра.
