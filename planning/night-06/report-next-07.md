# Сеанс07: typed ContextMap и живые данные домена для loot

Дата:2026-11-03. Вход `fb9e96dfa`, ветка `codex/night-vanilla-03`.
Код первого среза: `2a0ef1daf`. Workspace **4304/0/18**, Clippy/fmt exit0.

Продолжение 2026-11-04: код `70f27a545`, workspace **4308/0/18**.
Общий контекст чтения домена реализован по согласованному решению.

## Реализованный срез

Вместо разрозненных параметров в LootContext используется одна ContextMap с
типизированными lifetime-aware ключами. Заимствования ItemStack, EntityRef,
BlockEntityRef и DamageSourceInfo сохраняются. RNG, luck и состояние уровня
(game_time/weather) остаются отдельными, как LootContext/LootParams/ServerLevel
в Java. Второго параллельного контекста для compute нет.

`LootContextRef` удалён; number provider readers читают ту же карту. Переведены
conditions/functions, добыча блоков, block interaction, контейнеры, archaeology,
death/shearing/gift, fishing и advancement rewards, а также существующие tests.

Ключи включают имя и value family, поэтому одно имя с другим Rust типом не
перезаписывает и не удовлетворяет правильному ключу. Extension values
заимствуются через существующий keyed Downcast; новый production unsafe и
Any/TypeId не добавлялись. Семейства сохраняют Send/Sync. Optional None удаляет
параметр; validate сначала проверяет not-allowed, затем missing-required.

Статические игровые constructors проверяют реальные param sets; нарушение
здесь является ошибкой программной сборки контекста, как IllegalArgumentException
из Java Builder.create. Recoverable API validate/get_required возвращает Result
для командных consumers. User-facing compute пока отсутствует.

## Сверка и исправленные расхождения

- Chest открывший игрок передаётся как THIS_ENTITY, а не INTERACTING_ENTITY.
- KilledByPlayer проверяет наличие LAST_DAMAGE_PLAYER. Независимый bool удалён;
  его нельзя рассинхронизировать с реально разрешённой ссылкой на игрока.
- Block loot получает TOOL даже при пустом инструменте и ORIGIN в центре блока.
- Block interaction получает позицию центра блока. Cave vines используют общий
  путь с berry bush/pumpkin/block transformer вместо отдельной сборки без origin.
- Registry/context metadata сгенерирована из реального SteelExtractor output:
 16ключей и31набор. Никаких вручную заданных registry IDs и runtime JSON loader.

Primary sources: Java26.3 ContextMap/ContextKey/ContextKeySet, LootParams,
LootContextParams/LootContextParamSets, LootItemKilledByPlayerCondition,
RandomizableContainer.unpackLootTable, Block.getDrops/dropFromBlockInteractLootTable
и текущие Vecta callers. Cargo target и reference manifest совпадают:26.3,
5037Java, server SHA1 `33680f5f2ac32864d6d7cf5e56a705fdb3e05f4c`.

## Extractor и оставшийся объём

Два новых Java entry points выполнены из local SteelExtractor через Java25
source-file launch и подготовленные 26.3 classes/libraries. MinecraftServer
не запускался, EULA не принималась. Produced loot_contexts.json скопирован
побайтово, hash совпадает; source/reproduction в scripts/steel-extractor.

Независимо подготовлен output типов number providers: **28float+23int=51**,
а не49. NumberDispatcher был пропущен в обеих группах прежнего списка.
Этот output остаётся в `D:\CodexBuilds\SteelExtractor\run\steel_extractor_output\steel-registry\build_assets\context_number_provider_types.json`;
он не зарегистрирован как работающая provider factory. В продолжении 2026-11-04
отдельным entry point извлечены4float+26int configured values с registry-aware
codec round-trip. Они также остаются в output до реализации consumer, см.
scripts/steel-extractor/loot-contexts.md. Полная providers/compute не выполнена.

CONTAINER сейчас описывает реальную metadata SlotProvider в param sets. Fake
SlotProvider/value consumer не добавлен. LocationCheck, Reference и
`_interacted_block_entity` TODO не объявляются исправленными. Новый transport
не является полным loot parity. Score/Storage и EntityScores подключены во
втором срезе ниже; это не реализация всей современной provider factory.

## Согласованное владение данными домена

Проверены World, DomainScoreboards/DomainCommandStorage, helpers execute и Java
StoredNumberAccess. Scoreboard/storage живут у Server по domain, World имеет
только entity directory и не имеет read API к этим данным. Для общих loot/
compute providers предложен отдельный domain read context при инициализации
World, с shared Arc владением живыми scoreboard/storage, без World→Server
ссылки/копий состояния. Пользователь выбрал «Общий контекст чтения домена
(рекомендую)»: **решение одобрено, можно реализовывать**. Это согласование
владения domain services дополняет прежнее согласование единой ContextMap;
оба выбора повторно не спрашивать. Решение реализовано во втором срезе.

## Второй срез: live scoreboard/storage и настоящие consumers

DomainScoreboards и DomainCommandStorage хранят Arc существующих backend-объектов.
При инициализации production Server и тестового Server все миры получают
read context своего домена. У миров одного домена общие данные; разные домены
изолированы. Ссылка World→Server не добавлена, формат сохранения не изменён.
Read context не кэширует вторую копию scoreboard/storage. Чтение NBT использует
существующий CommandStorage API с временным значением для NbtPath.

