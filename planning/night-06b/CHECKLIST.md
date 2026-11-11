# Чек-лист ревью Claude (не повторять)

Перед каждым срезом пройди этот список. Каждый пункт уже стоил исправления.

**Java — единственный источник.**
- Перед словами «раньше было неправильно» прочитай конкретный Java-метод и ВСЕ его
  вызовы. Код Vecta с комментарием о ванили верен, пока Java не доказала обратное
  (ПКМ по результату крафта забирает весь стак: так в 26.3).
- Сверяй ВСЕ методы класса блока/предмета: `getStateForPlacement`, `updateShape`,
  `canSurvive`, `useItemOn`, `useWithoutItem`, `tick`, `randomTick`, `onPlace`,
  `triggerEvent`, `getShape`/collision/interaction, `getAnalogOutputSignal`,
  `affectNeighborsAfterRemoval`, а не только основной сценарий.
- Данные (имена, ID, шансы, веса, теги) — из extracted/generated, а не из памяти.
  Нет данных: обнови SteelExtractor (`D:\CodexBuilds\SteelExtractor`), скопируй
  выдумывай. Руками registry/protocol ID не вписывать.

**Арифметика и числа.**
- Знаки операндов: floorDiv/floorMod — не Евклид; проверяй отрицательный делитель.
- `subtractExact`, `addExact`, `multiplyExact` падают при переполнении, не заворачивают.
- Float: Java считает `int/float` и `float*int` во float. Не расширяй до double.
  `Math.max/min` сохраняют NaN, `Math.pow(1, NaN)` = NaN. Есть `java_max`,
  `java_min`, `java_pow`.
- Типы NBT: таймеры печи — int, не short. Проверяй round-trip значением вне
  диапазона short. Заявленное в отчёте проверяй по коду.

**Случайность.**
- Для каждого вызова выясни, чей `RandomSource` берёт Java: level random, `random`
  из `tick`/`randomTick`, random самой сущности. Лишний вызов level random сдвигает
  мировой поток. Новая `ItemEntity` берёт скорость из СВОЕГО random, не из level.
- Seeded vanilla RNG обязателен для worldgen, лута, детерминизма. Живая случайность
  геймплея — unseeded runtime RNG, но при той же вероятности.
- Внутри `world.random().with_level(...)` только вычисления. НЕЛЬЗЯ писать в мир,
  планировать блок-апдейты, звать `apply_source`: блокировка нереентерабельна,
  сервер повиснет. Результат вынеси и пиши после.
- Тест, который зелёный только при удачном броске, недопустим: фиксируй seed или
  проверяй инварианты.

**Контейнеры и автоматизация.**
- `BlockEntity::container_ref` = ванильное `instanceof Container`. Не отдавай его
  блок-сущностям, которые в Java не Container (кафедра: ломала воронки).
- Каждый новый `RandomizableContainerBlockEntity` добавляй в `unpack_pending_loot`
  (hopper.rs) и в проверку `canOpen`.
- Если Java-контейнер наследует `SimpleContainer`, `set_item` и `remove_item` сами
  вызывают `set_changed`. Трейт `Container` в Vecta этого не делает: переопредели.
- `WorldlyContainerHolder` проверяется ДО блок-сущности (порядок `getBlockContainer`).

**Отчёты и тесты.**
- В отчёте указывай итог ВСЕГО workspace (сумма всех `test result`), не одного пакета.
- Заявленное «сделано» проверяй по коду: отчёт однажды говорил про int-таймеры,
  а были short; говорил про три параметра контекста, а передавалось два.
- Тесты проверяют ванильные ожидания из Java, а не текущее поведение кода. Не
  закрепляй ошибку тестом (так случилось с результатом крафта).
- Используй `RecordingConnection`: проверяй реально отправленные пакеты, а не вызовы.
- Полный маршрут: packet → handler → server state → outgoing sync → save/load.
- Частичное помечай частичным. Не называй готовым то, чего нет. Клиент 26.3 вживую
  не проверялся: так и пиши.

## Правила AGENTS (коротко)

Vanilla 1:1 для геймплея, протокола, реестров, worldgen. Никаких стабов, fake
vanilla data, `todo!()`, ручных ID, правок generated/extracted. Без `.clone()` ради
borrow checker, `.unwrap()/.expect()` в production, `unsafe` (кроме keyed
`DowncastType` с уникальным ключом и `// SAFETY:`), `Any`/`TypeId`. Clippy
подавляй только `#[expect(..., reason = "...")]`. Game tick не ждёт async, не
спит, не опрашивает в цикле: используй job system. Не добавляй тесты ради
покрытия. Vanilla-имена для идентификаторов. Файлы небольшие, с разделением
ответственности.

