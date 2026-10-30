# W03-01 · исправления recipe codecs после ревью

Дата: 2026-10-30. База: `b63fde9f531e2815bc55e098efce693ae519f329`,
ветка `codex/night-vanilla-03`, Minecraft Java 26.3.

Владелец попросил изменить код сейчас, а тесты и остальные прогоны отложить.
**Сборка, type-check, тесты и Clippy этих исправлений не запускались.**
Предыдущие зелёные результаты относятся к прежним коммитам и не подтверждают
этот diff. Новый runtime regression suite также отложен.

## Исправления в коде

### Ссылка и inline-значение trim pattern

`SlotDisplay::SmithingTrim.pattern` теперь `RegistryHolder<TrimPattern>`.
Запись/чтение делегированы существующему общему codec. Для registry reference
он пишет `id + 1`, для direct value — `0` и полное значение.
Ранее записывался raw ID и читался такой же ID: клиент трактовал ноль как
начало inline-значения, а положительный ID — как ссылку на предыдущий pattern.

Эталон: `SlotDisplay.SmithingTrimDemoSlotDisplay.STREAM_CODEC`,
`TrimPattern.STREAM_CODEC`, `ByteBufCodecs.holder` в локальных исходниках 26.3.

### Полный holder set в TagSlotDisplay

`SlotDisplay::Tag` теперь содержит `RegistryHolderSet<Item>`, а не Identifier.
Codec поддерживает именованный тег, прямой список, пустой и одноэлементный
прямой набор через существующий тип. Ранее direct-наборы отвергались.

Эталон: `SlotDisplay.TagSlotDisplay.STREAM_CODEC`, `ByteBufCodecs.holderSet`.

### Runtime-список ингредиентов

`SingleInputEntry.input` теперь `RegistryHolderSet<Item>`. Декодирование больше
не пытается поместить runtime Vec в `Ingredient::Choice(&'static [ItemRef])`
и не отвергает списки из нескольких items.
`SingleInputEntry::from_ingredient` преобразует typed recipe ingredient для
синхронизации. Ненужные дублирующие holder-set read/write helpers удалены;
общий `Ingredient` для рецептов и его генераторы не менялись.

Эталон: `SelectableRecipe.SingleInputEntry.noRecipeCodec`,
`Ingredient.CONTENTS_STREAM_CODEC` и `ByteBufCodecs.holderSet`.

Потребитель в `synchronized_stonecutter_recipes` переведён на конструктор.
Существующие тестовые вызовы адаптированы к новому типу поля; для named-set
fixture взят уже существующий generated `ItemTag::WOOL`.
Эти адаптации не считаются новым покрытием и не объявляются прошедшими тестами.

## Открытый вопрос: источник ID slot_display

Ручные значения `SlotDisplayKind` не исправлены этим коммитом. Это отдельная
незакрытая проблема общей основы, а не разрешённый prototype или необязательная
оптимизация. Требуются извлечённые данные `slot_display` для 26.3, генерация
и согласованный с общей registry architecture путь расширения.

Готового output `slot_display` в repo build assets нет. Checkout SteelExtractor
не найден в проверенных ближайших каталогах; владелец пути не знает.
Не добавлялся альтернативный ручной список или parser Java вместо extractor.
До закрытия этого пункта общая SlotDisplay foundation не считается завершённой.

## Отложенные проверки

Когда проверки будут возобновлены:

- Registry reference и direct trim pattern: точные байты и read/write round-trip.
- TagSlotDisplay: named/direct/empty/singleton sets.
- SingleInputEntry: несколько items, пустой набор, tag и вложенные displays.
- Корректный отказ общего holder codec при неверных ID/размерах.
- Существующие packet tests и совпадение порядка списка камнереза с меню.
- Relevant registry/protocol/core checks и Clippy после узких тестов.

Миры, Minecraft-сервер и клиент не запускались; EULA не принималась.
Память VM и настройки сборки не менялись.
