# Ревью сеанса06: living load/save, Team/UUID и break callback

Дата: 2026-11-03. Проверен incoming диапазон `65716df85..e1efc174b`, ветка
`codex/night-vanilla-03`, рабочая копия `C:\Users\My comp\Desktop\Vecta`.
Источник отчёта: `planning/night-06/report-next-06.md`.

## Результат

Сеанс добавил полезный общий living loader и реальный region round-trip;
Team использует существующий domain scoreboard, break callback проходит через
Player вне inventory lock. Это сохранено. По ревью исправлены реальные
расхождения кодеков, packed attributes, raw effect load, сна и combat references.
Это не полная готовность G024: Brain и locator-bar icon ещё отсутствуют.

Код исправлений: `a91820a33`. Итог: **4299 passed /0 failed /18 ignored**,
Clippy/fmt exit0. Cargo manifest warning о mimalloc сохранён, см. ниже.

## Находки и исправления

1. **Числовые codecs и partial results.** Входящий decoder требовал некоторые
   значения строго определённого NBT типа, использовал TagValueInput conversion
   вместо Codec conversion и мог терять весь список effects из-за одного
   невалидного элемента. Java принимает числовые base/amount, INT truncates
   дробь, BOOL проверяет любое numeric nonzero, unsigned-byte amplifier сужает
   числовой ввод. Optional malformed поля могут давать partial default, а
   list codec сохраняет удачные элементы. Исправлены `living_data.rs` и
   `living_base/mod.rs`; регрессии идут через настоящий `/data merge entity`.
   HurtTime/DeathTime дополнительно сужаются до short, как getShortOr.
2. **Packed attributes нельзя применять обычным replace.** Java поддерживает
   независимые ID/operation/permanent индексы. После смены operation для одного
   ID Packed.apply прежний operation entry продолжает участвовать в расчёте.
   На base10, старом ADD_VALUE2 и новом MULTIPLIED_TOTAL0.5 Java даёт18;
   remove текущего ID оставляет12. Старый Rust set_modifier давал15→10.
   Добавлены соответствующие индексы и отдельный apply_packed, unconditional
   dirty sync и проверка18→12→14→10. Immutable modifier records копируются в
   независимые ownership индексы по смыслу данных, не ради borrow checker.
   Старые attribute tests перенесены без потери в отдельный tests.rs.
3. **Raw effect load не является игровым добавлением/удалением эффекта.**
   Java очищает map и вставляет decoded entries напрямую, выставляя effectsDirty
   при вставке. Входящая реализация дополнительно ставила update/remove packets
   и безусловный dirty flag. Эти побочные действия убраны; отдельная проверка
   подтверждает отсутствие gameplay packet queue после NBT load. Permanent
   modifiers эффектов, добавленные исполнителем, оставлены: Java действительно
   использует addPermanentModifier.
4. **Загрузка сна не должна заново начинать сон.** Входящий start_sleeping
   менял OCCUPIED, velocity и riding. Java load вызывает только setPosToBed.
   Выделено перемещение к поверхности кровати, общий расчёт координат сохранён.
   Regression на настоящей кровати проверяет reposition, сохранение Motion и
   отсутствие изменения OCCUPIED. Обычный start_sleeping по-прежнему выполняет
   игровые действия и помечает velocity sync.
5. **Combat references и expiry.** Загруженная UUID-only ссылка на mob могла
   разрешиться в ItemEntity и не попадала в expiry, пока к ней никто не
   обращался. Теперь resolution требует living capability, ищет по всем мирам
   текущего domain и выполняется перед combat-memory expiry. Player UUID fallback
   также использует domain lookup. Memory timer загружается независимо от
   присутствия UUID, как Java; save timers остаётся условным при наличии ссылки.
   Regression проверяет nonliving target и истечение старой загруженной ссылки.
6. **UUID и BlockPos codecs.** Входящие readers UUID и sleeping_pos принимали
   только IntArray, тогда как Java codecs принимают numeric stream и valid
   partial prefix при избыточной длине. Теперь используется общий decoder
   числовых collections с проверкой всего stream до использования prefix.
   Нечисловой tail не превращается в допустимый UUID или BlockPos. Sleep test
   проверяет overlong double list и invalid tail. UUID reader общий для
   Team/combat refs; live entity UUID по-прежнему сохраняется accessor-ом и не
   заменяется payload UUID.

## Первичные источники и исполняемая сверка

Target подтверждён Cargo suffix `0.16.1+mc26.3` и
`minecraft-src/.vecta-reference.json`:26.3,5037Java, server SHA1
`33680f5f2ac32864d6d7cf5e56a705fdb3e05f4c`.

