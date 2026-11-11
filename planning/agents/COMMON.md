# VectaMC: общие правила для трёх агентов Antigravity

> В Arena локальные пути Windows, папки сборки на диске D и локальная сборка не применяются: см. `planning/agents/ARENA.md`.


Этот файл читают все три агента. Свой бриф у каждого в его папке.

## Проект

VectaMC — сервер Minecraft Java 26.3 на Rust. Цель — поведение 1:1 с ванилью.
Единственный источник истины — Java-исходники:
`C:\Users\My comp\Desktop\Vecta\minecraft-src\minecraft\src\net\minecraft\`
(только чтение). Данные реестров только из SteelExtractor
(`D:\CodexBuilds\SteelExtractor`, вывод `run/steel_extractor_output/`),
копируй только нужные файлы. Руками registry/protocol/worldgen данные не вписывать.
Правила кода — `AGENTS.md` в твоей папке, они важнее общих привычек. Работу
проверяет Claude, ревью в `planning/reviews/` — прочитай их.

## Три агента

| Агент | Папка | Ветка | Журнал | Области |
|---|---|---|---|---|
| A | `C:\Users\My comp\Desktop\Vecta` | `codex/night-vanilla-03` | `planning/night-06/` | крафт и станки, зачарования, игрок (голод, еда, эффекты, урон, смерть, опыт, бой), предметы, оружие и снаряды |
| B | `C:\Users\My comp\Desktop\Vecta-b` | `codex/night-vanilla-03-b` | `planning/night-06b/` | редстоун, жидкости, огонь, рост растений, интерактивные блоки, порталы и измерения, команды |
| C | `C:\Users\My comp\Desktop\Vecta-c` | `codex/night-vanilla-03-c` | `planning/night-06c/` | мобы (ИИ, навигация, спавн, виды), прочие сущности, покрытие протокола |

- Работай только в своей папке и пушь только свою ветку. Не заходи в чужие
  папки, кроме чтения `minecraft-src` из папки A. Не мержи, не ребейзь, не
  force-push, не пушь в main. Слияние делает Claude после ревью.
- Чужую область не трогай. Нужен фундамент из чужой области — запиши в свой
  `blockers.md` «нужно от агента X: …» и бери следующий пункт.
- Общие трейты (`BlockBehavior`, `ItemBehavior`, `Container`, `Entity`,
  `LivingEntity`, damage pipeline, `World`) меняй только если без этого никак,
  минимально, и перечисли такие изменения в отчёте: они дадут конфликты слияния.
- `CHANGELOG.md` не трогает никто, кроме Claude.

## Перенос из SteelMC (2026-11-11, не дублировать)

Claude перенёс свежие коммиты SteelMC (ветка `codex/upstream-2026-11-11`,
отчёт `planning/reviews/upstream-2026-11-11.md`) и уже влил их во все три
ваши ветки. Итог после переноса: 4521 passed / 0 failed / 18 ignored. Уже есть:
кальмар и его цели, спавнер (`BaseSpawner`, `SpawnPlacements`), вагонетки
(`AbstractMinecart`, обычная вагонетка, поведение на рельсах) и вагонетка со
спавнером, оставляющие зелья и `AreaEffectCloud`, сохранение/загрузка
достижений, conduit block entity, взрывы кровати и якоря, модификаторы скорости
копания, отталкивание сущностей и spatial queries, рефактор жидкостей,
нестрогое чтение чисел NBT (`NbtValueInput`), звуки ближней атаки игрока,
мёртвый игрок не выбрасывает предметы.

НЕ пиши эти вещи заново. Если пункт очереди совпадает — пропусти его, запиши в
progress «закрыто переносом SteelMC» и возьми следующий. Стол зачарования из
SteelMC НЕ перенесён (их система компонентов несовместима с нашей), его агент
A пишет сам по Java.

## Сборка (у каждого свой target)

| Агент | CARGO_TARGET_DIR | TMP/TEMP | Логи |
|---|---|---|---|
| A | `D:\CodexBuilds\VectaMC` | `D:\CodexBuilds\tmp` | `D:\CodexBuilds\logs` |
| B | `D:\CodexBuilds\VectaMC-b` | `D:\CodexBuilds\tmp-b` | `D:\CodexBuilds\logs-b` |
| C | `D:\CodexBuilds\VectaMC-c` | `D:\CodexBuilds\tmp-c` | `D:\CodexBuilds\logs-c` |

Везде `CARGO_INCREMENTAL=0`, nightly Rust, все cargo с `-j 1`. Внутри одного
агента — никогда два cargo одновременно. При OOM или нехватке места на D: не
перебирай флаги, запиши блокер и подожди/переходи к чтению Java для следующего
среза.

Перед каждым коммитом кода:

```
cargo fmt --all --check
cargo test --locked --workspace -j 1
cargo clippy --locked --workspace --all-targets --all-features -j 1 -- -D warnings
```

После: `python scripts/audit_vanilla.py`, `python -m unittest discover -s scripts`.
Сначала узкий фильтр тестов (ноль найденных тестов — не прохождение). Последний
итог workspace: 4521 passed / 0 failed / 18 ignored.

## Режим работы

Автономно до конца лимита. Срез = Java-метод(ы) → код → тест на настоящем пути →
журнал → коммит → push → следующий срез. Коммить после КАЖДОГО среза. Блокер
записал — перешёл к следующему пункту, сеанс не останавливай. Каждые 3–4 среза
обновляй `checkpoint.md` (HEAD, незаконченное, точный следующий шаг). Перед
пунктом проверь `NIGHT_TASK_06.md` и код: не сделано ли уже.

Запрещено: принимать EULA, запускать ванильный сервер, менять систему/
toolchain/Cargo profiles, трогать `AGENTS.md`, `planning/night-01/report.md`,
`WORKBUDDY_START_06.md`, правка generated (`src/generated/`) и extracted JSON
руками, `.clone()` ради borrow checker, `.unwrap()/.expect()` в production,
`unsafe` (кроме keyed `DowncastType` с `// SAFETY:`), `Any`/`TypeId`, `todo!()`,
стабы и fake vanilla data. Clippy подавляй только `#[expect(..., reason = "...")]`.

