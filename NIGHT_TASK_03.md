# VectaMC — большой пакет разработки 03 для WorkBuddy / DeepSeek

> **Продолжение после ревью Codex: 30 октября 2026.** Это очередь из
> **72 игровых задач** для последовательных сеансов. Рабочая ветка уже
> `codex/night-vanilla-03`, база — её текущий tip. Не создавай её повторно
> от Night 02. W03-01..05 и список рецептов камнереза уже реализованы серверно;
> актуальные исправления и ограничения — в
> [ревью](planning/reviews/night-03-follow-up.md) и
> [стартовом запросе](WORKBUDDY_START_03.md). Последний полный baseline на
> `015152051`: **3890 passed / 0 failed / 18 ignored**, включая окончательный
> load-context diff и пять новых тестов. B-W03-07 закрыт. Настройка подкачки D:
> 16 ГБ сохранена, но не применена; тесты прошли без перезагрузки по явному
> запросу владельца. Следующий игровой ID — **W03-06**. Историческая база ниже сохранена
> для происхождения пакета, а не как команда откатить новые изменения.
>
> Не обещай выполнить весь пакет за ночь и не заканчивай после первой успешной
> механики. Уже существующие книги, мешки и entity pick не реализуй заново.

## 1. Задача владельца

Развивать VectaMC как Minecraft Java сервер на Rust с наблюдаемым поведением
vanilla 26.3. Нужны реальные игровые действия: предмет используется, меню
работает, результат синхронизируется, состояние переживает сохранение.
Список файлов, регистрация поведения и зелёная компиляция сами по себе этого
не доказывают. Плагины, другие версии и новый ренейм сейчас вне задания.

Прочитай этот файл целиком один раз. Дальше держи короткий журнал и перечитывай
только текущий этап и нужные исходники. Не трать сеанс на переписывание плана,
общий поиск всех TODO или повторную проверку уже проверенного без причины.

Порядок: проверенное основание → законченный игровой сценарий → узкие проверки
→ понятный коммит → push своей ветки → следующий доступный сценарий.
Не возвращай финальный ответ после одного пункта, если есть ресурс и независимая
задача с достаточными основаниями. В конце обязательно дай текстовый ответ
с итогами и ссылкой на отчёт, а не только созданные файлы.

## 2. База и сохранность работы

Для продолжения после ревью используй текущий tip `codex/night-vanilla-03`.
Значения ветки и SHA ниже — снимок исходной передачи 29 октября;
правила сохранности работы по-прежнему действуют.

- Каталог: `C:\Users\My comp\Desktop\Vecta`.
- Репозиторий: https://github.com/Daniiltoptl/VectaMC.
- Ветка передачи: `codex/night-vanilla-02`.
- Проверенный игровой коммит:
  `ac8c2d7887046748c2050513b36e56df5af972bf`.
- После него добавляется этот пакет отдельным документационным коммитом.
  Начинай с актуального tip, который содержит исправления и пакет.
- Создай от этого состояния `codex/night-vanilla-03`. Если такая ветка уже
  существует как продолжение этой работы, сначала проверь историю и продолжай
  её. Не перезаписывай одноимённую чужую ветку и не откатывай новые коммиты.
- Не делай merge в main, push в main, force-push, reset/clean всей папки.
- До любых изменений сохрани `git status --short`, branch и полный HEAD.
- Уже существует чужое изменение `planning/night-01/report.md`.
  **Не редактируй, не stage, не отменяй и не прячь его через общий stash.**
  На момент передачи SHA256 файла:
  `06680CBC971245B70C990D95C906F0F1E3685EFFDE61C13D562664B4B6B05222`.
  Если владелец изменил файл позже, исходным считай фактический стартовый hash.
- Stage только свои файлы или свои hunks. Перед коммитом читай staged diff.
- Исходники Minecraft, JAR, локальные кэши, миры, токены и логи не коммить.
- Не меняй AGENTS.md, правила проекта, toolchain и зависимости ради обхода
  проверки. Не запускай других агентов или Gemini/Mistral CLI.

Сначала прочитай `AGENTS.md`, `planning/reviews/N03-N19.md`,
`planning/reviews/N01-N02.md`, `planning/reviews/V05.09a.md`, `CHANGELOG.md`,
`VANILLA_PLAN.md`. Предыдущие NIGHT_TASK — исторические документы; актуальная
база и очередь находятся здесь. Не переноси старые отметки «не реализовано»
на уже существующий код и не ставь родительской группе «готово» за подзадачу.

## 3. Версия, эталон и подтверждённое состояние

Авторитет версии — `[workspace.package].version` в `Cargo.toml`:
на момент передачи `0.16.1+mc26.3`. Эталон подготовлен штатным
`update-minecraft-src.sh`; корень Java —
`minecraft-src/minecraft/src/net/minecraft/`.
В `minecraft-src/.vecta-reference.json` указаны версия 26.3 и 5037 Java-файлов.
Проверь совпадение с workspace. Текущий совпадающий эталон не скачивай заново.
При отсутствии или несовпадении действуют правила остановки AGENTS.

Последний проверенный игровой результат Windows:

| Проверка | Фактический результат |
| --- | --- |
| workspace tests, включая doctests | 3820 passed, 0 failed, 18 ignored |
| workspace Clippy, all-targets/all-features, `-j 1`, `-D warnings` | exit 0 |
| workspace check | exit 0 |
| fmt check | exit 0 |
| Python scripts unittest | 4 tests OK |
| audit | 3105 objects, 49 BE, 255 source markers |

Это сведения о базе, не число тестов, к которому надо подогнать результат,
и не процент готовности Minecraft. Особенности последнего прогона и границы
проверки описаны в ревью N03/N19.

Уже сделано и не должно регрессировать:

1. Рамки и glow-рамки: размещение, взаимодействие, опора, дроп, metadata/NBT,
   исправления ENTITY_DATA, имён, знаковых полей и border collision.
2. Мешки: оба click overrides в правильном порядке, выбор по компоненту,
   Fraction с checked-ошибками vanilla, выброс из рук обычным player drop,
   содержимое при уничтожении item entity, runtime-компоненты вложений.
3. Книги: edit/sign/open, соответствующие пакеты, resolution и копирование.
   Обе руки; разрешение текста без удержания inventory lock. Writable item
   с written-content компонентом тоже открывает написанную книгу.
4. Entity pick: существующие поддержанные типы, видимые/доступные сущности,
   reach через attribute и bbox, сохранение вытесненного creative stack.

Открыто и не считается готовым:

- N01: crafted count / ITEM_CRAFTED / item callbacks и часть menu lifecycle.
- N03: KnowledgeBook / server recipe book; канонический JSON component codec
  для точной границы длины страниц. Serde JSON не эквивалентен vanilla codec.
- N19: отсутствующие paintings/armor stands, Avatar includeData, multipart
  bodies, enabled-feature foundation. Новая очередь закрывает только реально
  реализованные зависимости; это не повод создать фиктивные сущности.
- Настоящий клиент, два настоящих игрока, Linux, полный restart не проверялись.
  Unit/server fixtures не выдавать за эти проверки. **EULA не принималась.**
  Этот запрос не разрешает принять её, поменять eula.txt или обходить проверку.

## 4. Как работать долго и не заменить реализацию отчётами

На каждый этап запиши коротко: конкретный разрыв → источники → зависимости
→ реализация → проверка. После успешного законченного изменения продолжай.
Не нужно составлять ещё один большой план или писать отчёт каждые десять строк.

Отсутствующий локальный метод при понятном контракте и готовых данных — работа,
а не автоматический блокер. Например, настоящий menu button handler и его
consumer нужно реализовать, а не просто отметить отсутствие callback.

Отсутствующие данные, неясная архитектура или обязательная общая система —
настоящая зависимость. Сначала проверь Vecta, vanilla, extracted data и тесты.
При оставшейся неопределённости останови зависимую задачу по AGENTS и задай
узкий вопрос: что проверено, чего нет, почему догадка изменит vanilla, какой
путь/вывод extractor/архитектурное решение нужен. Молчание владельца — не ответ.

Можно продолжить независимый маршрут, пока ожидается ответ. Нельзя обходить
общую сломанную сборку, неверную версию или неизвестный shared contract сменой
ID. Не заполняй общие основы заглушками, локальными имитациями или TODO ради
галочки. Если архитектура ясна из кода и эталона, реализуй законченную основу
с настоящим потребителем в том же проверяемом этапе.

После повторных одинаковых ошибок инструмента/сборки останови перебор вариантов,
сохрани точный лог и причину. Не проведи ночь в цикле чтения одного файла,
перезапуска одного зависшего процесса или одной и той же неуспешной команды.

Части существующего кода могли уже совпадать с vanilla. Тогда статус —
«проверено, уже работало», с конкретным сценарием/источником; не переписывай
рабочую систему ради нового коммита. Проверка не должна становиться предлогом
объявить всю неизвестную область завершённой.

## 5. Навигация по исходникам и данным

Это точки входа, проверенные при подготовке пакета; они не заменяют чтение
конкретных методов перед реализацией. Относительные Java-пути далее считаются
от `minecraft-src/minecraft/src/net/minecraft/`.

| Область | Vecta | Java |
| --- | --- | --- |
| menu buttons | `player/player_inventory/player_handlers.rs` | `server/network/ServerGamePacketListenerImpl.java` |
| меню | `inventory/menu/{kind,behavior,builder}.rs`, `menu/kinds/` | `world/inventory/` |
| результаты/рецепты | `inventory/slots/`, `inventory/recipe_manager.rs` | `world/inventory/ResultSlot.java`, `world/item/crafting/` |
| typed recipes | `vecta-registry/src/recipe/`, `vecta-registry/build/recipes/` | `world/item/crafting/` |
| containers | `inventory/container/`, `inventory/lock.rs` | `world/Container.java`, `world/WorldlyContainer.java` |
| block entities | `block_entity/entities/`, `block_entity/registry.rs` | `world/level/block/entity/` |
| block behaviors | `behavior/blocks/` | `world/level/block/` |
| item behaviors | `behavior/items/` | `world/item/` |
| сущности | `entity/`, `entity/entities/` | `world/entity/` |
| projectiles | `entity/projectile/`, `entity/entities/objects/projectiles/` | `world/entity/projectile/` |
| damage/equipment | `entity/damage/`, `entity/living_base/`, `inventory/equipment.rs` | `world/entity/`, `world/damagesource/` |
| ecology/world ticks | `world/`, `behavior/blocks/`, `fluid/` | `world/level/` |

Vecta-пути без crate prefix в таблице относятся к `vecta-core/src/`.

Подтверждённые конкретные точки следующего шага:

- `handle_container_button_click` пока только пишет log и TODO.
- `SContainerButtonClick` и dispatch уже существуют; не создавать дубликат.
- Menu callback lifecycle уже имеет `take_open_menu_for_callback` и
  `finish_open_menu_callback`. Сверь их реальные контракты и все return/error paths.
- Существуют typed `StonecuttingRecipe`, `SmithingRecipe`, их input/ref types
  и build generators. Сначала используй их, не вводи второй рецепт-реестр.
