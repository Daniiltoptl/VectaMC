# Ревью G022: `/data` и общие NBT paths

Дата: 2026-11-03. Вход: `7a7f68e41`, ветка `codex/night-vanilla-03`.
Проверен игровой коммит `464240cdf`, сопутствующие отчёты и существующие callers.
Исправления ревью: `cd11aa81a`.
Target Cargo.toml и local reference manifest совпадают: Minecraft Java 26.3,
5037 Java files, server SHA1 `33680f5f2ac32864d6d7cf5e56a705fdb3e05f4c`.

## Найденные и исправленные ошибки

1. **Wildcard result count (P1).** `modify_targets` перезаписывал число изменений
   последним выбранным target. `entries[] append` менял два списка, но возвращал 1;
   merge с уже совпадающим последним compound мог вернуть 0 и отменить весь command.
   Теперь результаты суммируются. Проверены два изменённых списка и два изменённых
   compound плюс третий, уже совпадающий.
2. **Числовые массивы (P2).** Java CollectionTag.addTag возвращает false для строки
   в int/byte/long array. Вставка должна пропустить её, не передвинуть index и
   продолжить следующий source. Vecta превращала false в InvalidIndex. Теперь
   проверка типа предшествует проверке индекса, как в Java; отрицательный invalid
   index сообщает вычисленное значение. Проверены mixed sources и incompatible
   source при невалидном index.
3. **Неоднородные wildcard parents (P1).** Рекурсивный обход объявлял отдельную
   неприменимую ветвь ошибкой, даже если соседняя ветвь разрешала весь путь.
   Общий обход родителей теперь учитывает результаты на каждом уровне по всем
   ветвям; final targets для set/insert/merge обрабатываются общей foundation.
   Проверен путь через список из scalar и compound, а также прежние failure tests.
4. **Глубина modify-merge (P2).** Java проверяет compound sources от глубины0,
   а Vecta добавляла длину target path, необоснованно запрещая допустимый source.
   Сохранена отдельная проверка глубины set/insert по длине пути. Regression
   использует source с глубиной511 и nested target.
5. **Float→string (P2).** Приведение f32 к f64 перед Rust formatting превращало
   `0.1f` в `0.10000000149011612f`. Java scientific notation также отличалась.
   Теперь применяется уже существующий canonical SNBT formatter, использующий
   Java Float/Double formatting. Новый тест обнаружил и ошибку самого общего
   formatter: trimming trailing zeroes не увеличивал decimal exponent, меняя
   значение `0.1` на `0.01`. Исправлен `vecta-utils/src/java.rs`; добавлены
   проверки magnitude/round-trip для f32/f64 на разных порядках. Контрольные
   0.1/0.01/10/1e7 и scientific notation сверены запуском установленной Java25
   (`codex-g022-java-floats.log`). Это проверка форматирования JDK, не Mojang-server
   differential и не утверждение о переборе всех возможных IEEE значений.
6. **UTF-16 substring (P2).** `String::from_utf16_lossy` заменял отдельно выбранную
   суррогатную половину emoji символом U+FFFD. Java сохраняет исходный UTF-16 unit.
   Подстрока теперь вырезается по границам MUTF-8 units без потери raw данных;
   get length и string-source без substring также сохраняют unpaired surrogates.
   Проверены обе половины emoji и повторное использование результата.
7. **Лишний rollback.** `load_with_owned_components` возвращает decode error до
   вызова loader. Ошибка не может оставить наполовину загруженную BE. Удалены
   второй save snapshot и ненужный reload с проигнорированной `.ok()` ошибкой.
   Реальное error propagation bridge сохранено; test rejected commands сохранён.

Растущие NBT path tests вынесены в `vecta-utils/src/nbt/path/tests.rs`.
Generated Rust и extracted JSON не редактировались; registry/protocol IDs не менялись.

## Перепроверенные блокеры

