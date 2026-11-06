# Сеанс 8: context number providers, configured values, печь и `/data compute`

Дата: 2026-11-04. Вход `cd73000d2`, ветка `codex/night-vanilla-03`.
Входящий workspace **4308 passed / 0 failed / 18 ignored**.

Реализовано главное из `WORKBUDDY_CONTINUE_06_07.md` §B и §C: настоящие
`ContextFloatProvider` / `ContextIntProvider` с registry dispatch, configured
values из extractor output, игровой consumer (печь) и настоящий
`/data modify … compute` с `default`/`block`/`entity` context sources.

## Срез 1: typed provider model + configured values (`9190929d2`)

Новый `vecta-registry/src/loot_table/number/`:

- `FloatProvider` (27 типов) и `IntProvider` (22 типа) — точный каталог 26.3
  (`context_number_provider_types.json`: 28 float + 23 int, минус
  `environment_attribute`).
- `ProviderRef` = registry key или inline payload, `ConditionRef` = predicate key
  или inline condition, `WeightedList` + `WeightedValue` = `WeightedList` Java.
- `get_float` / `get_int` — безопасные обёртки Java
  (`ContextFloatProvider.getFloat` / `ContextIntProvider.getInt`): любая
  арифметическая ошибка или не-конечный float дают 0. Композитные провайдеры
  вычисляют операнды через `*_unsafe`, который пробрасывает ошибку — как в Java.
- `get_float_or_throw` — третья точка входа Java.
- `FloatProviderRegistry` / `IntProviderRegistry` хранят свои значения и
  подключены к `REGISTRY` (freeze после vanilla init, plugin registration
  до этого).

### Арифметика — по каждому Java implementation, не одним правилом

Проверено standalone-оракулом на JDK 25 (`D:\CodexBuilds\oracle\Oracle.java`,
вывод в отчёте; MinecraftServer не запускался, EULA не принималась):

| Операция | Java поведение | Vecta |
| --- | --- | --- |
| `abs` / `negate` | `absExact` / `negateExact` — `MIN` кидает | `Err(Arithmetic)` → 0 |
| `add` / `mul` / `avg` (int) | аккумулятор `long`, затем `longToIntSafe` | то же, переполнение → 0 |
| `div` (int) | `MIN / -1 == MIN` без исключения, `/0` кидает | `wrapping_div`, `/0` → 0 |
| `floor_div` | `floorDivExact` — `MIN / -1` кидает | `checked_div_euclid` |
| `mod` / `floor_mod` | `%` и `floorMod`; `floor_mod(MIN,-1)=0` | то же |
| `pow` | `powExact`: любой отрицательный экспонент кидает, переполнение кидает, провайдер отдельно запрещает 0^0 | то же |
| `round` / `ceil` (float) | `Math.round(float)` и `Mth.ceil` сужают через int: NaN→0, ±Inf и 1e10 → границы int | то же |
| `mod` (float) | делитель вычисляется **первым**, ноль → NaN | то же |
| `sqrt`, `pow` (float) | `(float)Math.pow/sqrt`, `-1 → NaN` | то же |
| `sin` / `cos` | таблица `Mth.SIN` (65536) | существующий `vecta_math::trig` |
| `max` / `min` | старт с `∓Float.MAX_VALUE` / `i32::MIN/MAX` | то же |
| `uniform` | `Mth.nextFloat/nextInt` | то же, включая переполнение `max-min+1` |
| `weighted_list` | один `nextInt(totalWeight)` + линейный выбор | то же |

`from_int` / `from_float` — `intToFloatSafe` / `floatToIntSafe`
(не-конечное → ошибка, long narrowing → проверка диапазона).

### Configured values — реальные данные, не ручные константы

`build_assets/context_number_provider_values.json` — побайтовая копия extractor
output (sha256 `53a9763a67c5e7c5c58a5d2d239455309b3ea8223d1134e8d53f317247fef9ad`
совпадает с `D:\CodexBuilds\SteelExtractor\...`). Рядом лежит
`context_number_provider_types.json` как каталог. Build script
`build/context_number_providers.rs` компилирует 4 float + 26 int значений в
typed Rust: unknown payload type или незнакомое поле паникует на этапе сборки,
вместо того чтобы подставлять угаданную константу.