- `ResultSlot::on_take` содержит TODO crafted count/stat/item callbacks.
- Copper chest behavior содержит явные TODO block entity/menu/viewers.
- Painting variants и banner patterns уже имеют registry types/build scripts.
  Наличие типа ещё не доказывает полноту данных и tags для конкретной операции.
- `ItemEntity` имеет TODO базовой invulnerability. Для исправления прочитай
  общую цепочку damage; не вставляй одно локальное условие вместо foundation.

Никогда не редактировать generated Rust вручную. Правки идут в build scripts,
аннотированные реализации, source assets или SteelExtractor. Extracted JSON
не писать по памяти. Локальный путь SteelExtractor нужно обнаружить и проверить;
он не установлен этим пакетом и не должен выдумываться. Если данные отсутствуют
и их нельзя получить, сообщи точное имя необходимого output и команду/путь.

## 6. Очередь и зависимости

Рекомендуемый первый маршрут:
**01 → 02 → 03 → 05 → 06 → 09 → 10 → 11 → 12 → 14 → 17**.
04, 07, 08, 16, 24, 25, 28, 31, 32 и часть survival-маршрута доступны отдельно
при подтверждении своих оснований. Порядок — приоритет, не запрет закончить
более близкую независимую задачу.

Крупные ветки:

- Рабочие станции: 01 → 02/03; 05; 06/09 поддерживают все result menus.
- Склад: 10 → 11 → 12/13; 10 + transfer contracts → 14/15/17/18.
- Раздатчик: 19 → конкретные 20–23; 20 зависит от соответствующих снарядов.
- Декор: 28/29/30 → дополнение pick-result, без отката уже готового N19.
- Оружие: 37 → 38 → 39 → 40; 41/42 по готовности общих оснований.
- Взрывы: 43 → 44/50; декорация кристалла не заменяет взрыв.
- Мобы: 46 → конкретные 47–53; 48 требует дальнего боя; 54 — реальный spawn.
- Поздние foundations: 64/65/66/67–70 только с проверенными данными и архитектурой.
- Сквозная проверка: 71/72 сопровождают готовые системы, не заменяют их код.

Обозначения ниже: **опора** — что должно реально работать до реализации;
**приёмка** — какие сценарии доказывают результат. Перечни проверок применяй
по фактическим веткам vanilla, не добавляй механически одинаковые тесты.

## A. Рабочие станции и корректный инвентарь

### W03-01. Кнопки контейнеров и рабочий камнерез

Связь: V04.02, V04.07; продолжение N13. Опора: существующие menus и typed recipes.
Эталон: `world/inventory/StonecutterMenu.java`, `world/level/block/StonecutterBlock.java`,
`ServerGamePacketListenerImpl.handleContainerButtonClick`.

- Реализуй полный путь пакет → проверка menu/container → callback → выбранный
  рецепт → result → выдача и расход input. Одного общего пустого hook недостаточно.
- Сверь список/порядок рецептов и его синхронизацию с клиентом 26.3, data slots,
  reset выбора после смены входа, reach/stillValid и server-side authority.
- Не удерживай player inventory lock вокруг callback, который повторно читает
  игрока. Меню возвращается по всем веткам существующего callback lifecycle.
- Take/quick-move должны расходовать input и обновлять preview в порядке Java,
  с соответствующими звуками, статистикой и callbacks, когда они предусмотрены.

Приёмка: выбрать два разных рецепта, изменить вход после выбора, забрать
несколько результатов, shift-click при заполненном инвентаре, неверный button
и устаревший container ID без изменения другого меню; выход/удаление блока.

### W03-02. Ткацкий станок и шаблоны знамён

Связь: V04.07, V05.11; продолжение N14. Опора: 01, typed patterns/tags/components.
Эталон: `world/inventory/LoomMenu.java`, `world/level/block/LoomBlock.java`.

- Добавь настоящий block-use/menu путь с input banner, dye, pattern item и output.
- Сверь доступные patterns, item-provided pattern sets, лимит слоёв и выбор
  через button packet. Содержимое и ограничения брать из существующих данных.
- Output сохраняет нужные исходные components и добавляет правильный слой;
  recipe preview не расходует материалы. Consumption шаблона сверить с Java.
- При смене dye/banner/pattern item сбрасывается или пересчитывается выбор
  ровно по эталону; quick-move и закрытие не создают копий.

Приёмка: обычный узор, предметный шаблон, уже заполненный слоями banner,
повторный клик после изменения входа, поведение при полном inventory, save/load
полученного знамени. Неверные button IDs не обходят pattern restrictions.

### W03-03. Кафедра: книга, страницы, получение назад, редстоун

Связь: V06.07, V04.02; продолжение N15. Опора: 01, существующие books/components.
Эталон: `LecternBlock.java`, `block/entity/LecternBlockEntity.java`, `LecternMenu.java`.

- Реализуй размещение допустимой книги на кафедре, world-backed хранение и
  открытие настоящего menu, синхронизацию текущей страницы и действия кнопок.
- Сверь permissions на извлечение, разные состояния страницы, comparator
  output и кратковременные сигналы/события при перелистывании.
- Written-content resolution использует исправленный путь без рекурсивной
  inventory блокировки; не дублируй старую реализацию из N03.
- Повторные действия после снятия книги/смены блока не возвращают старую книгу;
  NBT сохраняет content и корректно восстанавливает page/defaults.

Приёмка: книга с одной/несколькими страницами, крайние кнопки, извлечение,
два server-side viewers, разрушение и load. Реальный клиент помечать отдельно.

### W03-04. Резная книжная полка: довести найденные расхождения

Связь: V06.07. Опора: существующие `chiseled_bookshelf` block/entity реализации.
Эталон: `ChiseledBookShelfBlock.java`, `ChiseledBookShelfBlockEntity.java`.

- Не создавать полку заново: сверить выбор слота по hit position/facing,
  допустимые item tags, вставку, извлечение и обмен при полном inventory.
- Проверить occupied properties, last-interacted slot/comparator, events и
  сохранение всех слотов, включая loading в уже использованный объект.
- Проверить автоматический доступ контейнеров, если Java его предоставляет;
  не придумывать sided policy по внешнему виду блока.
- Исправить только воспроизведённые расхождения и потребителей общей логики.

Приёмка: все области попадания на разных facing, обе руки, разные book types,
извлечение в полный inventory, сохранение/восстановление и точный comparator.

### W03-05. Кузнечный стол: улучшения и отделка

Связь: V04.07, V06.06. Опора: typed smithing recipes и trim/material registries.
Эталон: `SmithingMenu.java`, `SmithingTableBlock.java`, smithing recipe classes.

- Реализуй menu со template/base/addition и настоящим результатом для
  transform и trim; существующие типы recipes не подменять строковыми lookup.
- Сверь slot predicates и component patch исходного предмета, damage,
  enchantments, trim replacement, расход входов и vanilla recipe choice.
- Применяй output только по актуальным трём входам; клиентский preview не
  является доказательством разрешённого сервером результата.
- Доведи quick-move, отсутствие места для выдачи и возврат входов при закрытии.

Приёмка: upgrade, два разных trims, неподходящий template/base, смена входа
перед take, предмет с custom components, полный inventory, save/load результата.

### W03-06. Crafting result: количество, статистика, callbacks и remainders

Связь: V04.04, V11.03; открытая часть N01. Опора: текущие ResultSlot/ResultHandler.
Эталон: `ResultSlot.java`, соответствующие crafting menus и `ItemStack` callbacks.

- Разбери remove/quickCraft/onTake и учёт реально созданного количества,
  а не числа кликов. Не добавляй счётчик без вызовов реальных consumers.
- Доведи ITEM_CRAFTED, item crafting callbacks и recipe award там, где их
  основания уже есть; отсутствующий recipe book не имитировать локальным set.
- Проверь remainders, например контейнеры ингредиентов, их возврат в grid/
  inventory/мир, точный порядок при частично заполненном результате.
- Результат нельзя повторно забрать из stale recipe после изменения input.

Приёмка: pickup/quick-move/throw применимые к result slot, output count > 1,
повторный craft, remainder с полным inventory. Незавершённые hooks явно
оставляют группу частичной; не скрывать их за готовыми stats.

### W03-07. Наковальня и точило: закончить подтверждённые пробелы

Связь: V04.06, V06.06. Опора: существующие anvil/grindstone menus и enchantment data.
Эталон: `AnvilMenu.java`, `GrindstoneMenu.java`, их result slots и block behavior.

- Сверь repair/combination/rename, стоимость, prior-work penalty и ограничения
  режима игры, component preservation и снятие/сохранение enchantments.
- Проверь creative, повреждение инструмента, enchanted books, curse handling,
  получение XP точила и отсутствие результата при невозможной операции.
- Rename packet проходит через актуальное menu, не меняет другой контейнер
  и не допускает обойти cost сменой input между preview и take.
- Сверь quick-move и расход XP/items, звуки и повреждение самой наковальни.

Приёмка: граничные стоимости по source, перенос совместимых/несовместимых
enchantments, blank rename, stale take, полная сумка. Уже совпадающее не переписывать.

### W03-08. Печи: результат, топливо, XP и автоматизация

Связь: V04.06, V06.13, V11.03. Опора: существующие furnace/blast/smoker реализации.
Эталон: `AbstractFurnaceBlockEntity.java`, furnace menus/result slots/fuel contracts.

- Сверь recipe matching/cache invalidation, cook progress/reset, fuel burn,
  remainder топлива, переходы lit и data slots на настоящем block entity.
- Проверь извлечение результата игроком и автоматикой, накопление recipe XP,
  условие его выдачи и сохранение соответствующих счётчиков.
- Сверь sided insertion/extraction и смену recipe/input в процессе, world
  tick/unload правила. Не добавляй offline catch-up без vanilla основания.
- Исправления общего container доступа должны пройти старые меню и 14/15.

Приёмка: разные рецепты/виды печей, остановка топлива, забитый output,
remainder топлива, save/load на середине плавки, игрок против hopper extraction.

### W03-09. Жизненный цикл всех меню и отсутствие потерь

Связь: V04.01–03, V13.04; открытая часть N01. Опора: существующий menu lifecycle.
Эталон: `AbstractContainerMenu.java`, `ServerPlayer.java`, listener container handlers.

- Проверь close, replacement другим menu, death, disconnect, dimension change,
  исчезновение/удаление блока и invalid reach для реально имеющихся menu kinds.
- Carried stack и временные inputs возвращаются/падают ровно в соответствующей
  ветке vanilla; world-backed contents не возвращать игроку при каждом close.
- Проверь stale state_id/full sync и устаревший container packet после открытия
  нового menu. Не удерживай один lock при повторном входе в player/menu API.
- Сверь viewers/openers: повторный close не уменьшает счётчик второй раз.

Приёмка: предмет на курсоре + полный inventory при close/death/disconnect,
уничтожение открытого блока и два fixture players. Не утверждать клиентский
тест или реальный disconnect, если вызван только server-side метод.

## B. Склад, сундуки и перемещение предметов

### W03-10. Обычный сундук как настоящий контейнер

Связь: V06.01, V04.08; N06. Опора: контейнеры, block entities, menus, loot/locks.
Эталон: `ChestBlock.java`, `ChestBlockEntity.java`, `BaseContainerBlockEntity.java`,
`RandomizableContainerBlockEntity.java`, `ContainerOpenersCounter.java`.

