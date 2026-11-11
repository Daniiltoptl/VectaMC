# Агент B — progress (ночь 06, пакет 06)

Формат: дата/срез → что сделано → результат проверок → коммит.

## 2026-11-11 · Подготовка рабочего места

- Ветка `codex/night-vanilla-03-b` на tip `d92de0855`, tree clean.
- Прочитаны: AGENTS.md, BRIEF.md, CHECKLIST.md, NIGHT_TASK_06.md (точки
  входа групп G095/G101–G107/G116/G117/G119), reviews night-06-session08/09-claude,
  planning/night-06/checkpoint.md и blockers.md.
- minecraft-src не генерировался: Java-исходники 26.3 читаются из
  `C:\Users\My comp\Desktop\Vecta\minecraft-src\minecraft\` (только чтение),
  как указано в поправках к BRIEF.
- Сборка: `CARGO_TARGET_DIR=D:\CodexBuilds\VectaMC-b`, `-j 1`,
  `CARGO_INCREMENTAL=0`, TMP/TEMP=`D:\CodexBuilds\tmp-b`, логи
  `D:\CodexBuilds\logs-b`.
- Setup-находка: в Vecta-b отсутствовали gitignored extracted-входы сборки
  (`vecta-utils/build_assets/builtin_datapacks/`, `deprecated.json`,
  `en_us.json`, `version.json`, файл `builtin_datapacks/minecraft/.version`).
  Их производит сам build-скрипт vecta-utils (скачивание server jar), но в
  offline-ситуации это блокер сборки. Данные скопированы из локальной папки
  агента A (только чтение источника, версия совпадает: `0.16.1+mc26.3`,
  version.json `id: 26.3`), `.version` записан как `26.3`. В git не попадают
  (gitignore), в основную папку агента A ничего не писалось.
- SteelExtractor (`D:\CodexBuilds\SteelExtractor`) проверен: в
  `run/steel_extractor_output/steel-registry/build_assets/` есть
  block_entities/context_number_provider_*/loot_contexts/recipe_displays/
  slot_displays; builtin_datapacks там нет (см. выше, взяты из папки A).

## Срез 1: bonemeal на level random + кораллы (готово)

- Файл: `vecta-core/src/behavior/items/bonemeal.rs`. Найдено по
  blockers.md сеанса 10 п.2 + W06-053: `rand::rng()` вместо level random,
  grow_water_plant игнорирует clicked face и выбор кораллов по тегам.

### Что сделано

- `vecta_utils::random::RandomAccess` — object-safe фасад над ванильными
  генераторами (`next_i32`, `next_i32_bounded`, `next_f32/f64`, `next_bool`);
  `vecta_utils::random::shuffle` = ванильный `Util.shuffle` для `&mut dyn`
  (generic-метод сломал бы dyn-совместимость).
- `LevelRandomSource` (`vecta-core/src/world/random.rs`) — хэндл над
  `LevelRandom`, берёт блокировку на один draw и зовёт ванильные методы
  (`nextInt`/`nextFloat`), а не `rand::Rng`.
- `Bonemealable` и все реализации (~43) переведены с `&mut dyn rand::Rng` на
  `&mut dyn RandomAccess`; `find_spreadable_neighbor_pos` тоже принимает rng.
  Это изменение общего трейта — помечено в отчёте.
- `BoneMealItem::grow_water_plant` восстановлен по `BoneMealItem.java`:
  `PRODUCES_CORALS_FROM_BONEMEAL`, `WALL_CORALS` при `j == 0` и горизонтальной
  грани, `UNDERWATER_BONEMEALS` под `nextInt(4) == 0`, до 4 ретраев facing,
  `getRandomElementOf` = один bounded draw по тегу.
- `CropBlock`/`BeetrootBlock`: `getBonemealAgeIncrease` через level random,
  beetroot — `super.getBonemealAgeIncrease(level) / 3`.
- Random ticks растительности (росток/бамбук/лоза/келп/пропагул/стебель)
  берут `LevelRandomSource::new(world.random())`, как Java `randomTick(...,
  RandomSource random)`.
- Тесты: 3 новых в `bonemeal.rs` — один draw по размеру тега, facing только при
  наличии свойства, горизонтальный ретрай bounded 4.

### Проверки

- `cargo fmt --all --check` — ok.
- `cargo test --locked --workspace -j 1` — 4523 passed / 0 failed / 18 ignored
  (32 сьюта).
- `cargo clippy --locked --workspace --all-targets --all-features -j 1 -- -D warnings`
  — ok.
- `python scripts/audit_vanilla.py` — ok, `planning/audit` пересобран.
- `python -m unittest discover -s scripts` — 4 tests ok.

### Отклонение (записано в blockers.md)

- В Java `Block.updateShape` получает `RandomSource` параметром
  (`level.getRandom()`), поэтому `GrowingPlantBodyBlock.updateShape` берёт
  `headBlock.getStateForPlacement(random)`. В Vecta `BlockBehavior::update_shape`
  рандома не имеет (366 вызовов); конверсия body→head берёт отдельный legacy-поток
  с тем же диапазоном (`nextInt(6)`), живой геймплей — unseeded runtime RNG.
  Расширение трейта — отдельная задача, не в этом срезе.