`vanilla_loot_predicates` повторяет три код-бутстрапнутых предиката Java
(`block/fast_cooking`, `tool/can_shear`, `tool/can_silk_touch`), поэтому
`conditional` / `number_dispatcher` разрешают настоящие предикаты, а не
permissive-заглушку. Ключ вне таблицы — ошибка.

### Границы (осознанные, не маскировка)

- `environment_attribute` отсутствует: в Vecta нет foundation окружения;
  генератор паникует, парсер отдаёт `UnsupportedType`.
- `enchantment_level` требует `LevelBasedValue`; существующий тип хранит
  `'static`-внутренности (данные компилируются из extraction), поэтому runtime
  inline-парсер его отвергает. Нужен owned-вариант для command payloads.
- Inline-условия (`match_block` внутри payload команды) не парсятся:
  `LootCondition::MatchBlock` хранит `&'static [PropertyCheck]`, owned-версия
  потребовала бы утечку. Registry-предикаты работают.

### Тесты (9 в `loot_table::number`)

Граничные случаи int/float арифметики (таблица выше), порядок RNG
weighted-list с проверкой следующего результата RNG, `Mth.nextInt`/`nextFloat`
parity, rejection-пути парсера, inline-арифметика, и сквозная проверка:
`cooking/speed_default` = 2.0 и `cooking/time_bamboo` = 25 на smoker, 1.0 и 50
на furnace, а также что все 30 configured значений вычисляются без ошибок.

## Срез 2: печь читает providers (`9efa4655a`)

Раньше печь использовала модель до 26.3: хардкод-таблицу `VANILLA_FUEL_VALUES`
и `kind`-сплит, который делил рецепт и топливо пополам для smoker/blast.

26.3 вместо этого кладёт на предмет топлива компонент `minecraft:cooking_fuel`
с int-провайдером `burn_time` и float-провайдером `speed_multiplier`, а блок
вычисляет их в контексте `CONTAINER_PROCESS`:

- `FurnaceProviders` строит контекст из block state, центра блока и живого
  domain reader. Отсутствие компонента даёт 0 burn time и 1.0 multiplier —
  дефолты Java `getFromItem`. Параметр `CONTAINER` пока не может быть сохранён
  (его ключ не имеет value type), остальные три передаются.
- `total_cook_time` повторяет `getTotalCookTime`:
  `ceil(recipe_time / multiplier)`, при неположительном — исходное время.
- Порядок тика теперь как в Java: оба таймера и множитель записываются **до**
  проверки «горит ли топливо», таймер готовки масштабируется коэффициентом
  завершения при смене скорости топлива, завершение по `>=`, а не `==`.
- `speed_multiplier` сохраняется в NBT, таймеры перешли с short на int (как в
  26.3), `consume_fuel` выбрасывает crafting remainder в мир, если стек
  остался, и топливный слот принимает предметы с компонентом вместо старой
  таблицы по тегу.

Данные дают те же числа, что были в таблице (200/1600 у печи, 100/800 у blast),
но теперь они приходят из `cooking/time_coal` и `cooking/speed_default`,
вычисленных против реального block state. Новые тесты: округление множителя
(включая неположительный) и то, что предмет без компонента не зажигает печь и
не расходуется.

## Проверки

| Команда | Результат |
| --- | --- |
| `cargo test -p vecta-registry --lib loot_table -j 1` | **46 passed / 0 failed** |
| `cargo clippy --locked -p vecta-registry --all-targets --all-features -j 1 -- -D warnings` | exit 0 |
| `cargo test -p vecta-core --lib abstract_furnace -j 1` | **5 passed / 0 failed** |
| `cargo test -p vecta-core --lib data_compute -j 1` | **5 passed / 0 failed** |
| `cargo test -p vecta-core --lib context_provider_argument -j 1` | **1 passed / 0 failed** |
| `cargo clippy --locked -p vecta-core --all-targets -j 1 -- -D warnings` | exit 0 |
| `cargo test --locked --workspace -j 1` | **4326 passed / 0 failed / 18 ignored**, exit 0; `D://CodexBuilds//logs//session09-workspace.log` |
| `cargo clippy --locked --workspace --all-targets --all-features -j 1 -- -D warnings` | exit 0; `D://CodexBuilds//logs//session08-clippy.log`, `session09-*` |
| `cargo fmt --all --check` | exit 0 |
| `python -m unittest discover -s scripts` | 4 tests OK |
| `python scripts/audit_vanilla.py` | 3105 objects / 49 BE / **227** markers (минус снятые TODO furnace), provenance обновлён |