- Реализуй inventory-backed block entity, factory, load/save, открытие chest menu.
  NBT-preserving unimplemented fallback не считается контейнером.
- Сверь facing/waterlogging, block obstruction и sitting-cat проверку по наличию
  соответствующей entity capability; отсутствие кошек не заменять fake query.
- Custom name/lock/loot table+seed, viewers, sounds/block events, comparator
  и break drops должны пользоваться общими реальными основаниями.
- Проектируй основу с учётом 11/12/13, но не создавай лишний registry/framework.

Приёмка: поставить/открыть/положить/закрыть/load/сломать с сохранностью items,
lock и loot generation один раз. Неготовые обязательные foundations — точный блокер.

### W03-11. Двойной сундук и перестройка соединения

Связь: V06.01; N06. Опора: 10 и vanilla combining semantics.
Эталон: `DoubleBlockCombiner.java`, `ChestBlock.java`, `ChestMenu.java`.

- Сверь соединение по facing/type, secondary-use placement и запрет неверных
  сочетаний. Сохраняй исходные физические inventories обеих block entities.
- Combined view должен иметь правильный порядок слотов и реальные общие locks,
  без копии содержимого, расходящейся между viewers или половинками.
- Удаление половины, block replacement и разъединение обновляют menu validity,
  comparator, drops и соседний state в исходном порядке.
- Проверь взаимодействие с loot/name/lock каждой половинки и obstruction.

Приёмка: single→double→single, разные ориентации, заполненные половины,
два viewers и удаление одной стороны; save/load обеих половин без перестановки слотов.

### W03-12. Сундук-ловушка и счётчик открытий

Связь: V06.01, V07.02; N07. Опора: 10/11 и реальный redstone update path.
Эталон: `TrappedChestBlock.java`, `TrappedChestBlockEntity.java`, openers counter.

- Реализуй normal/direct signal, clamp и стороны по исходникам; comparator
  fullness остаётся отдельным контрактом от числа viewers.
- Открытие/закрытие, повторный open, spectator и периодический recount должны
  использовать vanilla правила, без локального счётчика на сетевых пакетах.
- Neighbor notifications и снятие питания при удалении/выходе/недействительном
  menu проходят общий мир, а не только меняют поле strength.

Приёмка: один/несколько viewers, двойной вариант, close/disconnect/removal,
соседний redstone consumer. Наличие field strength без наблюдаемого питания не готово.

### W03-13. Медные сундуки и преобразования вариантов

Связь: V06.01; продолжение существующего copper behavior. Опора: 10/11.
Эталон: `CopperChestBlock.java`, `WeatheringCopperChestBlock.java` и conversion code.

- Подключи реальные inventory/menu/viewer contracts к copper variants вместо
  текущих TODO; используй extracted sounds/properties, не ручной массив вариантов.
- Сверь соединение половин с разной oxidation/wax, нормализацию состояния,
  scraping/waxing, weathering ticks и условия, зависящие от viewers.
- Преобразование не должно заменять block entity пустым контейнером или
  рассинхронизировать открытую combined view/содержимое половин.
- Не включай AI copper golem в обещание готовности сундука: это отдельная сущность.

Приёмка: заполненный сундук до/после wax/scrape/oxidation, double conversion,
открытое menu при conversion и save/load с правильным вариантом и contents.

### W03-14. Воронка: block entity, тик и перенос

Связь: V06.02; N08. Опора: контейнеры, sided API, item entities, world ticking.
Эталон: `HopperBlock.java`, `HopperBlockEntity.java`, `HopperMenu.java`.

- Реализуй inventory/menu, facing, enabled/redstone, сохранение и tick path.
- Сверь последовательность push/pull, выбор слотов, cooldown, взаимодействие
  двух hopper и правила времени последнего тика; не подменяй одним timer на всё.
- Item merge учитывает components и stack limits обоих контейнеров. Всасывание
  item entity использует настоящую геометрию/порядок запросов из Java.
- Полный/запрещающий приём target не должен терять или дублировать source stack.

Приёмка: chest→hopper→chest, hopper chain, item сверху/сбоку в граничной области,
powered stop/resume, полный target, save/load с ненулевым cooldown.

### W03-15. Sided containers и совместимость автоматизации

Связь: V06.02, V04.03. Опора: 14 + настоящие furnace/chest/brewing consumers.
Эталон: `WorldlyContainer.java`, container holder/lookups и методы HopperBlockEntity.

- Сверь world lookup container/block entity/entity inventory и порядок выбора;
  не возвращай первый случайный контейнер из hash iteration.
- Операции insertion/extraction учитывают направление, доступные слоты,
  canPlace/canTake, компонентный максимум, actual capacity и dirty notification.
- Доведи подтверждённые дефекты совместно с реальными consumers; не создавай
  универсальный abstraction без используемого игрового маршрута.
- Lock ordering должен исключать взаимную блокировку и двойное lock того же
  контейнера при self/combined/container-entity случаях.

Приёмка: hopper к каждой стороне печи, двойному сундуку и существующим другим
контейнерам; compatible/incompatible components; заблокированная выдача сохраняет items.

### W03-16. Loot, lock, name и сохранение контейнеров

Связь: V02.04, V06.13, V11.01. Опора: реальные существующие container block entities.
Эталон: `BaseContainerBlockEntity.java`, `RandomizableContainerBlockEntity.java`.

- Сверь отложенное loot заполнение: trigger/player context, seed, повторное
  открытие, spectators и доступ автоматикой. Не генерируй loot при каждом read.
- Сверь имена/lock components и разрешения через существующие интерфейсы.
- При load defaults очищают ранее загруженные поля, если новый NBT их не содержит;
  не оставляй stale items/lock/loot из предыдущего состояния объекта.
- Round-trip сохраняет полный component patch, а не только item ID/count.

Приёмка: контейнер с loot до/после первого доступа, повторный load разного NBT,
заполненный named/locked контейнер, break/drop без повторной генерации loot.

### W03-17. Выбрасыватель и redstone pulse

Связь: V06.03; N09. Опора: 14/15 transfer contracts или эквивалентная готовая основа.
Эталон: `DropperBlock.java`, `DropperBlockEntity.java`, dispenser/container bases.

- Реализуй inventory/menu, facing, trigger edge, scheduled activation и выбор
  непустого слота с vanilla алгоритмом/RNG, без выбора из первого слота всегда.
- Ветка target container переносит предмет по sided rules; другая выдаёт item
  entity с корректными компонентами, velocity, sound/event и расходом.
- Полный target, пустой inventory, sustained power и повторные импульсы
  должны следовать разным веткам исходников, а не одному silent return.

Приёмка: импульс/постоянное питание, шесть facing, container/no-container/full,
заполненный component-rich stack и save/load между trigger и scheduled tick.

### W03-18. Автокрафтер и disabled slots

Связь: V06.04, V04.02; N11. Опора: recipe assembly/remainders, 01, transfer/output.
Эталон: `CrafterBlock.java`, `CrafterBlockEntity.java`, `CrafterMenu.java`,
соответствующий slot-state packet handler в listener.

- Реализуй world-backed grid, выбор разрешённых слотов, menu/slot-state путь,
  restrictions на toggle непустого слота и актуальный container ID по Java.
- Pulse берёт текущий recipe, потребляет inputs/remainders и выдаёт результат
  в нужном направлении с правильным fallback и animation state/ticks.
- Hopper insertion учитывает disabled slots и vanilla распределение предметов;
  comparator и triggered/crafting state не вычисляй по догадке.
- Используй общий typed recipe interpreter, не второй набор специальных рецептов.

Приёмка: успешный/невозможный craft, disabled/occupied slots, remainder,
повторные импульсы, полный output, stale slot-state packet, save/load grid/state.

## C. Раздатчики и дополнительные блок-сущности

### W03-19. Основа раздатчика с настоящим default consumer

Связь: V06.03; N10. Опора: inventory/ticks/RNG и pulse path из 17.
Эталон: `DispenserBlock.java`, `DispenserBlockEntity.java`,
`core/dispenser/DispenseItemBehavior.java` и default dispense implementation.

- Добавь реальный block entity/menu и default выброс предмета, включая empty
  activation, facing, выбор слота, events/sounds и сохранение inventory.
- Изучи vanilla hardcoded registration в исходниках 26.3; повтори необходимую
  структуру без выдуманного data registry или привязки ко всем items по имени.
- Extension path для поведения должен быть совместим с keyed capabilities
  проекта. Не вводи Any/TypeId и не заменяй Vecta registry архитектуру.
- Успех/отказ специальных behaviors должны управлять расходом и эффектами
  по исходному контракту, а не bool, которому потом приписывается иной смысл.

Приёмка: default stack с components, пустая активация, все facing и повторные
импульсы; далее подключать только полностью реализованные группы 20–23.

### W03-20. Раздатчик: реальные projectile consumers

Связь: V06.03, V08.12. Опора: 19 + готовые конкретные projectile implementations.
Эталон: projectile dispense behavior и регистрации 26.3; соответствующие item classes.

- Подключай доступные snowball/egg/potion/experience bottle и затем arrows/firework,
  когда готова их общая физика. Не регистрировать неработающие placeholders.
- Сверь spawn position/direction, speed/inaccuracy, owner/source/damage,
  перенос payload/components, event/sound и расход одного предмета.
- Не переиспользуй player throw, если его hand/stat/prediction побочные эффекты
  не соответствуют dispenser. Общая projectile основа должна быть общей корректно.

Приёмка: снаряд реально тикает/попадает и несёт правильный payload; направление,
стена вплотную, component potion, воспроизводимые RNG checks там, где нужны.
Отдельно перечисли ещё не подключённые типы, не объявляй весь dispenser готовым.

### W03-21. Раздатчик: пустые и заполненные вёдра

Связь: V06.03, V05.06. Опора: 19, готовые liquid/container placement contracts.
Эталон: bucket dispense registrations, `BucketItem.java`, `SolidBucketItem.java`.

- Сверь placement жидкости/твёрдого bucket content, pickup source, waterlogging,
  запрещённые измерения и блокируемое пространство по actual source methods.
- Empty/filled replacement, insertion remainder и fallback drop не должны
  уничтожать соседние contents при отсутствии места в inventory раздатчика.
- Для mob bucket нужен 33 и конкретная готовая сущность; без неё не создавать
  декоративную entity без поведения, сохраняющую только имя.

Приёмка: source pickup против flowing, приём waterlogged target, запрещённое
размещение, полный dispenser и correct remainder; liquid updates реально запускаются.

### W03-22. Раздатчик: экипировка, головы и тыквы

Связь: V06.03, V08.01. Опора: 19, живые equipment slots/eligibility/capabilities.
Эталон: соответствующие dispense registrations и equip/summon методы vanilla.

- Сверь target search, eligible slot, смену/запрет экипировки, component-driven
  restrictions и количество расходуемого stack. Не ограничиваться player class.
- Wearable/head/armor поведения объединять только при реальном общем контракте.
- Summoning через pumpkin/skull допускается только при готовой полной сущности
  и pattern check; отсутствие golem/wither не обходить успешным расходом предмета.