- **B-06-08 — ложная диагностика.** ComparatorBlock.get_input_signal уже вызывает
  block analog-output capability; BarrelBlock вычисляет fullness реального
  контейнера. `world/redstone.rs` — отдельный torch tracker. Не нужен новый
  контейнерный расчёт в get_signal. Добавлен server-backed тест `/data` → full
  barrel → scheduled comparator tick → output15 → remove Items → output0.
  Возрастание проверяется через очередь scheduler: до двух тиков output0,
  затем выбор due tick и реальный behavior.tick. После remove проверяются
  постановка нового tick и его обработчик; это не живой серверный restart.
- **B-06-09 — открытая реализация.** Detached item custom-data load действительно
  не резолвит attached lectern. Но `Player::server()` уже существует, а явный
  BlockEntityLoadContext согласован ранее. Новая World→Server ссылка не нужна;
  предлагать ограниченный resolver вместо storage/score/selectors неправильно.
  Передача контекста placement caller остаётся первым срезом следующего пакета.
- **B-06-06:** есть Entity subtype loaders и typed disk factories, но полноценного
  общего live entity NBT load для `/data entity` ещё нет. `Entity::apply_spawn_data`
  уже читает Pos/Motion/Rotation/Air/Fire/tags/флаги и вызывает subtype loader;
  нельзя снова писать эти части с нуля. Но его partial-update семантика не
  сбрасывает отсутствующие поля, а `set_position_local` не заменяет live world
  tracking. Полный load/UUID/passenger contract и интеграция остаются задачей,
  с согласованием действительно неясных lifecycle решений по AGENTS.
- **B-06-07:** compute присутствует в Java26.3; ContextFloat/Int providers и
  LootContextSources пока отсутствуют в соответствующих Rust loot/command paths.
  Не подменять их worldgen IntProvider. Добавлены в следующую рабочую очередь.

G022 остаётся частичной: block/storage работают, entity и compute не реализованы.
Item custom-data context и настоящий region I/O для G021 также остаются открыты.
Правильный порядок ordinary reload и already extracted onlyOpCanSetNbt сохранён.

## Проверки

Узкие проверки Codex: NBT paths **31 passed**, весь vecta-utils **228 passed**.
Первый core filter `data` выявил описанный formatter defect (142 passed/1 failed);
исправление внесено в общий formatter, а не в expected value теста.
Итоговый `cargo test --locked --workspace -j 1`: **4279 passed / 0 failed /
18 ignored**, exit0; сборка 8m01s. Оба расширенных command string tests и
server-backed comparator test прошли в полном прогоне. Входящий отчёт был4273;
добавлены пять utility regression tests и один server-backed test.

`cargo fmt --all --check`: exit0. `python -m unittest discover -s scripts`:
4 tests OK. Audit в отдельный каталог вне репозитория: 3105 objects /49 BE /
232 markers; это инвентаризация регистраций, не процент vanilla readiness.
`cargo clippy --locked --workspace --all-targets --all-features -j 1 -- -D warnings`:
**exit0**, 4m26s. Cargo выдал одно manifest warning об unused optional `mimalloc`:
`--all-features` включает `dhat-heap`, а существующий allocator cfg намеренно
исключает mimalloc в этом режиме. В обычной default-feature сборке зависимость
используется; её удаление изменило бы конфигурацию аллокатора. Ничего не
подавлялось, manifest/allocator не менялись. Это не полностью бесшумный прогон.
Логи — `D:\CodexBuilds\logs\codex-g022-*`.

Настоящий клиент26.3, Linux, Mojang differential, два сетевых клиента и живой
restart/unload не запускались. EULA не принималась. Server-backed tests используют
настоящие Server/World/Player и внутреннее packet capture, не сетевую сессию клиента.

## Продолжение

Актуальный большой запрос — `WORKBUDDY_CONTINUE_06_05.md`: 12 сквозных направлений,
более150 существующих карточек. Не тратить следующий сеанс на повторную реализацию
готовых рамок или очередной общий план. Итог исполнителя — `report-next-05.md`.

Два чужих файла сохраняются без изменений/индексации:

- `planning/night-01/report.md`:
  `06680CBC971245B70C990D95C906F0F1E3685EFFDE61C13D562664B4B6B05222`;
- `WORKBUDDY_START_06.md`:
  `48430DC1D56AF1E679E0A81411518002B8C406B3F27648AD1A197DF174AFAD5A`.
