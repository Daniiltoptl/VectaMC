# Сеанс07: единый typed ContextMap и миграция loot

Дата:2026-11-03. Вход `fb9e96dfa`, ветка `codex/night-vanilla-03`.
Код первого среза: `2a0ef1daf`. Workspace **4304/0/18**, Clippy/fmt exit0.

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
он не зарегистрирован как работающая provider factory. Configured provider
values ещё не извлечены. Полная реализация providers/compute не выполнена.

CONTAINER сейчас описывает реальную metadata SlotProvider в param sets. Fake
SlotProvider/value consumer не добавлен. Прежние Score/Storage provider stubs,
LocationCheck и `_interacted_block_entity` TODO этим срезом не объявляются
исправленными. Новый transport не является полным loot parity.

## Следующий архитектурный вопрос

Проверены World, DomainScoreboards/DomainCommandStorage, helpers execute и Java
StoredNumberAccess. Scoreboard/storage живут у Server по domain, World имеет
только entity directory и не имеет read API к этим данным. Для общих loot/
compute providers предложен отдельный domain read context при инициализации
World, с shared Arc владением живыми scoreboard/storage, без World→Server
ссылки/копий состояния. Пользователь выбрал «Общий контекст чтения домена
(рекомендую)»: **решение одобрено, можно реализовывать**. Это согласование
владения domain services дополняет прежнее согласование единой ContextMap;
оба выбора повторно не спрашивать.

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