Приёмка: подходящий/неподходящий target, уже занятый слот, две цели в области,
custom equippable component; success/fallback exactly по Java. Unsupported summon отдельно.

### W03-23. Раздатчик: огонь, удобрение, бутылки и оставшиеся готовые действия

Связь: V06.03, V06.13. Опора: 19 и настоящие item/block hooks для каждой группы.
Эталон: регистрации dispenser в 26.3, flint-and-steel/bonemeal/bottle consumers.

- Для каждого подключаемого действия сверить use target, успех/отказ, durability,
  расход/remainder, событие и fallback обычного выброса.
- Bonemeal вызывает существующий рост с нужным RNG; bottle получает реальное
  содержимое из мира; ignition использует общие fire/ignite правила.
- Не добавляй consumer, который только проигрывает эффект и уменьшает stack.
- Boats, rails/minecarts и TNT подключать после готовности их полных сущностей.

Приёмка: по одному success и failure каждого реально добавленного поведения,
полный remainder inventory, повторная активация, interaction с реальным блоком.

### W03-24. Зельеварка: рецепты, топливо, время, выдача

Связь: V06.05, V04.07; N12. Опора: данные potion mixing и fuel components.
Эталон: `BrewingStandBlock.java`, `BrewingStandBlockEntity.java`, `BrewingStandMenu.java`;
найти актуальную potion brewing реализацию 26.3, не брать список из старой версии.

- Сначала проверь extracted mixing data. Если их нет, правь extractor и генерируй
  output; не переписывай ingredient→potion таблицу вручную из wiki/памяти.
- Реализуй fuel, brew timer, slot rules/data slots, cancel/restart при смене
  ingredient и actual potion output с component preservation по эталону.
- Bottle blockstate, sided hopper rules, remainder ingredient, events и
  сохранение активной варки должны идти через реальные общие интерфейсы.

Приёмка: обычное/усиленное/продлённое/throwable преобразования где есть recipes,
неподходящий input, смена ingredient во время варки, отсутствие топлива,
automation и save/load на середине. Одного menu без рецептов недостаточно.

### W03-25. Украшенный горшок: хранилище, тряска, разрушение

Связь: V06.07. Опора: typed POT_DECORATIONS/components, block entity, loot.
Эталон: `DecoratedPotBlock.java`, `DecoratedPotBlockEntity.java` и item interaction.

- Реализуй world-backed содержимое, вставку рукой, отказы/остаток и capacity
  по vanilla, включая creative/survival и отличия empty/occupied stack.
- Сверь wobble event, comparator, break с подходящим инструментом/projectile,
  дроп decorations/whole pot и перенос components в правильной ветке.
- Используй existing recipe result для decorated pot; не второй список sherds.
- Hopper/container capabilities добавлять только согласно реальному vanilla API.

Приёмка: вставка до заполнения, incompatible stack, comparator, обычное и
разбивающее разрушение, сохранение decorations и содержимого, без дюпа stack.

### W03-26. Знамёна: блок-сущность, item components, преобразования

Связь: V05.11, V06.07. Опора: banner patterns/colors registry; 02 для loom route.
Эталон: `AbstractBannerBlock.java`, `BannerBlock.java`, `WallBannerBlock.java`,
`block/entity/BannerBlockEntity.java` и соответствующие item methods.

- Проверь/добавь standing/wall placement, support loss, rotation/mirror,
  base color, patterns, custom name, metadata/block entity update и NBT.
- Item→block→pick/drop→item сохраняет component patch в vanilla объёме.
- Сверь взаимодействия очистки/копирования/щита с уже существующими recipes;
  не объявлять их рабочими только из-за наличия recipe enum.

Приёмка: получить узор в loom → поставить на стену/пол → сломать/подобрать,
save/load и named multi-layer banner; удаление опоры даёт правильный drop.

### W03-27. Shelf 26.3: слоты, взаимодействие и соединения

Связь: V06.07. Опора: реальные equipment/inventory actions и extracted block properties.
Эталон: `ShelfBlock.java`, `block/entity/ShelfBlockEntity.java` именно локальной 26.3.

- Сначала прочитай полностью server-side interaction и linking semantics этой
  версии; не подменяй полку декоративным display или копией chiseled bookshelf.
- Реализуй хранение/обмен items, selection по hit/facing, компоненты и лимиты,
  связанные соседние состояния и redstone поведение, если предусмотрено Java.
- Все изменения инвентаря должны использовать реальные slots/locks/sync.
- Поддержи placement, updates, break drops и NBT с reset отсутствующих полей.

Приёмка: одиночная/соседние полки, разные facing и попадания, пустые/занятые
руки, полный inventory, удаление части структуры и round-trip contents.

## D. Декор, предметы в мире и транспорт

### W03-28. Картины с настоящими размерами и вариантами

Связь: V05.09, V08.13; N04. Опора: painting variants/tags, hanging entities, collision.
Эталон: `world/entity/decoration/painting/Painting.java`, `PaintingVariant.java`,
`HangingEntityItem.java`, hanging/attached bases.

- Реализуй item placement, допустимые грани и выбор подходящих вариантов
  с vanilla алгоритмом выбора и RNG. Проверить полноту extracted variants/tags.
- Учитывай заданный item variant/ENTITY_DATA, опору по всей площади, bbox,
  border и сосуществование hanging entities; порядок загрузки данных важен.
- Реализуй metadata, tick support checks, damage/drop, custom name, NBT
  и get_pick_result через уже имеющийся N19 API.
- Общие hanging-изменения должны сохранить исправления рамок и glow-варианта.

Приёмка: маленькая/большая картина, тесная ниша, заданный variant, потеря одного
блока опоры, другая hanging entity, creative/survival, save/load и pick result.

### W03-29. Стойка для брони: placement, equipment, flags, damage

Связь: V05.09, V08.13; N05. Опора: living/equipment/damage foundations.
Эталон: `ArmorStandItem.java`, `world/entity/decoration/ArmorStand.java`.

- Реализуй placement/orientation/space и безопасное применение ENTITY_DATA
  через существующий spawn pipeline, без обхода прав на operator-only fields.
- Взаимодействие использует hit position, обе руки, real equipment slots,
  DisabledSlots и stack counts; не отдельный фальшивый inventory стойки.
- Small/Marker/Invisible/NoGravity/ShowArms/NoBasePlate, poses, dimensions и
  metadata должны соответствовать source. Сохраняй все относящиеся fields.
- Сверь отдельные damage branches: последовательные удары, projectile/fire/
  explosion/creative/invulnerable, equipment drops и pick result.

Приёмка: поставить→одеть→снять→повредить→load, disabled slot, small/marker,
component-rich equipment, количество drops. Не готов damage — не изображать его локально.

### W03-30. Cushion и взаимодействие пассажиров

Связь: V05.09, V08.13; N20. Опора: riding/dismount, collision, entity registration.
Эталон: `CushionItem.java`, `world/entity/decoration/Cushion.java` локальной 26.3.

- Прочитай серверный lifecycle и варианты cushion именно этой версии; размеры,
  placement и passenger правила не брать из внешнего описания или старого кода.
- Добавь полноценную сущность/item behavior, взаимодействие, разрешённое число
  пассажиров, offsets, снятие/разрушение и dismount через общую систему езды.
- Metadata/components/NBT и pick/drop обязаны переносить нужный вариант.
- Потеря основания/перемещение/повреждение проверяются в применимых vanilla ветках.

Приёмка: поставить/сесть/встать/сломать, попытка второго взаимодействия,
занятое пространство, passenger removal, save/load. Простая неподвижная модель не готова.

### W03-31. ItemEntity: базовый damage, gamerules и уничтожение

Связь: V08.12, V05.05. Опора: общая non-living invulnerability/damage цепочка.
Эталон: `world/entity/item/ItemEntity.java`, `Entity.java`, damage source tags.

- Закрой подтверждённый TODO `isInvulnerableToBase` через правильный общий
  контракт и реальные callers. Сверь tags, fire/explosion/immunity и game rules.
- Проверь ENTITY_DAMAGE/markHurt, удаление entity и вызов ItemBehavior.on_destroyed
  по правильной причине, особенно bundle содержимое.
- Natural despawn, pickup и уничтожение — разные пути. Не вызывай destruct hook
  при любом discard и не дублируй содержимое мешка при повторном hurt.
- Обнови затронутые реальные consumers общей Entity damage capability.

Приёмка: повреждаемый/защищённый item, повторный lethal hit, bundle destruction
и естественный despawn, mobGriefing-related branches по source. Без новых magic exceptions.

### W03-32. ItemEntity: слияние, владельцы, подбор и сохранение

Связь: V08.12, V02.03. Опора: текущие item entity/inventory/persistence реализации.
Эталон: `ItemEntity.java`, player pickup и ItemStack merge contracts.

- Сверь delay/age, owner/thrower, merge eligibility, компонентное равенство,
  ограничения stack и порядок переноса age/health/ownership при слиянии.
- Подбор частичного stack не удаляет остаток; packets/stats/criteria используют
  фактически взятое количество. Не проводить ItemStack сравнение только по ID.
- Сохранение и повторный load восстанавливают delays/UUID/components, правильно
  обрабатывают отсутствующие поля и не оставляют stale owner.
- Если Vecta уже совпадает, закрепи только неочевидные риски регрессии.

Приёмка: одинаковые/различные components, свободно меньше одного stack,
защищённый owner, merge с разными timers, load→pickup и два претендента.

### W03-33. Вёдра с мобами и перенос состояния

Связь: V05.06. Опора: bucketable capability и конкретные настоящие mob implementations.
Эталон: `MobBucketItem.java`, bucketable entity interface и конкретные mob classes.

- Проверь существующие сущности, которые реально поддерживают capture/release.
  Не добавляй все registry entries через пустую entity factory.
- Захват переносит нужные flags/health/name/entity data, выдаёт remainder и
  синхронизирует inventory; выпуск использует vanilla placement/permission rules.
- Сверь fromBucket/persistence и допустимость despawn после release, звуки,
  stats/criteria и сохранение bucket item components.
- Связь с dispenser подключать только после 21 и законченного consumer.

Приёмка: поймать→сохранить bucket→выпустить→повторно поймать с нужным state,
неудачное placement, обе руки и creative. Отсутствующий mob — явная зависимость.

### W03-34. Длительное использование и щит

Связь: V05.01, V03.06. Опора: текущие Item.use/living item-in-use/block-attacks.
Эталон: `LivingEntity.java`, `ItemStack.java`, `ShieldItem.java`, relevant data components.

- Сверь начало/продолжение/окончание/прерывание использования, use duration,
  swap руки/слота, смерть, discard и packet release.
- Щит использует существующий компонентный контракт BLOCKS_ATTACKS, углы,
  delay, durability и disable/cooldown по 26.3, без item-ID-only ветки.
- Correct hand и synced flags должны наблюдаться другими игроками; не хранить
  две расходящиеся копии use state в item behavior и Player.
- Общее исправление прогнать на уже готовых food/potions/bundle/spyglass/instrument.

Приёмка: main/offhand, смена stack в процессе, hit до/после activation,
disabled shield, ломание и завершение еды. Projectile use в 39–41 не должен регрессировать.