## Чек-лист ревью (обязателен)

Полный список — раздел «Чек-лист ревью» в
`C:\Users\My comp\Desktop\Vecta\WORKBUDDY_CONTINUE_06_10.md` (только чтение).
Коротко:

- Сверяй ВСЕ методы ванильного класса, не только основной сценарий.
- floorDiv/floorMod не Евклид; `*Exact` падают при переполнении; Java считает
  `int/float` во float; `Math.max/min` сохраняют NaN.
- Для каждого случайного вызова выясни, чей `RandomSource` в Java (level, из
  `tick`, сущности). Новая `ItemEntity` берёт скорость из своего random.
- **Новое:** не оборачивай ванильный `RandomSource` в трейты крейта `rand`
  (`random_range`, `gen_range` и т.п.). Их алгоритмы не совпадают с Java
  `nextInt(bound)`/`nextFloat`/`nextDouble`, поэтому поток и значения
  расходятся с ванилью, хотя код выглядит «на level random». Зови методы
  ванильного генератора Vecta напрямую (трейт `vecta_utils::random::Random`: `next_i32_bounded`, `next_f32`, `next_f64` и т.п.).
- Внутри `world.random().with_level(...)` только вычисления, без записи в мир:
  блокировка нереентерабельна.
- `container_ref` только у тех, кто в Java `instanceof Container`;
  `SimpleContainer.setItem/removeItem` вызывают `setChanged`.
- Тест проверяет ожидание из Java, а не текущее поведение кода. Используй
  `RecordingConnection` для пакетов. Тест, зелёный только при удачном броске,
  недопустим.
- В отчёте — итог ВСЕГО workspace. «Сделано» только то, что проверено кодом
  и тестом; частичное помечай частичным. Клиент 26.3 вживую не проверялся —
  так и пиши.

## Отчёт

Итог сеанса — `report.md` в своём журнале: что заработало для игрока, закрытые
и частичные карточки, итог всего workspace, коммиты, блокеры, изменённые общие
трейты, следующий шаг. В конце ОБЯЗАТЕЛЬНО ответь обычным текстом с теми же
пунктами.