Прочитаны LivingEntity.readAdditionalSaveData/baseTick/startSleeping/setPosToBed,
AttributeInstance.apply/removeModifier и MobEffectInstance/Details codecs,
MobEffect.addAttributeModifiers, EntityReference, UUIDUtil/Util.fixedSize,
TagValueInput, DataCommands/EntityDataAccessor и соответствующие Vecta callers.

Помимо исходников запущены standalone Java25 oracles с настоящими классами
26.3 из `.vecta-cache/vanilla/26.3/minecraft-server.jar` и его libraries.
Registry Bootstrap выполнялся без запуска MinecraftServer; EULA не принималась.
Исходники/вывод вне repo, каталог `D:\CodexBuilds\logs`:

- `CodexSession06CodecOracle.java`, `codex-session06-codec-oracle.log`:
  BOOL256/0.5=true, INT-1.9=-1, byte255=-1, numeric double40; ListCodec partial
  сохраняет1,3 вокруг invalid элемента; UUID overlong numeric stream имеет
  partial prefix, а nonnumeric tail не имеет partial результата. BlockPos на
  [32.8,65.9,30.1,999] даёт partial (32,65,30), nonnumeric tail отменяет partial.
- `CodexSession06LivingCodecOracle.java`, `codex-session06-living-codec-oracle.log`:
  numeric effect fields, optional malformed defaults, numeric packed attributes
  и partial list эффектов проверены настоящими codecs.
- `CodexSession06AttributeOracle.java`, `codex-session06-attribute-oracle.log`:
  настоящий AttributeInstance даёт18 после Packed.apply и12 после remove.

Это точечная сверка классов/кодеков с Mojang-кодом, не сравнение полного
работающего сервера. JOML выводил предупреждения deprecated Unsafe; они не
скрывались и не относятся к новому Rust unsafe. Runtime logs oracle перенесены
из repo в `D:\CodexBuilds\logs\codex-session06-oracle-runtime`.

## Проверки

Входящие **4293 passed/0 failed/18 ignored** — результат исполнителя.
Наш первый узкий прогон: attribute8, living_load6, оба exit0. Затем полный
workspace прошёл; Clippy нашёл исправленные замечания: missing const, две
redundant closure и if_not_else в тесте. После окончательных правок запущена
финальная проверка, включая расширенный BlockPos regression.

Окончательный код после всех правок:

- `cargo test --locked --workspace -j 1`: **4299 passed /0 failed /18 ignored**,
  exit0; `D:\CodexBuilds\logs\codex-session06-verified-workspace.log`.
- `cargo clippy --locked --workspace --all-targets --all-features -j 1 -- -D warnings`:
  exit0; `D:\CodexBuilds\logs\codex-session06-verified-clippy.log`.
- `cargo fmt --all --check`: exit0.
- `python -m unittest discover -s scripts`:4tests OK.
- `python scripts/audit_vanilla.py`, после code commit:3105objects,49BE,
  230source markers. Это инвентаризация исходников, не процент vanilla parity.
- `git diff --cached --check`: exit0 для собственного code commit. В чужом
  `planning/night-01/report.md` есть прежние whitespace замечания; файл не правился.

All-features Clippy по-прежнему печатает Cargo manifest warning `unused dependency
mimalloc`: при dhat-heap allocator mimalloc не используется. Команда завершилась
exit0; это не полностью безмолвный прогон. Зависимости/profiles/lints не менялись.
После роста suite с incoming4293 добавлено6значимых tests, а не4299новых сценариев.

Новые tests не утверждают наличие настоящего клиента или сетевого round-trip.
Linux, client26.3, два реальных сетевых игрока и live restart/unload/tickets
не проверялись. Существующие region I/O tests не подменяют эти acceptance checks.

## Следующий архитектурный срез

Пользователь ответил «что считаешь лучше» на выбор контекста compute. Codex выбрал
и сообщил: **единый типизированный ContextMap в существующем LootContext,
миграция всех текущих loot callers, сохранение shared seeded RNG и domains**.
Это согласованное решение, повторное разрешение на этот путь не требуется.
В текущем review compute не реализован; следующий consumer — эта foundation,
реальные provider registries и `/data modify … compute`.

Уточнение к старым docs: у providers есть реальные TYPE/configured registries;
inline constant codec их не отменяет. Данные при необходимости получать через
SteelExtractor. Старое утверждение «extractor не нужен, это только inline codecs»
не является актуальным контрактом. Lifetime-safe дизайн и точные param sets
выводить из текущих callers и Java, не вводить вторую контекст-модель.

[Следующее большое задание](../../WORKBUDDY_CONTINUE_06_07.md) сохраняет всю
очередь12направлений/600карточек. Brain/locator и criteria остаются открытыми
срезами, наличие регистрации `/data entity` не означает full vanilla load.
Чужие два dirty файла не изменялись и не включались в commits.