### W03-35. Лодки, плоты и варианты с сундуком

Связь: V05.10, V08.11, V08.14. Опора: movement/riding/water collision и containers.
Эталон: актуальные boat/raft item/entity classes в `world/entity/vehicle/boat/`;
если путь в 26.3 иной, найди через `rg --files`, не придумывай структуру.

- Начни с одной законченной boat family и общего настоящего движения:
  buoyancy, controls, passengers, dismount, collision и entity tracking.
- Затем варианты/rafts и chest inventory через реальные данные и shared contracts.
- Item placement проверяет water/space/raycast/permissions; падение/повреждение
  и drop учитывают материалы и содержимое с правильной причиной удаления.
- Vehicle packets принимаются от допустимого controlling passenger; sync и
  сохранение passengers не подменяются teleport на каждый входной пакет.

Приёмка: поставить→сесть→управлять→высадиться→load, obstruction и два пассажира,
chest variant contents. Если movement foundation неясна, stop этой ветки.

### W03-36. Вагонетки и существующая chest minecart

Связь: V05.10, V08.11. Опора: настоящая rail physics/riding и существующие rails.
Эталон: minecart family 26.3, rail blocks, vehicle packets, minecart items.

- Сверь нынешнюю chest minecart: наличие inventory/NBT ещё не значит движение.
  Реализуй общий rail/off-rail movement с curves/slopes/powered/detector semantics.
- Подключи item placement, passenger/dismount, collision и содержимое chest cart,
  drop/load/UUID без отдельного симулятора только для одного класса.
- Hopper/furnace/TNT variants только при готовности соответствующих systems;
  command minecart не регистрировать без command execution foundation.
- Обновление rail detector/comparator использует реальный moving entity query.

Приёмка: цикл рельс с поворотом/уклоном/питанием, detector, пассажир, chest load,
сход/разрушение. Неподключённые варианты перечислить, группу не закрывать целиком.

## E. Снаряды, оружие и взрывы

### W03-37. AbstractArrow с настоящим полётом и попаданием

Связь: V05.02, V08.12; основа N16. Опора: projectile collision/owner/damage/metadata.
Эталон: `world/entity/projectile/arrow/AbstractArrow.java` и общие Projectile classes.

- Реализуй полёт, in-ground state, block/entity hit, owner/left-owner,
  piercing eligibility, knockback/damage и removed-target правила по source.
- Pickup mode, item payload, critical/flags, despawn, burning и save/load
  обязаны работать через общую Entity/ItemStack архитектуру.
- Не делай стрелу snowball, который просто снимает фиксированное здоровье.
- Enchantment callbacks подключать через существующие hooks; отсутствующий
  обязательный trigger не заменять зашитым бонусом damage.

Приёмка: блок/сущность/вода, owner, pickup разрешён/запрещён, piercing cases,
стрела в блоке после load, компоненты pickup item и реальная outgoing metadata.

### W03-38. Обычные, эффектные и спектральные стрелы

Связь: V05.02, V08.12. Опора: 37, potion/effect components.
Эталон: `arrow/Arrow.java`, `SpectralArrow.java`, arrow item classes и potion logic.

- Реализуй конкретные Arrow/SpectralArrow entity types и item spawn behavior,
  payload/component transfer и правильный get_pickup_item.
- Сверь длительности/применение potion effects и glowing, reset цвета/содержимого,
  water/in-ground effects и NBT. Значения брать из source/data, не из памяти.
- Выбор типа projectile по ammo item/components должен быть общим с луком,
  арбалетом и dispenser, где vanilla действительно использует один контракт.

Приёмка: обычная/эффектная/спектральная стрела, hit+pickup, save/load до и после
попадания, custom components и применение эффекта к настоящей living entity.

### W03-39. Лук: боеприпас, натяжение, выпуск

Связь: V05.02; N16. Опора: 34, 37/38, projectile weapon/item helper contracts.
Эталон: `BowItem.java`, `ProjectileWeaponItem.java`, relevant enchantment helpers.

- Реализуй поиск ammo, predicate/hand order, начало использования, release
  с нужной силой и minimum draw, расход и damage weapon в source порядке.
- Creative/Infinity и enchanted projectile modifiers проверяются через реальные
  enchantment/component systems, а не по названию зачарования в локальном if.
- Correct hand, stats/sound и projectile pickup mode должны соответствовать
  конкретной ветке. Swap item во время draw не выпускает чужой stack.

Приёмка: слабое/полное натяжение, нет стрел, ammo в разных слотах, creative,
weapon break, release после смены предмета; projectile реально наносит source damage.

### W03-40. Арбалет: заряд, хранение, multishot и выпуск

Связь: V05.02; N17. Опора: 34, 37–39, charged projectiles component/fireworks.
Эталон: `CrossbowItem.java`, projectile weapon helpers, charged component codec.

- Сверь charging progress, sound stages, selected ammo, loading consumption и
  хранение точных projectile templates/components в заряженном предмете.
- Release заряженного предмета, multishot directions, quick charge, piercing
  и firework route используют настоящие projectile implementations.
- NBT/network hash/codec заряженного item должен пережить save/load и обмен
  слота/руки, не терять unrelated components и не расходовать ammo повторно.

Приёмка: прерванный/завершённый заряд, load заряженного оружия, повторный shot,
multishot и rocket по готовности; полный inventory и creative. Части явно не скрывать.

### W03-41. Трезубец: бросок, возврат и riptide

Связь: V05.03, V08.12; N17. Опора: 34, projectile owner/damage, weather/movement.
Эталон: `TridentItem.java`, `arrow/ThrownTrident.java`, enchantment triggers.

- Реализуй charged throw, hand removal/payload, damage/durability, pickup и
  сохранение projectile state. Это предмет игрока, не отдельный созданный clone.
- Loyalty возврат связан с живым/доступным owner и правилами отсутствия owner;
  full inventory/death/disconnect не должны молча терять или дублировать item.
- Channeling/riptide с условиями воды/погоды/цели и movement проходят реальные
  общие hooks; если lightning foundation отсутствует, точный зависимый блокер.

Приёмка: бросок→попадание→подбор/возврат, owner ушёл, полный inventory, load
в полёте, riptide разрешён/запрещён. Нельзя назвать всё готовым только за обычный throw.

### W03-42. Wind charge и воздействие на блоки/сущности

Связь: V05.04, V08.12. Опора: projectile и общие explosion/interaction contracts.
Эталон: `WindChargeItem.java`,
`projectile/hurtingprojectile/windcharge/{AbstractWindCharge,WindCharge}.java`.

- Реализуй use/cooldown, spawn/direction/owner и полёт/hit через projectile base.
- Hit должен вызывать реальный vanilla wind interaction: impulse, fall-related
  state, разрешённые block triggers и effects, а не только particle packet.
- Связь с gamerules, block interaction category и owner permissions сверить
  по source. Не копировать damage explosion для ветрового действия без проверки.

Приёмка: hit ground/air/entity, self impulse, взаимодействующий блок,
cooldown/creative, owner исключения и корректная синхронизация velocity.

### W03-43. Общая система взрывов с настоящими consumers

Связь: V07.04, V08.12. Опора: world collision, damage, loot, RNG и packet delivery.
Эталон: `world/level/ServerExplosion.java`, связанные explosion interfaces/classes.

- Найди фактическую текущую основу. Сверь block resistance/ray traversal,
  entity exposure/damage/knockback, fire, interaction mode и loot decay/context.
- Порядок RNG и iteration должен совпадать там, где он определяет результат;
  не вводи «взорвать все блоки в радиусе» или только client explosion packet.
- Изменения распространяются через правильный server packet/event путь и
  world updates; world storage и клиент получают один фактический результат.
- Проверяемый этап включает настоящий существующий consumer, а не только API.

Приёмка: разные препятствия/сопротивления, закрытая от взрыва living target,
gamerules/drop modes, deterministic fixture по source там, где применимо,
реальные blocks+entities+packets. Неясная архитектура — остановить эту ветку.

### W03-44. TNT, end crystal и взрывающиеся предметы

Связь: V07.05, V05.09; продолжение N20. Опора: 43, entity ticking, damage source.
Эталон: TNT block/primed entity classes, `EndCrystalItem.java`, `EndCrystal.java`.

- TNT: ignition, fuse, motion, сохранение, chain reaction и owner/attribution.
- Crystal: item base/space validation, ENTITY_DATA где применимо, fire, damage
  и настоящий explosion. Существующий worldgen/NBT класс не переписывать вслепую.
- Dragon fight callbacks допустимы только с реальным fight manager; если его
  нет, отдельно задокументировать отсутствие End progression и зависимость.
- Bed/anchor explosions подключать к общей системе после проверки 59/60.

Приёмка: primed load/fuse, chain TNT, защищённый/разрушаемый crystal, недопустимая
база и creative; сохранность attribution. Dragon respawn не считать реализованным.

### W03-45. Рыбалка и фейерверки: проверить существующие реальные пути

Связь: V05.12, V08.12. Опора: имеющиеся fishing hook/firework classes и loot/effects.
Эталон: `FishingRodItem.java`, `FishingHook.java`, `FireworkRocketItem.java`,
`FireworkRocketEntity.java` — точные расположения найти в локальном дереве.

- Fishing: owner/hand lifecycle, cast/retrieve, hook target, water/open-water
  eligibility, timers/loot/XP/durability и исчезновение invalid owner.
- Firework: item payload, flight/lifetime, explosion effects/damage,
  attached/elytra и crossbow/dispenser routes при готовых consumers.
- Не объявлять success по наличию entity factory; проверить фактический tick,
  collision, результат retrieval/explosion и сохранение важного state.

Приёмка: бросить/вернуть удочку, поменять руку/предмет, target и loot case;
rocket с/без explosion components, load/tick, attached case при готовом gliding.

## F. Настоящие мобы и их жизненный цикл

### W03-46. Общая основа мобов через конкретного потребителя

Связь: V08.01–02, V02.03. Опора: текущие Mob/Monster/AI/pathfinding/equipment/loot.
Эталон: `Mob.java`, `LivingEntity.java`, monster bases и конкретный выбранный mob.

- Для ближайшего реального моба из 47–53 выясни отсутствующие общие contracts:
  spawn finalization, goal lifecycle, targets, attributes, despawn/persistence.
- Реализуй только нужное законченное основание с этим consumer; не проектировать
  гигантский Brain/AI framework заранее и не ставить пустой default goal.
- Equipment/loot/death attribution/XP и player tracking должны работать через
  общий source-compatible путь, а не вручную заданный drop в каждом классе.
- Сверь save/load UUID/equipment/target-related persistence по vanilla fields.

Приёмка: моб появляется штатно, выбирает цель/действует, получает урон/умирает,
выдаёт нужный loot и загружается. Наличие только модели/attributes не результат.

### W03-47. Зомби и подготовленные варианты

Связь: V08.04. Опора: 46, melee/pathfinding/sunlight/equipment и data variants.
Эталон: `world/entity/monster/zombie/Zombie.java` и доступные variant subclasses.

- Добавь обычного zombie с реальными goals, navigation, attack, daylight,
  baby/equipment/spawn attributes и persistence по данным/source.