EntityRef теперь передаёт отдельный score-holder: profile name для игрока и
UUID для остальных сущностей. Display name и тип сущности не подставляются.
Lookup не создаёт отсутствующий score. Добавлены TARGET_ENTITY и актуальные
26.3 имена ролей. Все текущие игровые constructors передают live reader.

Score/Storage number providers, EntityScores, loot functions, ranges, rolls и
entries используют полный LootContext. Целочисленные значения не проходят
через f32: scoreboard сохраняет точность i32, storage использует Java boxed
Number-семантику существующего NbtNumeric (long narrowing, truncation double).
Storage требует ровно один numeric result NbtPath. Missing score/objective не
считается настоящим нулём при EntityScores; fallback provider остаётся нулевым.
Чтения не расходуют RNG. Scale удалён из Score: в ScoreboardValue 26.3 его нет.

Сверены локальные StoredNumberAccess, ScoreboardValue, StorageValue,
EntityHasScoreCondition, IntRangePredicate и score-holder имена Entity/Player.
Новая LootDataReader capability не содержит no-op методов.

Четыре сквозных Rust server tests проверяют live update между двумя мирами,
изоляцию другого домена, отсутствие побочных score entries, неизменность RNG,
`/data merge/modify/remove storage` → SetCount, boxed casts, multi-match и
non-numeric paths, profile/UUID identity и EntityScores с отсутствующим objective.

Проверки окончательного кода второго среза:

- `cargo test --locked --workspace -j 1`: **4308passed/0failed/18ignored**;
  `D:\CodexBuilds\logs\codex-session07-domain-workspace.log`.
- `cargo test --locked -p vecta-core --lib server::tests::loot_data -j 1`:
  **4passed/0failed**; `D:\CodexBuilds\logs\codex-session07-domain-tests.log`.
- `cargo clippy --locked --workspace --all-targets --all-features -j 1 -- -D warnings`:
  exit0; `D:\CodexBuilds\logs\codex-session07-domain-clippy.log`.
- `cargo fmt --all --check`: exit0; Python4tests OK.
- Audit после code commit: **3105objects/49BE/227markers**.

Следующий срез — реальные 51 provider types/codecs, configured provider values
и compute, затем оставшиеся направления большого ТЗ. Старый NumberProvider
ещё не заменён полной ContextFloatProvider/ContextIntProvider системой.
Context-free helpers не являются контекстными consumers. Reference/location
predicate foundations, SlotProvider и environment readers остаются открытыми;
их нельзя заменять permissive predicates ради Conditional provider.

При подготовке следующего среза найден старый fallback build generator:
неизвестный тип NumberProvider превращался в Constant(value_or_1). Теперь
неизвестный тип и отсутствие обязательных полей дают ошибку сборки с причиной.
Текущие assets используют766uniform и18binomial объектов; их поддержка
сохранена. Современные неподдержанные типы не маскируются готовыми константами.
Это проверка extracted input, а не новый полный codec dispatch.

Код подготовки/генератора `8b8140b35`. После этой последней правки:
`cargo test --locked -p vecta-registry --lib loot_table -j 1` — **37/0/0**,
`cargo clippy --locked -p vecta-registry --all-targets --all-features -j 1 -- -D warnings`
и `cargo fmt --all --check` — exit0. Логи:
`D:\CodexBuilds\logs\codex-session07-provider-generator-tests.log` и
`codex-session07-provider-generator-clippy.log`.
Полный workspace4308/0/18 относится к второму срезу `70f27a545`; после guard
генератора повторён затронутый loot suite, весь workspace заново не запускался.
Audit после последнего code commit:3105objects/49BE/227markers.

## Проверки и границы

Входящий проверенный workspace4299/0/18. Узкий loot suite после миграции:
37passed,0failed; общий type-check core/lib/tests прошёл. После разделения map,
key families и registry metadata по файлам исправлено их подключение; затем
проверки повторены на окончательном коде:

- `cargo test --locked --workspace -j 1`: **4304passed/0failed/18ignored**,
  exit0; `D:\CodexBuilds\logs\codex-session07-workspace-final.log`.
- `cargo clippy --locked --workspace --all-targets --all-features -j 1 -- -D warnings`:
  exit0; `D:\CodexBuilds\logs\codex-session07-clippy-final-03.log`.
- `cargo fmt --all --check`: exit0.
- `python -m unittest discover -s scripts`:4tests OK.
- Audit после code commit:3105objects/49BE/230markers.

Clippy потребовал исправить lifetime elision, явные imports генератора,
документацию panic инвариантов и преобразование pointers в тестах. Lints не
отключались. Cargo manifest warning unused mimalloc при all-features/dhat
сохранился; это exit0 с известным warning, не полностью безмолвный прогон.

Добавлены5значимых tests: wrong type/validation priority, optional removal/
chest entity key, borrowed extension ownership, player presence и общий seeded
RNG call order. Старые seeded/table evaluation order tests сохранены.

Настоящий client26.3, Linux, два сетевых игрока, полный Mojang-server comparison
и live restart/unload не выполнялись. Foreign report.md/WORKBUDDY_START_06.md
не менялись и не stage; сверять их hashes перед передачей.