## Незавершённое

### `modify … compute` — реализован

Сквозной путь работает: parsing → provider/reference → общий context →
evaluation → NBT result → существующий manipulator → setData → dirty/sync.

- **Аргументы** (`vecta-protocol/src/packets/game/c_commands.rs`):
  `minecraft:context_float_provider` = 55 и `minecraft:context_int_provider` = 56,
  размещённые по реальному порядку регистрации Java (`slot_source` = 57 остаётся
  без Vecta-аргумента). Никаких вручную придуманных id.
- **Парсеры** (`command/execution/argument.rs`): `parse_context_provider`
  повторяет `ResourceOrIdArgument` — resource key, inline SNBT compound или
  голое число (`ConstantValue.INLINE_CODEC`, `Codec.FLOAT`/`Codec.INT` принимают
  любой numeric tag через `NbtNumeric`). Unknown resource падает на parse с
  `argument.resource_or_id.no_such_element`, неоцениваемый inline payload — с
  `argument.resource_or_id.failed_to_parse`, как в Java.
- **Дерево** (`command/builtins/data/mod.rs`): `<manipulator> compute` →
  `default` / `block <computePos>` / `entity <computeTarget>` →
  `float|integer <provider>` — по `LootContextSources.addContextSources`.
- **Context sources**: `ORIGIN` = позиция источника команд (Java не двигает её
  для block source), `THIS_ENTITY` из command source, `BLOCK_STATE`/опциональный
  `BLOCK_ENTITY` для block, отдельный `TARGET_ENTITY` для entity (не THIS).
  Требуемые/допустимые наборы проверяются настоящим `ContextMap` с
  `COMMAND_COMPUTE_DEFAULT/POSITION/ENTITY`.
- **Данные**: live domain reader (`with_data(world)`), оценка внутри mutable
  level RNG (как `LootContext.Builder.create(Optional.empty())`), никакой
  World→Server ссылки и никаких снапшотов.
- **Запись**: результат — один `NbtTag`, идущий в существующий manipulator
  (`set`/`insert`/`append`/`prepend`/`merge`) через общий `apply_source`:
  прежний single-target guard, запрет записи в players, rollback и
  `SetChanged` сохранены.
- **Парсер провайдеров**: numeric slot'ы (`fallback`, `on_true`, `value`,
  `data`, `inputs`) теперь тоже принимают голые числа — это `INLINE_CODEC`.

Тесты (`server/tests/data_compute.rs`, 5) и юнит-тест парсера:
inline float/int constants, inline compound payload, configured resource на
smoker (2.0) против furnace (1.0), вложенная арифметика (`div(7,2)` = 3.5 через
scale), score provider с живым scoreboard (fallback 42 → живая 5),
unknown resource / unsupported type / malformed SNBT / unloaded block → ошибка
и **нет мутации**.

### Прочие открытые границы

- `environment_attribute` — нужен настоящий API environment attributes.
- owned `LevelBasedValue` для inline `enchantment_level`.
- owned-представление inline-условий (сейчас только registry-предикаты).
- Компостер: у Vecta нет взаимодействия вообще (только comparator level),
  поэтому `compostable/*` провайдеры пока без игрового потребителя.
- `slot_source` argument (protocol id 57) и `CONTAINER` context parameter без
  value type остаются открытыми.
- `CONTAINER` parameter не имеет value type → `CONTAINER_PROCESS` контекст не
  валидируется набором; печь передаёт три из четырёх параметров.
- `ITEM_DURABILITY_CHANGED` criteria — B-06-10 без изменений.

## Следующее действие

1. `/data modify … compute` (см. выше точный объём), начиная с protocol
   вариантов 55/56 и argument parser'ов.
2. Затем компостер как потребитель `compostable/*`.
3. Затем направления 4–12 файла 05 (растения/TreeGrower, recipe book, clouds,
   стрелы, фейерверки, пчёлы, карты, автоматизация).