- Варианты husk/drowned/zombie villager добавлять только при готовности их
  отличий: water/conversion/weapon/villager systems, без пустых subclass shells.
- Сверь loot/XP, damage calls и взаимодействия с doors/difficulty/gamerules,
  а не просто прямой walk_to(player) каждый тик.

Приёмка: погоня вокруг препятствия, attack cadence, солнце/вода применимо,
baby/equipped save/load и death drops. Для каждого варианта отдельный статус.

### W03-48. Скелет и дальний бой

Связь: V08.04. Опора: 46, 37–39 и ranged goal/navigation.
Эталон: `monster/skeleton/AbstractSkeleton.java`, `Skeleton.java` и ranged goals.

- Реализуй выбор оружия/melee-ranged goals, дистанцию/видимость/charging,
  aim и spawn arrow через общий bow/projectile contract.
- Доведи daylight/equipment, drops/pickup/attributes, combat goal после смены
  оружия и сохранения. Не спавни стрелу без связанного реального оружия/боеприпаса.
- Stray/bogged/wither skeleton подключать после их effects/biome/nether данных
  и actual behavior, не одним скином и типом entity.

Приёмка: цель за стеной/в зоне видимости, переход distance/weapon, реальные
projectile hits, sunlight protection, save/load equipment и death attribution.

### W03-49. Паук и пещерный паук

Связь: V08.04. Опора: 46, climbing/navigation/light/target contracts и effects.
Эталон: `monster/spider/Spider.java`, `CaveSpider.java` и их goals.

- Реализуй wall climbing и metadata, размеры, path choices, target rules
  и melee с правильными hooks/attributes, а не teleport вверх по стене.
- Сверь светозависимое поведение, spawn effects/variants и riders, если есть
  готовые riding foundations; неподдержанный jockey не имитировать.
- Cave spider требует реального poison application и source difficulty rules.
- Damage/loot/saving идут через living/mob foundations, без дублированного health.

Приёмка: вертикальная преграда, узкий проход для размеров, смена освещённости,
attack и poison cases, load и совместимость entity tracking.

### W03-50. Крипер и управляемое раздувание

Связь: V08.04, V07.05. Опора: 46, 43, targeting/navigation и synced data.
Эталон: `world/entity/monster/Creeper.java`, swell goal и explosion consumers.

- Реализуй start/stop swelling, fuse/radius/ignited/powered state, server tick
  и metadata. Удаление цели/потеря reach меняют state по source.
- Explosion учитывает mobGriefing, owner/damage attribution и charged variant;
  остаточные effects/loot special cases не заменять одним discard.
- Player ignition/durability и lightning требуют общих реальных interactions;
  отсутствие lightning не означает готовность charged gameplay целиком.

Приёмка: приблизиться/отойти, принудительный ignite, powered NBT/load,
разный gamerule, стены/targets и настоящий world explosion, без client-only анимации.

### W03-51. Слизень и размерозависимое поведение

Связь: V08.04. Опора: 46, dimensions/collision/jump controls, loot/spawn contracts.
Эталон: `monster/cubemob/Slime.java` и связанные move/goals/spawn методы.

- Size управляет dimensions/attributes/damage/XP, metadata и split lifecycle.
  Не зашивай один размер в factory, игнорируя NBT/commands.
- Реализуй движение/прыжки/target conditions и split children с vanilla RNG,
  count/size/position и причиной удаления родителя.
- Magma cube — отдельная готовая вариация только с её fire/fluid/jump behavior,
  не переименование slime в новой registry entry.

Приёмка: разные размеры, hit/knockback, смерть со split, повторное damage
удалённого parent без duplicate children, load и spawn restrictions по source.

### W03-52. Существующие животные: полный полезный survival цикл

Связь: V08.03, V09.03. Опора: cow/mooshroom/pig/sheep/chicken implementations.
Эталон: соответствующие local vanilla classes и common AgeableMob/Animal logic.

- Не переписывай животных заново. Сверь temptation, breed food, love/age,
  child spawn, growth, persistence и фактические navigation/interaction branches.
- Проверь milk/bowl/shear/dye/egg/saddle/steering по применимым типам и source,
  component/stack remainder, permissions, creative и drops.
- Breeding/food callbacks должны работать с настоящими item/inventory changes,
  без двойного consumption и детёнышей от одного повторённого packet.
- При общих age/goals исправлениях пройти всех существующих consumers.

Приёмка: приручения здесь не выдумывать; размножить пару, вырастить детёныша,
получить продукт, sheep regrowth/color inheritance где готово, save/load ages.

### W03-53. Волк и кошка через общие taming/owner contracts

Связь: V08.05. Опора: 46, tameable/owner/sitting/navigation и variants/data.
Эталон: актуальные Wolf/Cat/TamableAnimal classes и их цели в локальной 26.3.

- Сначала выясни общий owner/taming контракт и registry variants. Если его
  архитектура не определена, спроси; не делать два несовместимых owner UUID API.
- Реализуй одну законченную сущность: tame/feed/sit/follow, owner validation,
  combat/avoid goals, components/variants, damage/death и save/load.
- Затем второй consumer и совместное использование capability, включая sitting
  cat query для сундука, если vanilla требует именно эту проверку.
- Teleport/follow между измерениями сверять по source, не переносить из памяти.

Приёмка: tame→sit→follow→load, owner отсутствует/меняется мир, feeding,
дружественная/враждебная цель; неподдержанные cosmetic variants перечислить отдельно.

### W03-54. Естественный спавн готовых мобов

Связь: V09.01, V08.02. Опора: настоящие mobs, biome spawn data, chunk/tick/lighting.
Эталон: `world/level/NaturalSpawner.java`, SpawnPlacements, SpawnState и server tick.

- Разбери текущую реализацию и данные: categories/caps, player distance,
  eligible chunks, pack size/placement, light/collision/liquid restrictions.
- Включай только готовые сущности, явно отмечая влияние отсутствующих типов;
  нельзя считать весь biome spawn полным с несколькими mob factories.
- Сверь попытки/порядок RNG, despawn/persistence, difficulty/gamerules и
  spawn finalization без глобального таймера «создать моба у игрока».
- Не меняй extractor weights и worldgen data вручную ради видимого населения.

Приёмка: контролируемые light/biome/distance cases, caps с двумя игроками,
despawn исключения, category gates и отсутствие spawn внутри solid blocks.

## G. Выживание и физика существующего мира

### W03-55. Добыча блоков, инструмент и drops

Связь: V03.04, V06.15. Опора: существующий player break pipeline, block/item data/loot.
Эталон: `ServerPlayerGameMode.java`, player destroy-speed/tool methods, block loot code.

- Сверь start/abort/stop destroy packets, progress/cracks, distance, target
  replacement и authoritative block state; клиент не выбирает результат добычи.
- Tool correctness, hardness, efficiency/effects, underwater/airborne modifiers,
  durability, creative/adventure predicates и permissions берутся из source/data.
- Loot context получает правильные tool/components/block entity/player и RNG;
  silk/fortune не подменять ручной выдачей items для нескольких блоков.
- Повторный finish для уже изменённого блока не даёт второй drop или stat.

Приёмка: правильный/неправильный tool, смена блока/руки в процессе, abort/retry,
be contents, fragile tool, creative/adventure, два игрока на одном target.

### W03-56. Ближний бой и component-driven weapon behavior

Связь: V03.05, V05.01. Опора: текущие damage/attributes/equipment/enchantment hooks.
Эталон: Player attack и LivingEntity damage methods, damage source tags,
актуальные ATTACK_RANGE/weapon components 26.3.

- Сверь reach/cooldown, critical/sweep/knockback, sprint conditions, shield
  interaction, armour/toughness/effects, invulnerability windows и damage ordering.
- Затронутые mace/kinetic weapon callbacks исполняются через правильный общий
  компонентный путь, не по жёсткому списку нескольких vanilla items.
- Equipment damage/stats/attribution и player velocity packets не должны
  повторяться после отклонённой атаки или удара по removed entity.
- Не заменяй Vecta architecture полной Java иерархией, если capability уже подходит.

Приёмка: обычный/critical/sweep где source допускает, shield, разные armor/effects,
out-of-range/hidden target, повторный packet, weapon break и два fixture players.

### W03-57. Голод, насыщение, еда и употребляемые компоненты

Связь: V03.07, V05.12. Опора: `player/food_data.rs`, item use и consume effects.
Эталон: `world/food/FoodData.java`, food/consumable components и player/living ticks.

- Сверь exhaustion accumulation, saturation/food bounds, regen/starvation,
  difficulty/gamerule branches и packet updates по реальным server ticks.
- Food/use завершение учитывает canAlwaysEat, duration, effects/remainder,
  hand change и creative; различать completed и interrupted consumption.
- Не делать async/sleep внутри tick и не использовать wall-clock вместо
  игровой последовательности тиков.
- Пройти уже существующие consume-effect implementations, включая probabilistic
  effects с source range/probability и нужным RNG контрактом.

Приёмка: насыщенный/голодный игрок, regen/starvation transitions, interrupted
food, remainder при полной сумке, potion/food effects и save/load food state.

### W03-58. Смерть, опыт, keepInventory и respawn

Связь: V03.08, V02.02, V13.04. Опора: living death, inventory/menu lifecycle и XP entities.
Эталон: ServerPlayer death/restoreFrom/respawn code, Player inventory drop rules.

- Сверь death transition, одинарность loot/XP spawn, gamerules, curse-related
  equipment effects и реальное player data после respawn.
- Menu/carried inputs проходят 09; не дропать и inventory, и его stale snapshot.
- Spawn point/anchor/bed validity, respawn dimensions, attributes/effects,
  selected slot, recipe/stat persistence и outgoing respawn state сверить по Java.
- Disconnect между death и respawn и повторный запрос не создают второго тела,
  drops/XP или некорректной живой копии игрока.

Приёмка: keepInventory on/off, XP, cursed equipment, открытое menu, invalid spawn,
повторный respawn и save/load. Реальный reconnect отмечать только после его выполнения.

### W03-59. Кровати, сон и пропуск ночи

Связь: V09.04, V03.03. Опора: текущие `player/sleep*`, `world/sleep*`, bed behavior.
Эталон: Player/ServerPlayer sleep methods, bed classes и ServerLevel sleep tick.

- Сверь time/dimension/monster/distance checks, occupied state, spawn point,
  messages, sleeping position/pose и wake conditions.
- Sleep quorum, gamerules, spectator/disconnected players и weather skipping
  должны основываться на current participants, не накопленном счётчике пакетов.
- Bed destruction/chunk/world changes корректно завершают сон и обновляют flags.
- Взрывающийся bed в запрещённом измерении требует 43; без общего взрыва не
  выдавать успешный normal sleep как fallback.

Приёмка: один/несколько fixture players с разным процентом сна, враг рядом,
разрушение кровати, wake/death/disconnect, respawn point и dimension-specific branch.

### W03-60. Порталы, измерения и подтверждение телепорта

Связь: V03.03, V10.06, V08.14. Опора: существующие `portal/`, `world/portals.rs`, lifecycle.
Эталон: portal/dimension transition classes 26.3, ServerPlayer teleport/ACK methods.

- Сверь Nether portal shape/search/construction, coordinate scaling, world border,
  cooldown и повторное попадание, включая entities/passengers по source.
- Player transfer корректно меняет world registration/tracking, chunks/menu,
  respawn/position packets и ожидаемый teleport ACK без дубликата игрока.
- End portal/gateway и anchor branches проверять отдельно; отсутствие dragon
  fight не означает завершённость End progression.
- Если worldgen/chunk readiness требует jobs, tick не ждёт async spin/sleep.

Приёмка: туда/обратно, border, invalid frame/destination obstruction, cooldown,
ACK старого teleport, passenger cases при готовности, сохранение нового измерения.

### W03-61. Жидкости и падение в воде/лаве

Связь: V07.06, V03.02. Опора: существующие `fluid/`, scheduled ticks и collision.
Эталон: `LiquidBlock.java`, FlowingFluid/WaterFluid/LavaFluid и living fluid travel.

- Сверь spread directions/source conversion, water/lava mixing, waterlogged
  states, update scheduling и правила загрузки границы чанков.
- Entity immersion/buoyancy/drag, drowning/fire и movement modifiers должны
  использовать реальный fluid state, не проверку имени блока под ногами.
- Bucket interactions и dispenser 21 не могут независимо рассчитывать другой
  результат mixing/placement или пропускать neighbor updates.
- Исправлять только подтверждённые расхождения; существующую flowing систему
  не переписывать целиком по одному визуальному симптому.

Приёмка: источник/поток/восстановление источника, mixing, waterlogging,
перенос через chunk boundary fixture, entity swim/drown и save/load scheduled state.

### W03-62. Огонь, дождь, молния и их server-side эффекты

Связь: V07.07, V09.04. Опора: текущие weather/fire/block updates/damage.
Эталон: fire/weather/lightning classes и ServerLevel tick в локальной 26.3.

- Сверь ignition/spread/aging/extinguish, flammability data, gamerules и
  порядок random/scheduled ticks; не ручная таблица горючих блоков.
- Weather state/packets, rain checks и fire/entity effects основаны на dimension/
  biome/position contracts, а не единственном global isRaining для всех случаев.
- Lightning требует настоящей entity/event/damage цепочки и relevant block/mob
  transformations; visual lightning отдельно не выдавать за gameplay.
- Channeling/creeper/copper потребители должны использовать один правильный путь.

Приёмка: burnable/nonburnable neighbors, дождь/крыша, doFireTick-related branches,
молния в target, metadata/packets и конкретные поддержанные transformations.

### W03-63. Редстоун и поршни в реальной контейнерной цепочке

Связь: V07.01–03. Опора: существующие wire/diode/piston/comparator/world redstone.
Эталон: соответствующие local block classes, neighbor update и scheduled tick code.

- Не начинать редстоун заново. Выбери воспроизводимые схемы с новыми chest/
  hopper/dropper/crafter consumers, найди реальные несовпадения ordering.
- Сверь edge triggering, quasi-connectivity где предусмотрена, comparator
  fullness/overrides, scheduled repeaters и update propagation.
- Piston resolver/moving block entity сохраняют rules push/pull/sticky/slime/
  honey и не теряют block entities вопреки vanilla movable policy.
- После фикса общего update path обязательно проверить старые consumers.

Приёмка: pulse→dropper/hopper lock, comparator заполнения, repeater chain,
piston constraints, соседние chunk fixtures и отсутствие оставшихся powered states.

## H. Оставшиеся общие основания и сквозная проверка

### W03-64. Канонический component JSON и точная граница страниц

Связь: V01.02, V05.08; известный открытый дефект N03. Опора: text component codec API.
Эталон: `ComponentSerialization.CODEC`, `WrittenBookContent.resolvePage/isPageTooLarge`
и codec ветка component tree локального Minecraft.

- Прочитай `planning/reviews/N03-N19.md` и текущую TextComponents dependency.
  У неё есть codec NBT; наличие serde JSON не означает наличие vanilla codec JSON.
- Нужен типизированный canonical encoder/length calculation, совпадающий с
  компактными строками и nested components Java. Вычитать фиксированный overhead
  из serde длины или угадывать по JSON object keys запрещено.
- Выясни правильного владельца codec API и доступный editable checkout зависимости.
  Не правь `.cargo/git/checkouts` и не подменяй dependency незапушенным cache patch.
  Если ownership/подключение неясны — точный архитектурный вопрос по AGENTS.
- Сохрани exception fallback: при ошибке resolution оригинал возвращается
  без последующей size check; не откатывать исправление ac8c2d788.

Приёмка: source-derived boundary для plain/styled/nested/Unicode/escaped text,
raw/filtered pages и malformed resolution, вместе с NBT self-inventory regression.

### W03-65. Recipe book и knowledge book как единый настоящий маршрут

Связь: V04.05, V05.08, V11.03. Опора: typed recipes, player persistence и protocol displays.
Эталон: `stats/ServerRecipeBook.java`, `KnowledgeBookItem.java`, listener recipe handlers.

- Сначала составь локальную карту существующих unlock/settings/display APIs.
  Известно, что полноценный player awardRecipes путь прежде не был найден.
- При ясной архитектуре реализуй server recipe book с настоящими consumers:
  unlock/remove/settings, initial sync, save/load, recipe lookup/validation.
- KnowledgeBook проверяет всю relevant chain и consume/error ordering по Java;
  нельзя «сначала валидировать всё», если эталон расходует item раньше проверки.
- Recipe placement/limited crafting/criteria подключать к реальной книге;
  не выдавать локальный набор identifiers за синхронизированную систему.

Приёмка: unlock→packet→load, повтор/unknown recipe, valid/invalid knowledge book,
creative/survival, actual use handler и crafted recipe consumer. Архитектура неясна — спросить.

### W03-66. Стол зачарований с seeded выбором

Связь: V04.07, V06.06, V11.02. Опора: 01, enchantment data/effects, player seed/XP.
Эталон: `EnchantmentMenu.java`, enchanting table block/entity и EnchantmentHelper.

- Реализуй bookshelf/power validation, input/lapis slots, offers/costs/clues,
  выбранный button и выполнение на актуальном server item.
- Используй vanilla seeded RNG с точным потреблением вызовов и выбором из
  relevant tags/data. Thread RNG ради «похожих» enchantments не подходит.
- Consumption lapis/XP, enchanted book conversion, player seed update и
  сохранение components должны соответствовать конкретному source branch.
- Если поддержаны не все enchantment effects, не объявлять всю enchanting
  progression готовой; отдельно записать реально выдаваемые и работающие эффекты.

Приёмка: фиксированный seed/source golden result, разные power layouts,
недостаточный lapis/XP, creative, input swap перед button, take/load результата.

### W03-67. Данные карт, идентификаторы и настоящая empty map

Связь: V05.07, V02.04; N18. Опора: world saved-data lifecycle и typed map components.
Эталон: `EmptyMapItem.java`, `MapItem.java`,
`world/level/saveddata/maps/MapItemSavedData.java` и storage/saved-data manager.

- Выясни существующий world saved-data API и allocation/persistence counters.
  Если общего основания нет и ownership не определён, не хранить карты в global
  HashMap, теряющемся после restart, и не выдавать один фиксированный map ID.
- Реализуй ID allocation, creation center/scale/dimension, colors/decorations,
  dirty state и save/load через правильную world architecture.
- Empty map use создаёт реальную запись и component-linked item с правильным
  расходом/remainder/stats; не только замену item ID на filled map.

Приёмка: две карты с разными ID, две ссылающиеся на одни данные copies,
создание/load на границе координат и отсутствие сброса счётчика после restore.

### W03-68. Обновление карт, рамки и стол картографа

Связь: V05.07, V04.07, V05.09. Опора: 67, book/recipe callbacks где применимо, frames.
Эталон: `MapItem.java`, `MapItemSavedData.java`, `CartographyTableMenu.java`,
`ClientboundMapItemDataPacket.java`, ItemFrame map methods.

- Реализуй реальные terrain/decorations updates, per-viewer dirty tracking,
  map packet и tracking/in-frame count/cleanup с vanilla limits/source order.
- Frame insert/remove/load/destruction обновляет общую запись карты; никакого
  отдельного fake frame counter, не связанного с persisted map.
- Cartography clone/scale/lock использует manager и реальные item callbacks,
  component state, current inputs и consumption, не только preview item.
- Exploration maps требуют готового structure search; не возвращай spawn/world
  origin вместо отсутствующего результата поиска.

Приёмка: map use/move→dirty packet, два viewers, frame insert/remove/load,
clone/scale/lock и bounds. Не выдавать client visual observation за unit packet decode.

### W03-69. Вибрации и sculk sensor

Связь: V06.09, V09.05. Опора: game event routing, listeners, scheduling, world storage.
Эталон: `SculkSensorBlock.java`, calibrated variant/entity classes, VibrationSystem.

- Сначала проверь настоящий game-event foundation: emission, source context,
  distance/occlusion, delivery delay, chunk lifecycle и сохранение listener state.
- Реализуй один полный consumer sensor: frequency/power, active/cooldown,
  waterlogged state и neighbor updates. Не listener, сразу питающий блок от любого события.
- Calibrated sensor использует реальные side input/frequency filters.
- Catalyst/shrieker/warden progression только после соответствующих death/XP/
  warning/spawn foundations, отдельный статус без фиктивного spawn.

Приёмка: допустимое/игнорируемое событие, wool occlusion по source, timing,
filtered frequency, save/load in-flight state и настоящий redstone consumer.

### W03-70. Обычный spawner и граница trial/vault систем

Связь: V06.10, V09.02. Опора: реальные mob factories, spawn data, weighted RNG и ticking.
Эталон: `SpawnerBlockEntity.java`, BaseSpawner/spawn data classes и mob rules.

- Доведи normal spawner: nearby player, delay, attempt/nearby limits, spawn
  validation, finalizeSpawn, NBT и vanilla reset algorithm.
- Mob entity NBT проходит общий safe load/spawn pipeline; неизвестные types
  не превращаются в «успешно заспавненный» default mob.
- Trial spawner/vault/ominous rewards — отдельные state machines и data contracts.
  Начинать их можно только с проверенными registries/loot/player tracking;
  normal spawner не отмечает их выполненными.

Приёмка: активный/неактивный player range, cap, невозможное пространство,
delay save/load, entity data и разрешённый concrete mob; точный список unsupported типов.

### W03-71. Loot, enchantment triggers и advancement по готовым consumers

Связь: V11.01–05. Опора: текущие loot/enchantment/stat/criterion foundations.
Эталон: local loot functions/conditions, enchantment effect components,
`advancements/triggers/` и ServerAdvancementManager/player advancements.

- Для новых 01–70 составь краткий список реально обязательных callbacks и
  проверь их фактическое выполнение: crafting, smelting, brewing, pickup,
  kill/projectile, tame/breed, enchantment/item-use — по готовым маршрутам.
- Исправь подтверждённые пропуски существующей foundation, включая правильный
  context/actor/tool и порядок RNG; не дописывай stats вместо advancement engine.
- Если общего criterion/event foundation нет, сначала согласуй неясную архитектуру
  и запиши точную зависимость, не разбрасывай TODO по всем новым классам.
- Loot output и component preservation сравнивать с source/data, не с похожим
  примером из другой версии; используемые flags/conditions должны работать.

Приёмка: законченный игровой consumer реально вызывает проверенный callback
и меняет нужный persistent progress/loot, без duplicate rewards после reload/retry.

### W03-72. Сквозной survival-маршрут и сохранность мира

Связь: V00.04–06, V02, V13, V14. Опора: фактически реализованные прошлые этапы.
Эталон: соответствующие source paths каждого включённого сценария, не внешний гайд.

- Собери выполняемый маршрут из готовых consumers: добыть→скрафтить→переплавить
  →сложить→передать hopper/dropper→использовать workstation→сохранить/загрузить.
- Добавь две player fixtures с общим миром/контейнером, конкурирующие действия,
  death/close и stale packets. Fixture означает server test, не реальных клиентов.
- Persistence тесты работают с отдельным temp world, не с пользовательским save.
  Проверяй actual inventories/entities/components/UUID, не только успешный read.
- Настоящий Mojang/Vecta client comparison и длительный server restart/run
  возможны только при наличии клиента/явного EULA согласия; если их нет,
  подготовь сценарии и честно оставь эту часть невыполненной.

Приёмка: воспроизводимые шаги, точный state после каждого перехода и load,
границы выполненного. Это не сертификат full vanilla, а проверка конкретного маршрута.

## 7. Общие критерии готового этапа

Этап можно отметить «реализовано и проверено», когда выполнен весь его заявленный
сценарий и обязательные ветки. Если закрыт только самостоятельный подмаршрут,
родитель остаётся «частично», а отчёт явно называет работающий объём и ограничения.

Для каждой системы проверь применимые случаи:

- Main/offhand; survival/creative/adventure/spectator; permission/operator
  differences только там, где есть в source. Не переносить одно правило на всё.
- Item counts, component-dependent stack limits, unrelated patch/name/entity data,
  полный inventory, carried stack, remainder и возврат предмета при отказе.
- Порядок input validation/consumption/failure именно vanilla. Общее правило
  «при ошибке всегда ничего не тратится» неверно для некоторых source paths.
- Current server state после mutation/close: stale recipe/button/packet не имеет
  права выдать прежний output или изменить другой menu/entity/block.
- Real outgoing packet/metadata/data slots, stat/event/sound route. Проверка
  вызванного метода не равна клиентскому наблюдению звука или анимации.
- Tick ordering, cooldown, seeded RNG там, где детерминизм наблюдаем и нужен
  по AGENTS; не сортировать unordered data произвольно ради воспроизводимого теста.
- Save/load и повторный load в тот же объект; numeric NBT coercion, defaults,
  знаковые поля, reset отсутствующих значений и сохранение full component payload.
- Общие контракты проверять на существующих consumers: menus, frames/glow,
  bundles, books, projectile owners, containers и registry build generators.

Не добавлять тесты на очевидный constant/type shape ради количества.
Хороший regression test должен поймать правдоподобную ошибку или объяснить
неочевидное правило. Для исправления бага, где возможно, сначала воспроизведи
его тестом, затем сохрани red/green результат в журнале.

Не добавляй production `.unwrap()`/`.expect()`, borrow-checker appeasement clones,
неодобренный unsafe и Any/TypeId в shared plugin-extensible foundations.
Исключение для keyed DowncastType с уникальным owner key и SAFETY — как в AGENTS.
Semantic snapshot для освобождения reentrant lock не путать с клоном ради borrow
checker; владение и lock scope должны иметь ясную причину.

Не добавляй catch-all, который скрывает потерю items/components или делает вид,
что операция успешна. Bundle arithmetic должна сохранить проверенный Java
Commons Fraction error path: packet ошибка не равна entity tick fatal path.

## 8. Сборки и ограничение расходов на проверки

Проверь доступность/место на C:/D:. На этой машине Cargo/temp уже переполняли C:
и parallel builds зависали. Используй **`-j 1`** и существующий build cache:

```powershell
$env:CARGO_TARGET_DIR = 'D:\CodexBuilds\VectaMC'
$env:CARGO_INCREMENTAL = '0'
$env:TMP = 'D:\CodexBuilds\tmp'
$env:TEMP = 'D:\CodexBuilds\tmp'
```

Сначала проверь существование/пригодность temp/build путей; не меняй системные
переменные навсегда. Каталог логов — `D:\CodexBuilds\logs\` или ignored
`.vecta-cache/`, с отдельным именем этапа и сохранённым exit code.

Не убивай все rustc/Java процессы по имени и не чисти чужие кэши/миры.
Старые зависшие rustc существовали до этого сеанса. При зависании различай
свой процесс, старый процесс, compiler diagnostic и отсутствие свободного места.

До первого изменения достаточно проверить HEAD/dirty tree/эталон и прочитать
известный baseline. Если игровая база изменилась или есть признаки регрессии,
сделай baseline повторно. Не запускай полный suite перед каждым новым файлом.

После каждого законченного изменения — узкий проверяющий фильтр:

```powershell
# Вместо имени ниже подставь реальный test module/сценарий текущего этапа.
cargo test --locked -p vecta-core --lib stonecutter -j 1

# При изменении packet/registry contracts — их реальные узкие проверки.
cargo test --locked -p vecta-protocol --lib container_button -j 1
cargo test --locked -p vecta-registry --lib stonecutting -j 1
```

Названия фильтров выше — примеры, не заявление о существующих сейчас тестах.
**0 tests — не успешная приёмка.** Выбирай реальные тесты и проверяй итог вывода.
После узких green checks stage→diff review→commit→push и следующий этап.

При изменении общего контракта после узких тестов проверь затронутые consumers.
Не начинай следующий блок поверх неразобранной регрессии. Полный workspace
прогон уместен на границе группы/перед итоговой передачей, а не после каждого
тривиального rename. Если после проверки меняется поведение, повтори relevant tests.

Обязательная итоговая проверка изменённого игрового кода:

```powershell
cargo fmt --all --check
cargo check --locked --workspace -j 1
cargo test --locked --workspace -j 1 --no-fail-fast
cargo clippy --locked --workspace --all-targets --all-features -j 1 -- -D warnings
python -m unittest discover -s scripts
python scripts/audit_vanilla.py
git diff --check
git diff --cached --check
```

Запускай последовательно. Для документационных-only изменений Rust suite
не нужен; проверь ссылки, queue IDs, diff и отсутствие ложных утверждений.
При доступных `typos`/`prek` следуй actual repository requirements; если инструменты
отсутствуют, так и напиши, не выдавай пропуск за pass и не устанавливай обходы молча.

Audit может менять свои tracked outputs — проверь их diff и stage только
соответствующие текущему коду. Generated Rust/extracted data правила AGENTS
имеют приоритет над желанием получить красивый статус audit.

## 9. Коммиты, журнал и передача следующему сеансу

Документы создавать по мере появления содержимого, не кучей пустых файлов:

- `planning/night-03/progress.md`: текущая ветка/база и завершённые этапы.
- `planning/night-03/blockers.md`: проверенные блокеры, зависимости и вопросы.
- `planning/night-03/scenarios.md`: выполненные/подготовленные сценарии и evidence.
- `planning/night-03/report.md`: итог при настоящем завершении сеанса.

Для progress достаточно: ID, конкретный результат, source methods, commit SHA,
actual check/results, граница проверки, следующий ID. Указывай фактическое состояние,
не ставь заранее «полностью реализовано» или fabricated command output.

Для blockers: ID → что проверено в Vecta/Java/data/tests → что отсутствует →
риск догадки → точный необходимый file/output/decision → зависимые задачи.
Не копируй один общий «нет инфраструктуры» на все 72 задачи без чтения кода.

Коммит содержит законченный проверяемый игровой кусок с понятным сообщением.
Если крупная задача делится, каждый commit сохраняет рабочие основания;
статус родителя остаётся частичным до закрытия обязательного объёма.
Не коммить сломанную общую основу как feature. Не терять незавершённое через reset.

Push делай после собственных проверенных коммитов в свою ветку. Проверь exit code;
перед финалом сверь local/remote SHA. Ошибка push не превращается в «запушено».
Ожидаемый remote: origin Daniiltoptl/VectaMC, не upstream Steel-Foundation.

CHANGELOG обновляй краткими фактами о working gameplay. README сохраняй аккуратным:
не переписывай оформление после каждого пункта, не обещай full vanilla, plugins,
совместимость Paper или версии, которых нет. Старые review reports не исправлять
задним числом; новые сведения добавлять в текущий журнал/ревью.

Перед исчерпанием ресурса не начинай огромную новую систему. Закончи текущий
проверяемый кусок, сохрани реальные uncommitted paths, если они остались,
и выполни итоговые проверки с достаточным запасом. Если лимит оборвал работу,
следующий сеанс начинает с actual git state и журнала, а не с полного нового аудита.

## 10. Обязательный итоговый отчёт для Codex

Итог должен позволять проверить изменения без догадок. Включи:

1. Ветку, стартовый/итоговый полные SHA, список коммитов, compare URL,
   фактический push status и оставшиеся dirty paths, включая чужие отдельно.
2. Статусы W03-01…W03-72: реализовано и проверено / проверено, уже работало /
   частично / блокер / не начато. Допустима компактная таблица и диапазоны для
   не начатых задач; не генерируй 72 пустых описания вместо полезного отчёта.
3. Для каждого результата: действие игрока до/после, relevant vanilla методы,
   source/data paths Vecta, общие контракты и затронутые старые consumers.
4. Реальные bug reproductions и regression checks; команды, exit codes,
   passed/failed/ignored, важные промежуточные ошибки без выдачи их за clean pass.
5. Codegen/extractor/dependency/persistence/protocol изменения отдельно,
   с source/output provenance и влиянием на уже сохранённые состояния.
6. Клиент, Mojang comparison, реальные 2+ players, restart/unload, Linux:
   выполнено или не выполнено. Не заменяй эти строки числом unit tests.
7. Открытые игровые ограничения и архитектурные вопросы. У каждого блокера
   конкретная проверенная причина и следующий независимый шаг.
8. Самый близкий следующий ID и минимальные команды/источники для продолжения.

После записи отчёта **ответь владельцу обычным текстом**: что заработало,
сколько задач реально закрыто/частично, итог проверок, commit/ветка и ссылка
на report. Не завершай только файлами и не пиши «всё готово» о полном Minecraft.

## 11. Начало работы без нового планирования

1. Прочитать правила и актуальные ревью, сверить base/version/dirty hash.
2. Перейти на безопасное продолжение `codex/night-vanilla-03` от actual tip.
3. Прочитать `StonecutterMenu.java`, relevant listener handler, typed recipe
   contracts и текущий Vecta callback lifecycle.
4. Реализовать **W03-01** через настоящий packet/menu/output путь и проверить его.
5. Коммит/push своей ветки; сразу **W03-02** или следующий независимый готовый этап.

Продолжай по очереди до реального конца сеанса или проверенного общего блокера.
Большой список не отменяет точность vanilla, а требование vanilla не повод
бесконечно откладывать реализацию, когда эталон, данные и архитектура уже ясны.
