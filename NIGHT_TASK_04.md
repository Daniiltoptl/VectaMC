# VectaMC — большой пакет продолжения 04

Дата подготовки: 30 октября 2026. **120 задач для длинных сеансов разработки.**
Цель — наблюдаемое поведение Minecraft Java 26.3: настоящий игровой маршрут,
синхронизация и сохранение. Это очередь на несколько продолжений, а не обещание
закончить Minecraft за одну ночь. Плагины, Paper API, Folia и другие версии отложены.

## 1. Откуда продолжать

- Рабочий каталог: `C:\Users\My comp\Desktop\Vecta`.
- Remote: `origin` → `https://github.com/Daniiltoptl/VectaMC.git`.
- Ветка: **`codex/night-vanilla-03`**, продолжать её фактический tip.
- Снимок при подготовке: `9e27a42f0e2b92f531715b4de5371d2176abc474`.
  Этот файл попадёт в более новый docs-коммит: начинать с tip, не откатываться к SHA.
- Workspace: `0.16.1+mc26.3`; авторитет версии — `Cargo.toml`, суффикс `+mc...`.
- Эталон: `minecraft-src/.vecta-reference.json`, 26.3, 5037 Java-файлов,
  подготовленных штатным `update-minecraft-src.sh`.
- Код live NBT read context: `1f114ffb8`; полный проверенный снимок: `015152051`.
  После него до указанного tip менялась только документация.
- Последний полный результат: **3890 passed / 0 failed / 18 ignored**, включая
  doctests, `cargo test --locked --workspace -j 1`, exit 0.
- Пять финальных `book_resolution::lectern_load` тестов прошли отдельно.
  Clippy workspace/all-targets/all-features с `-D warnings`, fmt, Python-проверки
  и audit прошли для того же Rust-кода. Cargo предупреждает о mimalloc при
  dhat-heap; подробности в ревью, предупреждение не подавлялось.
- Клиент 26.3, Linux и два настоящих сетевых клиента не проверялись.

Сначала прочитай `AGENTS.md`, `planning/reviews/lectern-load-context.md`,
`planning/reviews/night-03-follow-up.md` и текущие `planning/night-03/` журналы.
`NIGHT_TASK_03.md` сохраняется как подробный предшественник. В нём есть исторические
указания начинать с W03-01: **для этого продолжения они устарели**.

Чужой dirty-файл: `planning/night-01/report.md`. Не редактировать, не stage,
не reset/stash. Стартовый известный SHA256:
`06680CBC971245B70C990D95C906F0F1E3685EFFDE61C13D562664B4B6B05222`.
Если владелец изменил его после подготовки, сохранить фактическое новое состояние.
Не использовать старый checkout `.codex/worktrees/steel-fixes/Vecta` как актуальную базу.

## 2. Что уже сделано и не должно делаться заново

W03-01..05 имеют серверную реализацию: камнерез, ткацкий станок, кафедра,
резная книжная полка, кузнечный стол. Это не объявление полной готовности групп.
Сохраняй конкретные исправления и дополняй только оставшийся контракт:

- Кнопки меню уже имеют packet/dispatch; не создавать второй handler.
- `CUpdateRecipes` отправляется при initial join. Он содержит property sets и
  stonecutter single-input displays; **не требует полного `RecipeDisplay`**.
- Loom использует registry/tags banner patterns; отсутствие recipe book не
  блокирует его список шаблонов. Предыдущий такой блокер был ошибочным.
- Slot-display ID уже получены SteelExtractor, typed registry/codecs существуют.
  Не возвращать числовой enum, ручной JSON или второй реестр рецептов.
- Частично вмещающийся shift-result камнереза оставляет остаток через player drop.
- Размещение книги в кафедру использует мир/центр блока и поставившего игрока.
- Live NBT load использует отдельный `BlockEntityLoadContext`, **без игрока-источника**.
  Первичная disk/worldgen загрузка идёт до Vanilla `setLevel` и не разрешает книгу.
  Не добавлять повторный resolve при активации чанка или ссылку World → Server.
- Рамки, мешки, книги, entity pick и ряд предметов уже реализованы частями.
  Проверить текущее состояние; не заменить работающий код новой заглушкой.
- B-W03-05 и B-W03-07 как блокеры последних runtime-проверок закрыты.
  B-W03-04 про implicit block-entity components остаётся открытой работой.

**Проверенные текущие пробелы первого маршрута:** `ResultSlot::on_take` содержит
TODO crafted count/stat/item callbacks; `CraftingHandler` содержит TODO recipe
retention, limited crafting, award/trigger. Ниже эти части разделены по зависимостям.
Остальные карточки — задания проверить и закончить область, **не утверждение,
что каждый перечисленный сценарий сейчас сломан или отсутствует**.

## 3. Порядок длинного сеанса

Первый маршрут: **001 → 002 → 003 → 004 → 005 → 006**. Затем выбери ближайший
полностью обоснованный маршрут: **011–020 → 021–030** или **007–010**.
Пропуск по реальной зависимости фиксировать, затем брать независимую задачу.
W04-001..010 разворачивают старые W03-06/W03-65; не закрывать W03-06 только
по наличию счётчика, если callbacks/awards остались недоделанными.

| Волна | Карточки | Осязаемый результат |
| --- | --- | --- |
| A | 001–010 | Результаты крафта, рецепты и реальная книга рецептов |
| B | 011–020 | Полный путь компонентов блок-сущностей и корректные меню |
| C | 021–030 | Сундуки и работающая автоматизация предметов |
| D | 031–040 | Раздатчики, зельеварка и дополнительные контейнеры |
| E | 041–050 | Остальные станции, опыт, лодки и вагонетки |
| F | 051–060 | Декор, выпавшие предметы и использование предметов |
| G | 061–070 | Снаряды, оружие, взрывы и урон |
| H | 071–080 | Конкретные мобы, спавн и сохранение их состояния |
| I | 081–090 | Цельный survival и события прогресса |
| J | 091–100 | Книги, карты, зачарование, sculk и первый сквозной мир |
| K | 101–110 | Генерация, чанки, сохранение и работа нескольких миров |
| L | 111–120 | Поздние системы и сравнение с официальным сервером |

Номера — приоритеты, а не требование строить на отсутствующей основе.
Например, projectile dispenser ждёт нужный снаряд; crafter ждёт настоящий
system crafting callback; карта не должна появляться с фиктивными map data.
Если callback игры требует отсутствующей общей системы, не заменять его no-op:
оставить зависимую карточку частичной и согласовать неясную архитектуру по AGENTS.

Готовность конкретного контракта важнее статуса большой карточки. В 005 разделены
player crafting hook и map-specific system post-process: готовый первый контракт
позволяет продолжить соответствующие consumers 006; map-ветка остаётся на 092–095.
095 использует уже готовый hook и закрывает его отложенный consumer — ждать полного
005 до начала map foundations не нужно. В 007–010 и 090 так же отделять recipe
knowledge от зависимого advancement trigger, не объявляя весь award-путь законченным.

Весь этот файл — **задание на разработку**, не предложение снова написать план.
Не заканчивать после первого зелёного пункта, если есть ресурс и следующая
обоснованная задача. Не тратить сеанс на глобальный аудит всех TODO.

## 4. Общий контракт выполнения карточки

1. Найти конкретный игровой разрыв и relevant код Vecta, локальную Java и данные.
2. Если уже работает, отметить конкретный проверенный сценарий, не переписывать.
3. Исправить реальный consumer: packet → handler → состояние → выходная
   синхронизация → сохранение/загрузка, когда эти части нужны механике.
4. Добавить регрессию на неочевидное поведение. Не тестировать только ID,
   форму struct, константу, регистрацию или собственный helper без consumer.
5. Запустить узкие проверки; прочитать число исполненных тестов и exit code.
6. Проверить diff, сделать понятный commit, push в origin текущей ветки, продолжить.

Не вводить стабы, fake data, `todo!()`, постоянные пустые callbacks, runtime
JSON-registry loading, ручные extracted ID или самодельный рецепт-реестр.
Generated Rust не редактировать. Данные получать extractor/build-script путём.
Game tick не ждёт async jobs, I/O или sleep. Учитывать container lock order и
reentrant text queries; snapshots допустимы по смыслу, не ради обхода borrow checker.
Требования на keyed downcast, RNG, unsafe, unwrap/expect — строго по AGENTS.

Чётко различать: реализовано и проверено; уже работало и проверено; частично;
блокер; не начато. Unit harness с двумя Player не называть двумя настоящими клиентами.
Наличие исходников Java не означает, что запускалась Mojang server comparison.

## A. Результаты крафта и рецепты

### W04-001. Действительные пути извлечения результата

Связь: W03-06. Основание: существующие `ResultSlot`, `ResultHandler`, menu click.
Эталон: `ResultSlot.java`, `Slot.java`, `AbstractContainerMenu.java`.

- Сверить remove, safeTake, quickCraft, swapCraft, clone и onTake с call sites Vecta.
- Отдельно разобрать fake-slot: текущий `remove` забирает весь preview; не менять
  это механически на split без проверки вызывающих веток и ванильных ограничений.
- Зафиксировать, кто хранит количество и когда оно сбрасывается; сразу перейти
  к работающему исправлению 002, не расходовать ночь на отдельный дизайн-отчёт.
- Приёмка: обнаруженный случай счёта по кликам или повторного onTake воспроизводится
  через настоящее меню; к нему есть regression, а не тест очевидного поля.

### W04-002. Учёт реально созданного количества

Зависимость: 001. Эталон: `ResultSlot.java`, `ItemStack.java`.

- Провести removeCount через обычное взятие, quick craft и swap craft там,
  где эти пути действительно существуют; не прибавлять один craft дважды.
- ITEM_CRAFTED получает число созданных предметов, включая output count > 1.
- Сбросить накопление на правильной границе, сохранив порядок vanilla callbacks.
- Приёмка: один рецепт с несколькими выходами, повторное изготовление, смена
  рецепта между действиями; итог статистики соответствует взятому результату.

### W04-003. Pickup, Shift, throw, swap и creative result

Зависимость: 002. Эталон: `AbstractContainerMenu.java`, `CraftingMenu.java`.

- Сверить ограничения частичного взятия fake result, обмен hotbar и offhand.
- В creative safeClone отделить item callback от расхода ингредиентов и stats.
- Невмещающийся результат обрабатывать по конкретному menu, сохранив исправление камнереза.
- Приёмка: почти полный inventory, занятый курсор, throw результата, hotbar swap;
  ингредиенты/выходы/stat не дублируются и не исчезают. Только допустимые vanilla ветки.

### W04-004. Remainders и положение рецепта в сетке

Зависимость: 001. Эталон: `ResultSlot.java`, `CraftingInput.java`, `CraftingRecipe.java`.

- Сверить positioned input и соответствие локальных координат реальным grid slots.
- Возврат тары: grid → совместимое слияние → inventory → world drop в vanilla порядке.
- Проверить remainders при изменившемся/пропавшем рецепте; не подменять fallback догадкой.
- Приёмка: рецепт с тарой, сдвинутая shaped-сетка, несовместимый остаток в grid,
  полный inventory; все возвращаемые stacks и components сохраняются.

### W04-005. Настоящие item crafting callbacks

Зависимость: 001–003. Эталон: `Item.java`, `ItemStack.java`, `MapItem.java`.

- Не путать `ItemStack.onCraftedBy(player,count)` с `onCraftedBySystem(level)`:
  в 26.3 первый выдаёт stat и вызывает item onCraftedBy, второй — post-process.
- Найти реальные переопределения в эталоне, проверить ItemBehavior/capability путь.
- Ввести необходимый shared hook только с настоящим consumer и корректным контекстом.
- Карточку map post-processing не закрывать пустой функцией; зависимость — 092–095.
- Приёмка: один callback за нужное действие, правильный stack до/после; ручной
  и системный крафт не приписывают друг другу игрока или статистику.

### W04-006. Контракты результатов разных рабочих станций

Зависимость: 002–005. Эталон: конкретные `*Menu.java` и result slots в `world/inventory/`.

- Проверить consumers ResultSlot/ResultHandler: crafting, stonecutter, smithing,
  anvil, grindstone, furnace; состав не угадывать — найти реальные реализации.
- Не навязать всем слотам crafting semantics: XP, recipe awards и ingredient
  consumption различаются. Общий helper только для реально общего контракта.
- Проверить stale preview после изменения inputs перед take и внешней автоматизации.
- Приёмка: минимум один regression на каждого изменённого consumer; сохранены
  существующие smithing/loom/stonecutter tests. Неподдержанные awards пометить 008/010.

### W04-007. RecipeDisplay и данные для книги рецептов

Связь: W03-65. Основание: уже готовый SlotDisplay. Эталон: `RecipeDisplay.java`.

- Определить точный состав display variants/registries/codecs для 26.3.
- Получить нужный output из SteelExtractor; typed generated data, расширяемые refs.
- Существующий update-recipes packet не переписывать: книга рецептов имеет свои пакеты.
- Приёмка: точные байты из правил vanilla codec, inline/reference payload,
  malformed input и настоящий consumer, создающий display из typed recipes.

### W04-008. Серверная книга рецептов и её сохранение

Зависимость: 007. Эталон: `ServerRecipeBook.java`, `RecipeBook.java`, `RecipeBookSettings.java`.

- Хранить знания игрока, settings/highlights и identities по актуальному контракту.
- Провести initial sync, add/remove, seen/settings packets и persistence игрока.
- Не имитировать знания локальным HashSet только внутри result menu.
- Приёмка: unlock → disconnect/save → reload → initial sync; повторный unlock
  не дублирует данные; чужой domain/player не наследует знания другого.

### W04-009. Размещение рецепта из клиентской книги

Зависимость: 007–008. Эталон: `ServerPlaceRecipe.java`, `RecipeBookMenu.java`.

- Провести packet recipe placement до действительного открытого меню и grid.
- Сверить craft-all, недостаток ingredients, occupied grid и возврат прежних inputs.
- Учитывать component-aware matching и лимиты stacks; не доверять output клиента.
- Приёмка: запрос чужого menu/id, несовместимые components, частичный inventory,
  повторный запрос после смены inputs; нет потерь/dupe и ghost state.

### W04-010. Awards, limited crafting и knowledge book

Зависимость: 008; 090 для advancement consumer. Эталон: `RecipeCraftingHolder.java`,
`KnowledgeBookItem.java`, `CraftingMenu.java`.

- Подключить реальное запоминание использованного рецепта и awardUsedRecipes.
- Limited crafting проверяет настоящее состояние знаний; knowledge book
  использует typed recipe identities и vanilla условия расхода предмета.
- RECIPE_CRAFTED snapshot/trigger подключать к реальной advancement системе,
  без фиктивного consumer. При её отсутствии отдельно указать незакрытый пункт.
- Приёмка: неизвестный/известный рецепт, настройка limited crafting, save/load,
  повторный крафт и creative; родитель W03-06 остаётся частичным до всех зависимостей.

## B. Компоненты блок-сущностей и жизненный цикл меню

### W04-011. Общий контракт implicit block-entity components

Связь: B-W03-04. Эталон: `BlockEntity.java`, `DataComponentMap.java`, `DataComponentPatch.java`.

- Сверить apply/collect implicit components, base component map, removal из custom NBT.
- Найти подходящее место в существующем BlockEntity и typed component APIs.
- Реализовать hook с первым реальным consumer книжной полки; не универсальный
  сериализатор «всё NBT в item». Неясное владение/архитектуру согласовать по AGENTS.
- Приёмка: contents/components идут через реальный item↔block-entity путь;
  явные defaults/removals не теряются и не превращаются в второй storage.

### W04-012. Применение компонентов при установке блока

Зависимость: 011. Эталон: `BlockItem.java`, конкретные `*BlockEntity.java`.

- Разобрать custom block data, applyImplicitComponents и setPlacedBy в нужном порядке.
- Первый законченный consumer — chiseled bookshelf; затем готовые containers.
- Контекст книги кафедры не заменить source игрока для live NBT load.
- Приёмка: установить stack с содержимым/именем, сохранить/загрузить, получить
  тот же предмет обратно; запрещённые для placement данные обрабатываются как vanilla.

### W04-013. Creative pick-block с includeData

Зависимость: 011–012. Эталон: `ServerGamePacketListenerImpl.java`, `BlockEntity.java`.

- Довести `handle_pick_item_from_block`: права, includeData, выбранный hotbar slot,
  поиск существующего stack и передача components по реальному packet пути.
- Содержимое получать per-BE hooks, не ручным списком исключений в packet handler.
- Приёмка: полка с разными книгами и контейнер с component-rich item; pick → place
  сохраняет содержимое, без includeData получается обычный предмет по vanilla.

### W04-014. Base components в disk/owned/live NBT round-trip

Зависимость: 011. Эталон: `BlockEntity.java`, `LevelChunk.java`.

- Сверить saveWithFullMetadata, saveWithoutMetadata, loadWithComponents и factory paths.
- Не потерять base map при owned↔borrowed bridge и не читать server text до setLevel.
- Согласовать сбор custom/implicit fields, исключив дублирование одного компонента.
- Приёмка: borrowed load, owned load, save/reload и chunk unload/load дают одинаковое
  содержимое; старые lectern-load пять тестов остаются зелёными.

### W04-015. Живое редактирование NBT блок-сущности

Зависимость: 014. Эталон: `BlockDataAccessor.java`, `DataCommands.java`.

- Проверить уже существующие /data и /execute store consumers, их save/load границу.
- После восстановления component-aware full save обновить execute-store load:
  не очищать components из-за неполного промежуточного compound.
- Сохранить явный BlockEntityLoadContext, setChanged и нужные block updates.
- Приёмка: изменение Page/CustomName/доступного custom field сохраняет unrelated
  components; ошибочная path не делает частичный commit и не выдаёт ложный success.

### W04-016. Loot-backed контейнеры без преждевременной генерации

Связь: W03-16. Эталон: `RandomizableContainerBlockEntity.java`, `RandomizableContainer.java`.

- Найти момент unpackLootTable, player/luck context и сброс ссылки после генерации.
- Использовать существующий loot/RNG контракт, не заполнять контейнер фиктивными items.
- Выявить consumers открытия, automation и destruction, которые действительно trigger loot.
- Приёмка: loot открывается однократно, seed/save/load повторяемы где требует vanilla;
  соседний запрос comparator или сериализация не генерируют лишний loot.

### W04-017. Lock, custom name и права открытия

Зависимость: готовый container consumer. Эталон: `BaseContainerBlockEntity.java`, `LockCode.java`.

- Сверить современный lock predicate 26.3, имя меню и проверку stack в руках.
- Не заменить lock сравнением одного display name из старой версии Minecraft.
- Приёмка: подходящий/неподходящий ключ, creative/spectator по эталону, обе руки
  где применимо; lock/name переживают item-component и disk round-trip.

### W04-018. Закрытие меню при всех изменениях состояния

Связь: W03-09. Эталон: `AbstractContainerMenu.java`, `ServerPlayer.java`.

- Проверить смерть, disconnect, dimension transfer, removal блока, distance/validity.
- Возврат transient inputs/cursor и drops не должен выполняться дважды.
- Callback take/finish должен восстановить menu при каждом error/return path.
- Приёмка: закрытие в середине crafting/anvil/lectern operation, повторный close;
  нет потерянного курсора, дублирования inputs и оставшегося viewer count.

### W04-019. Авторитетный sync меню и properties

Зависимость: 018. Эталон: `AbstractContainerMenu.java`, `ServerGamePacketListenerImpl.java`.

- Сверить container id, state id, suppressed remote updates/full resync и data slots.
- Не копировать клиентский changed-slots map в серверное состояние как истину.
- Приёмка: устаревший click, неверное окно, output update и property update после
  server tick; клиенту отправляется восстановленное состояние по протоколу 26.3.

### W04-020. Два игрока у одного живого контейнера

Зависимость: 018–019, конкретный shared container. Эталон: `ContainerOpenersCounter.java`.

- Проверить последовательные interleaved clicks, внешнее изменение и watchers.
- Правильно разделить общее содержимое и per-player cursor/menu state.
- Приёмка: один забирает stack, второй кликает stale preview, затем блок ломается;
  items учитываются ровно один раз. Harness-test отдельно от реальных двух клиентов.

## C. Сундуки и перемещение предметов

### W04-021. Одиночный сундук как полноценный блок

Связь: W03-10. Эталон: `ChestBlock.java`, `ChestBlockEntity.java`, `ChestMenu.java`.

- Подключить настоящий BE/container/menu, размещение, доступ и drops.
- Проверить открывание, перекрытие крышки и block/entity events по 26.3.
- Приёмка: два игрока, comparator, destruction, unload/reload; contents сохраняются.

### W04-022. Двойной сундук и порядок половин

Зависимость: 021. Эталон: `ChestBlock.java`, `CompoundContainer.java`.

- Сверить shape/join orientation, порядок slots, объединение menu и invalidation.
- Разделение сундука не должно держать устаревший объединённый container handle.
- Приёмка: установить/сломать обе стороны в разном порядке, выгрузить половину,
  открыть с каждой стороны и перенести предметы через границу половин.

### W04-023. Сундук: viewers, lid и повторная проверка открытий

Зависимость: 021–022. Эталон: `ContainerOpenersCounter.java`, `ChestLidController.java`.

- Сверить start/stop open, scheduled recount, spectator и смену открытого меню.
- Не хранить вечный счётчик, который ломается при disconnect или block removal.
- Приёмка: два открытия, один disconnect, разделение double chest, повторный вход;
  sounds/events/viewers согласованы и не создают вечную открытую крышку.

### W04-024. Сундук-ловушка и сигнал

Зависимость: 023. Эталон: `TrappedChestBlock.java`, `TrappedChestBlockEntity.java`.

- Провести реальные opener changes в redstone neighbor updates.
- Пределы и направление сигнала брать из эталона, не вводить собственную шкалу.
- Приёмка: один/несколько игроков, закрытие, disconnect, double form;
  comparator contents и trapped opening signal не подменяют друг друга.

### W04-025. Медные сундуки и преобразования

Зависимость: 021–023. Эталон: `CopperChestBlock.java`, `WeatheringCopperChestBlock.java`.

- Довести существующий copper chest behavior: container, menu, viewers.
- Окисление, воск и очистка сохраняют contents/name/lock при смене block state/type.
- Приёмка: все поддержанные extracted variants, соединение по vanilla правилам,
  transform при открытом меню, save/load и взаимодействие с автоматизацией.

### W04-026. Совместимость barrel, shulker и ender storage

Основание: существующие реализации. Эталон: `BarrelBlockEntity.java`,
`ShulkerBoxBlockEntity.java`, `EnderChestBlockEntity.java`.

- Проверить consumers после общих изменений 011–020, не реализовывать их заново.
- Сохранить per-player ender storage и ограничения вложения shulker по компонентам.
- Приёмка: creative pick/place, разрушаемый filled shulker, disconnect;
  состояние игроков не смешивается, предметы/замки не исчезают.

### W04-027. Воронка: настоящий tick и перенос

Связь: W03-14. Основание: работающий container и world ticker. Эталон: `HopperBlockEntity.java`.

- Реализовать или довести push/pull/item pickup, cooldown и redstone enabled state.
- Обнаружение соседнего container и порядок slots сверить по источнику.
- Приёмка: chest→hopper→chest, stack merge/limit, blocked destination, disabled
  hopper, chunk unload и восстановление cooldown без искусственного polling.

### W04-028. Sided automation и печные слоты

Зависимость: 027. Эталон: `WorldlyContainer.java`, `AbstractFurnaceBlockEntity.java`.

- Общий доступ по стороне/canPlace/canTake применить к реальным consumers.
- Проверить топливо, input, output и пустую тару; не зашивать печные исключения в hopper.
- Приёмка: доступ сверху/сбоку/снизу, stack-limit/component mismatch,
  двусторонний перенос без зацикливания locks и дублирования предметов.

### W04-029. Выбрасыватель и redstone pulse

Связь: W03-17. Зависимость: container transfer. Эталон: `DropperBlock.java`, `DropperBlockEntity.java`.

- Scheduled activation, выбор непустого слота, вставка в соседний container или eject.
- Использовать нужное vanilla random поведение; не выдавать все stacks за pulse.
- Приёмка: пустой/полный сосед, длинный powered signal, два импульса,
  disabled/removed destination; ровно наблюдаемый vanilla результат и события.

### W04-030. Автокрафтер со slot toggles

Связь: W03-18. Зависимость: 004–005, recipes, containers. Эталон: `CrafterBlockEntity.java`, `CrafterMenu.java`.

- Disabled slots, ingredient распределение, trigger, recipe matching и output routing.
- System crafting callback не требует игрока; remainders идут по точному маршруту.
- Приёмка: hopper input, disabled slot, full output container, repeated pulses,
  unload/reload; не заменять отсутствующий map post-process пустым hook.

## D. Раздатчики и дополнительные блок-сущности

### W04-031. Основа раздатчика с рабочим default action

Связь: W03-19. Эталон: `DispenserBlock.java`, `DispenserBlockEntity.java`, `DefaultDispenseItemBehavior.java`.

- Сделать typed behavior dispatch с реальным default consumer и container storage.
- Registry расширения по нужному контракту, без списка случайных item ID в tick.
- Приёмка: обычный предмет, пустой dispenser, facing, pulse; расход, ItemEntity,
  sound/level event и save/load соответствуют эталону.

### W04-032. Раздатчик: готовые снаряды

Зависимость: 031 и соответствующий 061–066. Эталон: `ProjectileDispenseBehavior.java`.

- Подключить реальные зарегистрированные projectile consumers, начальную позицию,
  скорость/разброс, item payload и отсутствие player owner где vanilla его не даёт.
- Начать с уже готового снаряда; остальные оставить именованными зависимостями.
- Приёмка: каждый подключённый тип действительно летит и попадает, расходует
  один предмет и сохраняет данные; registration-only результат не принимается.

### W04-033. Раздатчик: вёдра и fluid pickup

Зависимость: 031, fluid contracts. Эталон: `DispenseItemBehavior.java`, `BucketItem.java`.

- Реальные placement/pickup условия, returned bucket и fallback default dispense.
- Различить контейнеры жидкости/водных существ; не придумывать упрощённую жидкость.
- Приёмка: source/flowing fluid, занятая/доступная цель, лишняя тара при полном
  dispenser; компоненты существ не теряются при поддержанном bucket path.

### W04-034. Раздатчик: экипировка

Зависимость: 031, equipment consumers. Эталон: `EquipmentDispenseItemBehavior.java`.

- Проверить фактическое имя/регистрацию behavior в 26.3 и условия выбора living entity.
- Передача stack, equipment slot, drop chance и fallback соответствуют consumer.
- Приёмка: подходящая сущность, занятый слот, неподходящая цель, full stack;
  нет дублирования экипировки или расхода при непрошедшем action.

### W04-035. Раздатчик: взаимодействующие предметы

Зависимость: 031, готовые block/item mechanics. Эталон: `DispenseItemBehavior.java`.

- По очереди довести bonemeal, ignition, bottles, shears и остальные реальные
  registrations эталона; таблицу поддержки вести по consumers, не обещать всё сразу.
- В каждом action сверить success/failure, durability, residue и fallback.
- Приёмка: успешная/неуспешная цель каждого подключённого item, full inventory
  dispenser, game/level events; не симулировать отсутствующий рост/стрижение.

### W04-036. Зельеварка: полный brewing cycle

Связь: W03-24. Эталон: `BrewingStandBlockEntity.java`, `BrewingStandMenu.java`, `BrewingRecipe.java`.

- Typed mix data, fuel, ingredient change, время и valid bottle slots.
- Сверить hopper access, custom potion components, bottle-state updates и save.
- Приёмка: обычный recipe chain, прерванный brew, invalid ingredient,
  отсутствие fuel, extraction при завершении; recipe/protocol данные из extractor.

### W04-037. Украшенный горшок

Связь: W03-25. Эталон: `DecoratedPotBlock.java`, `DecoratedPotBlockEntity.java`.

- Вставка, stacked-item/components, wobble event, comparator и разрушение.
- Decorations/custom name и contained item проходят компонентный round-trip.
- Приёмка: заполнение до предела, несовместимый item, hopper interaction,
  разрушение разным инструментом и restore после chunk reload.

### W04-038. Знамёна как блоки и предметы

Связь: W03-26. Эталон: `BannerBlockEntity.java`, `BannerItem.java`.

- Standing/wall варианты, base color, patterns и item components не расходятся.
- Сохранить существующий loom: его registry/tags не зависят от recipe book.
- Приёмка: создать в loom → установить → pick/drop → установить снова;
  порядок узоров и components сохраняется, invalid patterns обрабатываются по codec.

### W04-039. Shelf 26.3

Связь: W03-27. Эталон: `ShelfBlock.java`, `ShelfBlockEntity.java`.

- Сверить точную механику версии: слоты, сторона попадания, redstone/соединения,
  обмен с руками/hotbar и компоненты; не переносить поведение bookshelf по аналогии.
- Приёмка: крайние hit positions, заполненные руки/слоты, linked shelves,
  разрушение, automation если оно есть в источнике, save/load и pick/place.

### W04-040. Замыкание маршрута рабочих станций

Зависимость: изменённые 001–039. Эталон: конкретные menu/block classes.

- Пройти craft → stonecut → smith → grind → storage, сохраняя component-rich stack.
- Проверить input change перед take, menu close и блокирование результата.
- Приёмка: один компактный end-to-end harness и список клиентских шагов;
  не объявлять станции полными при незакрытых awards/recipe/component зависимостях.

## E. Станции, транспорт и контейнеры в движении

### W04-041. Наковальня: стоимость и действительный take

Связь: W03-07. Эталон: `AnvilMenu.java`, `AnvilBlock.java`.

- Сверить repair/combine/rename, prior-work penalty, curses и component preservation.
- Creative, too-expensive и invalid recipe не заменять общим правилом стоимости.
- XP/input расходуются на take; rename packet проверяет текущее menu и inputs.
- Приёмка: смена input после preview, shift-take с почти полным inventory,
  одноразовый расход XP, vanilla sound и повреждение наковальни.

### W04-042. Точило: сохранение curses и опыт

Связь: W03-07. Эталон: `GrindstoneMenu.java`.

- Разобрать disenchant/repair ветки, combined durability и несовместимые stacks.
- XP берётся из реально снятых enchantments и выдаётся на правильной границе.
- Приёмка: curse-only item, обычный enchanted item, enchanted book, повторный take,
  creative и смена inputs; component-preservation проверить на нетривиальном stack.

### W04-043. Печь: smelting и управление временем

Связь: W03-08. Эталон: `AbstractFurnaceBlockEntity.java`, `AbstractFurnaceMenu.java`.

- Сверить fuel acquisition, burn/cook progress, recipe change и output capacity.
- Derived lit state и menu properties обновляются при реальном server tick.
- Приёмка: fuel без input, invalid output, partial progress, потеря топлива,
  смена input и reload в середине готовки; no async wait на игровом тике.

### W04-044. Печь: recipe usage, output stats и опыт

Зависимость: 043; 008/090 для зависимых awards. Эталон: `FurnaceResultSlot.java`.

- Сверить used-recipes accounting, долевой XP и границы ручного/автоматического take.
- Hopper не получает player stats; поздний player extraction не дублирует XP.
- Приёмка: несколько smelting recipes до take, частичный output, hopper removal,
  разрушение печи, save/load accumulated usage; отсутствие award consumer явно указать.

### W04-045. Коптильня и плавильная печь

Зависимость: 043–044. Эталон: `SmokerBlockEntity.java`, `BlastFurnaceBlockEntity.java`.

- Использовать общую корректную furnace основу, разные recipe types/time как в source.
- Не копировать весь furnace tick в отдельные неподдерживаемые реализации.
- Приёмка: принимаемые/отклоняемые recipes, fuel container residue, разные времена,
  правильный menu/properties и один полный save/load cycle для каждого consumer.

### W04-046. Остатки топлива и автоматизация станций

Зависимость: 028, 036, 043–045. Эталон: соответствующие container classes.

- Сверить returned containers, bottle/fuel slots и sided restrictions в общей цепи.
- Проверить arbitration между player take и hopper tick без повторного acquire locks.
- Приёмка: lava-bucket fuel, output full, смена target, две hopper вокруг станции;
  сумма предметов/остатков совпадает с vanilla сценарием, exceptions не скрываются.

### W04-047. Существующая chest minecart

Связь: W03-36. Эталон: `MinecartChest.java`, `AbstractMinecartContainer.java`.

- Проверить текущую реализацию: открытие, loot, drop при destruction, menu validity.
- Contents и vehicle state сохраняются вместе; hopper видит движущийся container.
- Приёмка: открыть из двух мест по пути движения, entity removal при открытом меню,
  unload/reload и extraction; не создавать второй несовместимый chest inventory.

### W04-048. Лодки и плоты: базовая физика

Связь: W03-35. Эталон: `AbstractBoat.java`, `Boat.java` и конкретные boat classes.

- Placement, collision, buoyancy, paddle/input, passengers и damage/drop.
- Управление и authoritative correction сверить с packet/vehicle ownership в 26.3.
- Приёмка: вода→берег, два пассажира, выход под препятствием, destruction,
  клиентское движение после reload; список variants брать из generated registries.

### W04-049. Лодки и плоты с сундуком

Зависимость: 021, 048. Эталон: `AbstractChestBoat.java`, `ContainerEntity.java`.

- Согласовать open inventory и посадку по secondary action, loot и contents drops.
- Container validity следует живой сущности; disconnect не удерживает её в памяти.
- Приёмка: открытие на воде, destruction с items, transfer через hopper где
  vanilla допускает, save/load и player dismount без потери содержимого.

### W04-050. Вагонетки, рельсы и отдельные variants

Связь: W03-36. Эталон: `AbstractMinecart.java`, `MinecartBehavior.java`.

- Сначала один законченный обычный cart: slope, curves, acceleration, collision,
  passengers; затем отдельные consumers furnace/hopper/TNT при готовых основаниях.
- Не объявлять все minecarts готовыми по одному physics base или регистрации.
- Приёмка: замкнутый rail route, powered/braking rails, сход, reload в движении;
  TNT variant ждёт 068–069, hopper variant — 027–028.

## F. Декор, выпавшие предметы и использование

### W04-051. Картины: варианты, размер и опора

Связь: W03-28. Эталон: `Painting.java`, `PaintingVariant.java`, `HangingEntityItem.java`.

- Placement выбирает реально помещающийся variant по данным/tags и vanilla выбору.
- Bounding box, support loss, damage/drop, spawn metadata и NBT должны работать вместе.
- Приёмка: несколько размеров, недостаточная стена, пересечение с hanging entity,
  break support, save/load и правильный entity pick без отката N19.

### W04-052. Стойка для брони: установка и экипировка

Связь: W03-29. Эталон: `ArmorStand.java`, `ArmorStandItem.java`.

- Placement hit/rotation, flags/poses и equipment slots через общие contracts.
- Сверить disabled slots, arms, small/marker и swap rules конкретной версии.
- Приёмка: обе руки, занятый equipment slot, creative/survival, недоступный slot,
  component-rich armor; состояние видно packet/metadata и переживает reload.

### W04-053. Стойка для брони: damage и drops

Зависимость: 052 и общий damage. Эталон: `ArmorStand.java`.

- Разобрать player attacks, projectiles, fire/explosion и flags, не один remove-on-hit.
- Сохранить корректные equipment drops и doEntityDrops/creative ветки.
- Приёмка: повторный удар/разные damage sources, invisible/marker по source,
  break с экипировкой, reload; один предмет не выпадает дважды.

### W04-054. Cushion и пассажиры

Связь: W03-30. Эталон: `Cushion.java`, `CushionItem.java`.

- Placement/support, interaction, seating и collision по 26.3, не по стороннему плагину.
- Использовать общие riding/dismount contracts и существующий entity pick.
- Приёмка: посадка, повторное действие, занятое место, снятие опоры, disconnect,
  save/load; зарегистрированная неподвижная entity не считается рабочим сиденьем.

### W04-055. ItemEntity: damage и уничтожение содержимого

Связь: W03-31. Эталон: `ItemEntity.java`, `Entity.java`.

- Сверить базовую invulnerability и damage tags, item resistance и lifecycle.
- Item destruction callbacks должны видеть stack, в том числе bundle contents.
- Приёмка: огонь/лава/explosion/immune item по данным, doEntityDrops,
  контейнерный предмет; общий damage путь не подменять локальным if.

### W04-056. ItemEntity: merge, age и owner

Связь: W03-32. Эталон: `ItemEntity.java`.

- Merge ограничен item/components/count, pickup delay, age и target/owner правилами.
- Выбор сохраняемой entity и возраст после merge взять из source.
- Приёмка: одинаковые и разные components, max stack, разные owner/delay,
  despawn boundary; не использовать приблизительное «похожее содержимое».

### W04-057. ItemEntity: pickup и сохранение

Зависимость: 056. Эталон: `ItemEntity.java`, `Inventory.java`.

- Частично заполненный inventory должен принять только вместившееся количество.
- Player stats/events, оставшийся stack и entity removal синхронизируются один раз.
- Приёмка: два игрока по очереди подбирают один stack, forbidden owner, delay,
  save/reload остатка; запрещённый pickup не исчезает только на клиенте.

### W04-058. Вёдра с существами

Связь: W03-33. Эталон: `MobBucketItem.java`, `Bucketable.java`.

- Сохранить type-specific data, name, health и only-from-bucket flags по consumers.
- Выпуск создаёт настоящую поддержанную entity через общую factory, не NBT marker.
- Приёмка: capture→inventory→save→release, обе руки, creative и занятая цель;
  неизвестные mob foundations остаются зависимостью, без декоративных substitutes.

### W04-059. Длительное использование и щит

Связь: W03-34. Эталон: `LivingEntity.java`, `ShieldItem.java`, `ItemStack.java`.

- Start/use/release/finish/cancel и active-hand state связать с реальным stack.
- Shield blocking берёт правила из components и damage context, включая disable.
- Приёмка: смена held stack, teleport/death/menu action во время use,
  фронтальный/обратный удар и release; без залипшей анимации или лишнего расхода.

### W04-060. Рыбалка и фейерверки: реальные оставшиеся ветки

Связь: W03-45. Эталон: `FishingHook.java`, `FireworkRocketEntity.java`.

- Проверить уже существующие implementations и закрыть конкретные source mismatches.
- Рыбалка: owner/retract/loot/state; ракета: lifetime, explosion payload и elytra use.
- Приёмка: обычный и неуспешный сценарий, save/reload живой entity, owner loss;
  отсутствующая система loot/damage/flight должна быть явной зависимостью.

## G. Снаряды, оружие и взрывы

### W04-061. Общий projectile путь на конкретном consumer

Основание: существующие throwable projectiles. Эталон: `Projectile.java`, `ProjectileUtil.java`.

- Сверить swept hit, block/entity ordering, owner exclusion и left-owner state.
- Исправлять общую основу только вместе с regression существующего snowball/egg/etc.
- Приёмка: тонкое препятствие, быстрый полёт, owner рядом, chunk boundary,
  сохранение скорости/owner; клиентское отображение не заменяет server collision.

### W04-062. AbstractArrow: полёт, втыкание и подбор

Связь: W03-37. Зависимость: 061. Эталон: `AbstractArrow.java`.

- Воздух/вода, in-ground transition, piercing, bounce и pickup policy по source.
- Owner/damage/pickup stack сохраняются; block change освобождает стрелу по vanilla.
- Приёмка: попадание в блок/живую entity, застрявшая стрела после reload,
  creative-only pickup, missing owner и повторное collision без двойного урона.

### W04-063. Обычные, tipped и spectral arrows

Зависимость: 062, эффекты. Эталон: `Arrow.java`, `SpectralArrow.java`.

- Реальные typed arrow factories, effects/components, glowing/duration и item return.
- Duration и probability берутся из эталона/данных, не воспроизводятся по памяти.
- Приёмка: одинаковый базовый shot для трёх consumers, effects после попадания,
  pickup/restored item, save/load; все variants не закрывать тестом registry name.

### W04-064. Лук: натяжение и выпуск

Связь: W03-39. Зависимость: 059, 062–063. Эталон: `BowItem.java`, `ProjectileWeaponItem.java`.

- Выбор боеприпаса, draw progress, release speed, damage/enchantment и расход.
- Creative/infinity-type conditions сверить с components/enchantments 26.3.
- Приёмка: короткое/полное натяжение, нет стрел, обе руки, смена item при use,
  durability и вернувшаяся pickup arrow; нельзя spawn только visual packet.

### W04-065. Арбалет: charge components и выпуск

Связь: W03-40. Зависимость: 064 и готовые projectile types. Эталон: `CrossbowItem.java`.

- Charge stages, charged_projectiles, discharge и multishot по реальным data.
- Firework payload подключать к готовому rocket consumer, не к default arrow.
- Приёмка: charge→сменить слот→save/load→fire, отменённая зарядка,
  multishot, partial ammo; расход и durability не повторяются на release.

### W04-066. Трезубец и возврат

Связь: W03-41. Зависимость: 059, 061, equipment/damage. Эталон: `TridentItem.java`, `ThrownTrident.java`.

- Бросок сохраняет исходный stack; pickup/loyalty возврат учитывает owner и мир.
- Riptide — отдельный action с условиями среды и motion, не teleport к цели.
- Приёмка: hit/miss, full inventory при возврате, owner death/disconnect,
  reload и dimension change; stack не существует одновременно в руке и projectile.

### W04-067. Wind charge и block interactions

Связь: W03-42. Зависимость: 061, соответствующая explosion форма. Эталон: `WindCharge.java`.

- Сверить импульс, hit behavior, доступные block interactions и fall-distance effects.
- Не заменить весь взрыв произвольным radial knockback без источника.
- Приёмка: entity/block hit, рядом interactable blocks, owner jump,
  fluids/обратная сторона стены; наблюдаемая зона действия совпадает с source.

### W04-068. Общая система взрыва с первым настоящим consumer

Связь: W03-43. Эталон: `ServerExplosion.java`, `Explosion.java`.

- Рассчитать affected blocks/entity exposure, damage, knockback и destruction policy.
- Использовать gamerules/tags/loot context и нужный RNG contract; не сферу фиксированного radius.
- Первый consumer должен действительно менять мир и отправлять нужную синхронизацию.
- Приёмка: стена между взрывом и entity, fluids/resistant blocks, container contents,
  doTileDrops/другие actual rules; не теряются pending drops и block updates.

### W04-069. TNT и end crystal как consumers взрыва

Связь: W03-44. Зависимость: 068. Эталон: `PrimedTnt.java`, `EndCrystal.java`, `TntBlock.java`.

- Ignition, fuse, chain reaction, owner/damage attribution и save/load живой TNT.
- Crystal placement/support/damage использует существующий entity контракт.
- Приёмка: redstone/fire/projectile triggers по vanilla, chain TNT, unload/reload,
  crystal hit; размещаемая модель кристалла без взрыва не закрывает карточку.

### W04-070. Ближний бой и общий damage pipeline

Связь: W03-56. Эталон: `Player.java`, `LivingEntity.java`, `DamageSource.java`.

- Attack strength, reach/target validity, armor/toughness, enchantment effects,
  knockback, invulnerability frames и death path проверять в верном порядке.
- Data-driven weapon behavior 26.3 сверить по components, не по старой таблице мечей.
- Приёмка: быстрые/полные атаки, shield, armor, rejected target и PvP settings
  где они есть; источник урона сохраняется для stats/loot без фиктивных events.

## H. Конкретные мобы и их жизненный цикл

### W04-071. Зомби и минимально необходимая mob основа

Связь: W03-46/47. Эталон: `Mob.java`, `PathfinderMob.java`, `Zombie.java`.

- Выбрать конкретного zombie consumer и довести spawn, goal ticks, navigation,
  target selection, melee, equipment и death/loot в существующей архитектуре.
- Не строить универсальный AI framework без работающего моба и не имитировать Brain.
- Приёмка: зомби находит доступного игрока, идёт вокруг препятствия, атакует,
  теряет цель и переживает reload; разновидности отдельно после проверенного base.

### W04-072. Скелет и дальний бой

Связь: W03-48. Зависимость: 062–064, 071. Эталон: `AbstractSkeleton.java`, `Skeleton.java`.

- Bow/equipment goals, visibility, cooldown и выбор melee/ranged по source.
- Выпускается настоящий снаряд с owner/damage; сложность влияет по vanilla.
- Приёмка: укрытие, приблизившийся игрок, смена оружия, daylight branches,
  reload; не стрелять сквозь стену только потому, что target существует.

### W04-073. Паук и пещерный паук

Связь: W03-49. Зависимость: mob/navigation/damage. Эталон: `Spider.java`, `CaveSpider.java`.

- Climbing state, target conditions, melee reach и poison consumer для варианта.
- Унаследованные goals не копировать с зомби без сверки эталона.
- Приёмка: вертикальная поверхность, obstacle, player target loss,
  poison difficulty rules и reload; metadata climbing соответствует движению.

### W04-074. Крипер: swelling и настоящий взрыв

Связь: W03-50. Зависимость: 068, 071. Эталон: `Creeper.java`, `SwellGoal.java`.

- Fuse/swelling state, отмена по условиям, charged flag и ignition interaction.
- Explosion gamerule/source attribution использует общую систему.
- Приёмка: player входит/выходит из дальности, obstacle, ignition,
  charged consumer и save/load в середине fuse; dropped loot по реальному damage path.

### W04-075. Слизень: размер, движение и разделение

Связь: W03-51. Эталон: `Slime.java`.

- Size-derived dimensions/attributes, jumping goals, contact damage и split on death.
- Spawned children — реальные entities; ограничения spawn и persistence по данным.
- Приёмка: несколько размеров, смертельный удар, children ownership/state,
  collision в тесном месте и reload; разделение не повторяется после removal.

### W04-076. Полезный цикл существующих животных

Связь: W03-52. Основание: pig/cow/sheep/chicken/mooshroom implementations.
Эталон: `Animal.java`, конкретные species classes.

- Проверить уже готовые breeding/age/follow, кормление, milk/shear/egg consumers.
- Довести подтверждённые пробелы, используя общий age/breeding foundation.
- Приёмка: два взрослых → ребёнок → сохранение возраста → adult,
  species interaction и loot; не считать NBT-сохраняемую entity полноценным животным.

### W04-077. Волк и кошка: taming, owner и sit

Связь: W03-53. Эталон: `TamableAnimal.java`, `Wolf.java`, `Cat.java`.

- Owner UUID, tame interaction, sit/follow/teleport и команды игрока по source.
- Equipment/variant data брать из registry; чужой игрок не управляет питомцем.
- Приёмка: failed/successful tame, owner disconnect/reconnect, blocked following,
  owner death и save/load; не хранить сильный Player ref как вечного владельца.

### W04-078. Естественный спавн готовых мобов

Связь: W03-54. Эталон: `NaturalSpawner.java`, `SpawnPlacements.java`.

- Категории/caps, chunk selection, local conditions и group spawn по данным 26.3.
- Включать только реально реализованные species без подмены одного mob type другим.
- Приёмка: eligible/ineligible biome, освещение/поверхность по source, игроки,
  cap и despawn; seeded части выбора не переводить на incidental RNG.

### W04-079. Обычный spawner с конкретным мобом

Связь: W03-70. Зависимость: готовая entity factory. Эталон: `BaseSpawner.java`, `SpawnerBlockEntity.java`.

- Delay, player proximity, spawn potentials/NBT и nearby-entity limits.
- Spawner использует actual placement/finalize spawn rules по конкретной ветке.
- Приёмка: активен/неактивен, blocked space, multiple potentials,
  save/reload delay и chunk unload; particles/event без spawn не считаются реализацией.

### W04-080. Entity lifecycle на границе AI, loot и persistence

Зависимость: реализованные 071–079. Эталон: `Mob.java`, `LivingEntity.java`, `Entity.java`.

- Removal, death, despawn, equipment drops, passengers и owner references не смешивать.
- Save/load восстанавливает durable state; runtime goals/targets восстанавливаются по vanilla.
- Приёмка: смерть во время атаки, unload с целью/пассажиром, duplicate UUID,
  повторная загрузка; нет повторного loot и утечки world/entity handles.

## I. Выживание, мир и прогресс игрока

### W04-081. Добыча блоков, инструмент и drops

Связь: W03-55. Эталон: `ServerPlayerGameMode.java`, `Block.java`, `ItemStack.java`.

- Start/abort/finish mining, tool speed/correctness, enchantments, durability и loot context.
- Ограничения режима и interaction distance проверяются до выдачи drops.
- Приёмка: подходящий/неподходящий tool, смена held stack, block replacement
  во время digging, creative/adventure; drop не выдаётся за несостоявшееся разрушение.

### W04-082. Голод, насыщение и употребление

Связь: W03-57. Эталон: `FoodData.java`, `Consumable.java`, `FoodProperties.java`.

- Сверить exhaustion, regeneration/starvation и difficulty rules на тиках.
- Item use завершает реальный consumable с effects, residue и компонентами 26.3.
- Приёмка: полная/неполная hunger, interrupted use, всегда съедобный item,
  residue при полном inventory, reload; не расходовать stack дважды.

### W04-083. Смерть, опыт и respawn

Связь: W03-58. Эталон: `ServerPlayer.java`, `Player.java`, `ExperienceOrb.java`.

- Разделить death drops, keepInventory, curse/equipment rules, XP и respawn copy.
- Завершить active use/menus и корректно перенести разрешённые player data.
- Приёмка: смерть с cursor/container, несколько orbs, disconnect до respawn,
  новый player instance и reload; старый entity/inventory не остаётся доступным.

### W04-084. Кровать, сон и ночь

Связь: W03-59. Эталон: `BedBlock.java`, `ServerPlayer.java`, `SleepStatus.java`.

- Условия сна/пробуждения, respawn position, occupied state и sleep threshold.
- Неверное измерение использует настоящий explosion consumer при готовом 068.
- Приёмка: obstructed bed, несколько игроков, опасность, destruction во сне,
  успешный пропуск ночи и последующий respawn; не телепортировать к сломанной кровати.

### W04-085. Измерения, порталы и подтверждение телепорта

Связь: W03-60. Эталон: `PortalProcessor.java`, `PortalForcer.java`, `PortalShape.java`.

- Проверить existing transition pipeline, scaled coordinates, cooldown и placement.
- Loading destination идёт через server jobs; tick не ждёт chunk I/O в цикле.
- Приёмка: портал туда/обратно, missing destination portal, stale teleport ack,
  passengers и disconnect в пути; один player не числится в двух мирах одновременно.

### W04-086. Жидкости и движение в них

Связь: W03-61. Эталон: `FlowingFluid.java`, `LivingEntity.java`.

- Распространение, scheduled ticks, waterlogging, liquid interaction и entity movement.
- Различить server simulation и client-local particles/sounds, не рассылать лишнее.
- Приёмка: граница чанков, blocked flow, lava/water meeting, breathing/fall damage,
  bucket source и reload pending ticks; fixed seed cases при deterministic расчётах.

### W04-087. Огонь, дождь и молния

Связь: W03-62. Эталон: `FireBlock.java`, `LightningBolt.java`, `ServerLevel.java`.

- Spread/extinguish, gamerules, weather checks, lightning damage/transformation consumers.
- Не выполнять все client-local эффекты как server broadcasts.
- Приёмка: дождь под крышей/снаружи, protected block, entity on fire,
  lightning rod/consumer где уже готов, save/reload; RNG/determinism по конкретной системе.

### W04-088. Редстоун и поршни на реальной машине

Связь: W03-63. Эталон: `RedstoneWireBlock.java`, `PistonBaseBlock.java`, `PistonMovingBlockEntity.java`.

- Сверить neighbor update/scheduled tick порядок, quasi-ветки только из source.
- Проверить push limits/sticky behavior и запреты перемещения actual block entities.
- Приёмка: clock→hopper/dropper/crafter chain, sticky piston и solid support,
  reload moving block; нет рекурсивного spin loop или «успешно» без обновления соседей.

### W04-089. Loot и enchantment effects на реальных событиях

Связь: W03-71. Эталон: `LootTable.java`, `EnchantmentHelper.java`.

- Проверить context params, tool/entity/source/luck и component-driven effects 26.3.
- Привязать конкретные mining/melee/projectile/loot consumers, не таблицу no-op hooks.
- Приёмка: известные seeded inputs дают source-compatible результат;
  отсутствующие params и неверные predicates не превращаются в произвольный fallback.

### W04-090. Advancement triggers, включая recipe crafted

Связь: W03-71. Зависимость: соответствующие working consumers. Эталон: `PlayerAdvancements.java`, `RecipeCraftedTrigger.java`.

- Сначала проверить существующий progress/criteria/persistence и predicate foundations.
- Подключить реальные inventory/craft/death/etc events с snapshot в правильной фазе.
- Приёмка: trigger один раз, несколько criteria, revoke/reload, невозможный
  predicate; 010 не считать полным без реального recipe-crafted consumer.

## J. Карты, книги и поздние игровые основания

### W04-091. Canonical component JSON и лимит страниц

Связь: W03-64. Эталон: `WrittenBookContent.java`, `ComponentSerialization.java`.

- Определить версию/owner component codec dependency и поддерживаемый путь правки.
- Сериализация должна давать vanilla форму JSON и точную UTF-16 длину там, где она нужна.
- Приёмка: astral Unicode, escaped strings, styles/siblings/resolvables,
  точная граница длины и resolution failure fallback; не мерить serde output по UTF-8 bytes.

### W04-092. Durable map data и выдача идентификаторов

Связь: W03-67. Эталон: `MapItemSavedData.java`, `MapIndex.java`.

- Проверить SavedData/storage архитектуру, world/dimension ownership и durable ID counter.
- Ввести настоящую map record: center, scale, flags, colors, decorations по source.
- Приёмка: создать несколько карт, сохранить сервер, загрузить и создать ещё;
  ID не конфликтуют и не сбрасываются. При неясном storage owner спросить по AGENTS.

### W04-093. Empty map и первичная синхронизация карты

Зависимость: 092. Эталон: `EmptyMapItem.java`, `MapItem.java`, `ClientboundMapItemDataPacket.java`.

- Use создаёт реальные map data, заменяет/возвращает stack и связывает map_id.
- Packet codec и initial patch/decoration data брать из реального stream codec.
- Приёмка: обе руки, creative/survival, полный inventory, initial sync/rejoin;
  не отправлять пустую фиктивную картинку вместо map storage foundation.

### W04-094. Обновление карты, markers и рамки

Связь: W03-68/N18. Зависимость: 093, готовые item frames. Эталон: `MapItem.java`, `MapItemSavedData.java`, `ItemFrame.java`.

- Colors/dirty regions и per-player tracking сверить с dimension/scale/holding rules.
- Frame decoration add/remove, limits и framed-map cleanup использовать actual manager.
- Приёмка: карта у двух игроков и в рамке, поворот/разрушение рамки, unload/reload,
  rejoin и движение; не копить ghost markers и не отправлять весь map без необходимости.

### W04-095. Картографический стол и map post-processing

Зависимость: 005, 092–094. Эталон: `CartographyTableMenu.java`, `MapItem.java`.

- Scale/lock/clone recipes используют настоящие map data и нужные component mutations.
- Довести `onCraftedPostProcess`, если он был зависимостью crafter/item callbacks.
- Приёмка: take/shift-take, повторное взятие, invalid input, map_id/locked state
  после save/reload; не изменять original map неожиданно при клонировании.

### W04-096. Стол зачарований и seeded offers

Связь: W03-66. Эталон: `EnchantmentMenu.java`, `EnchantmentHelper.java`.

- Bookshelf access, player enchantment seed, offer/cost generation и eligible data.
- Выбор кнопки валидирует current input/lapis/XP; post-action seed обновляется по source.
- Приёмка: одинаковый seed/input/context, invalid selection, creative,
  смена input после preview и rejoin; не использовать unseeded RNG для offers.

### W04-097. Вибрации и sculk sensor

Связь: W03-69. Эталон: `VibrationSystem.java`, `SculkSensorBlockEntity.java`.

- Настоящий game-event consumer, distance/delay, occlusion и frequency data.
- Sensor phases, calibrated variant и редстоун-сигнал без ручного shortcut от игрока.
- Приёмка: event через стену/шерсть по source, несколько events в одном tick,
  cooldown, reload pending vibration; не заменять систему timer-only animation.

### W04-098. Trial spawner и vault как отдельные системы

Связь: W03-70. Зависимость: 079, реальные entity/loot foundations.
Эталон: `TrialSpawner.java`, `TrialSpawnerBlockEntity.java`, `VaultBlockEntity.java`.

- Прежде чем писать, сверить state machine, saved state, players/rewards и registry data.
- Реализовывать законченный consumer по одному; обычный spawner не выдавать за trial.
- Приёмка: participant joins/leaves, wave завершение, cooldown и single reward,
  reload; если отсутствует общий foundation, точный blocker вместо fake rewards.

### W04-099. Seeded generation: первый воспроизводимый мир

Связь с дальнейшими 101–104. Эталон: `ChunkGenerator.java`, `RandomState.java`.

- Выбрать фиксированные seed/dimension/chunk cases и сравнимые intermediate stages.
- Проверить реальные worldgen RNG ownership и порядок потребления randomness.
- Приёмка: повторный запуск и разный порядок запросов chunks дают vanilla-совместимые
  результаты на выбранных этапах; не объявлять всю генерацию готовой по одному hash.

### W04-100. Первый сквозной survival-маршрут

Связь: W03-72. Зависимость: фактически реализованные системы; не требование ждать все 120.

- Мир → добыча → crafting → печь → storage → бой → drops → death/respawn → reload.
- Добавить component-rich предмет и один сценарий с двумя игроками/automation.
- Приёмка: сохранить before/after state, реальные commands/logs и точные
  незакрытые шаги; тестовый harness, настоящий клиент и Mojang comparison указать отдельно.

## K. Генерация, чанки и сохранность большого мира

Карточки 101–110 — расширение предыдущей очереди. Они требуют **проверки текущего
унаследованного кода**, а не полной замены world engine. Начинай с минимального
source-confirmed расхождения и воспроизводимого seed/chunk/state.

### W04-101. Chunk generation pipeline и статусы

Зависимость: 099. Эталон: `ChunkStatus.java`, `ChunkGenerator.java`, `ChunkPyramid.java`.

- Сверить order/dependencies этапов, proto→full conversion и соседние chunk access.
- Не исполнять feature дважды при повторном job completion или concurrent request.
- Приёмка: одинаковый chunk запрошен из двух мест, отменённый/повторный job,
  сохранение на допустимом статусе; tick не ждёт worker в цикле.

### W04-102. Noise, биомы и surface generation

Зависимость: 101. Эталон: `NoiseBasedChunkGenerator.java`, `NoiseChunk.java`, `MaterialSystem.java`.

- Изолировать расхождения density functions, interpolation, biome sampling и surface rules.
- Extracted configs компилировать в typed data, не парсить datapack JSON на runtime.
- Приёмка: выбранные seed/coordinates включая отрицательные, chunk seams,
  несколько биомов и dimension-specific cases; источник expected данных воспроизводим.

### W04-103. Carvers, ores, trees и placed features

Зависимость: 101–102. Эталон: `PlacedFeature.java`, `Feature.java`, `WorldCarver.java`.

- Сначала source-confirmed mismatch одного реального feature, затем общий helper.
- Positions, RNG streams, height/biome filters и neighbor access соответствуют version data.
- Приёмка: feature на границе chunk, отрицательная координата, seed replay,
  неподходящий biome; не закрывать весь trees/ores раздел одним успешным видом.

### W04-104. Структуры, шаблоны и начальные блок-сущности

Зависимость: 101. Эталон: `StructureStart.java`, `StructureTemplate.java`, `StructureTemplateManager.java`.

- Сверить placement, rotations/mirrors, processors, structure refs и entities/BE data.
- Initial NBT decoding сохраняет detached semantics 26.3; не разрешать lectern books автоматически.
- Приёмка: structure через несколько chunks, transform с directional blocks,
  loot/BE metadata, save/load и повторная генерация без duplicate entities.

### W04-105. Heightmaps и освещение

Эталон: `Heightmap.java`, `LevelLightEngine.java`, `ThreadedLevelLightEngine.java`.

- Проверить updates при setBlock, chunk edges, load/reload, sky/block light и occlusion.
- Light jobs асинхронны по необходимости; готовность chunk и packet data согласованы.
- Приёмка: закрыть/открыть световой колодец, источник на границе, transparent blocks,
  reload; не отдавать клиенту light version от предыдущего состояния chunk.

### W04-106. Сохранение чанков, entities и pending ticks

Эталон: `SerializableChunkData.java`, `EntityStorage.java`, `LevelTicks.java`.

- Сверить block/BE/components, scheduled block/fluid ticks, entity/passenger persistence.
- Write completion принадлежит server job/save системе, не блокирует game tick.
- Приёмка: unload/reload с pending furnace/fluid/redstone actions, entities на границе,
  clean shutdown и повторный save; ошибки I/O не скрывать как успешное сохранение.

### W04-107. Chunk tickets, simulation и view distance

Эталон: `DistanceManager.java`, `ChunkMap.java`, `ServerChunkCache.java`.

- Проверить отличие видимого и тикающего chunk, player movement и ticket lifecycle.
- Cancelled jobs не публикуют chunk обратно после окончательного unload.
- Приёмка: быстрое движение/teleport/rejoin, несколько игроков, изменённый view range,
  entity crossing границы; не тикать всю карту ради маскировки missing tickets.

### W04-108. Изоляция нескольких миров и domains

Основание: существующая Vecta architecture. Эталон: relevant `ServerLevel.java` gameplay paths.

- Не копировать ограничения vanilla dedicated server поверх Vecta admin architecture.
- Проверить source world для selectors, scoreboard/storage routing, spawn sync,
  entities, shared durable data и переходов; ownership каждого вида данных явный.
- Приёмка: одинаковые coordinates в двух мирах, console/player command context,
  переход/respawn/reload; данные и события не утекают в чужой world/domain.

### W04-109. Время, погода и разные clocks

Эталон: `ServerLevel.java`, `ServerLevelData.java`, `WeatherData.java`.

- Различить game time, day time, command time query и clocks конкретных миров.
- Sleep/weather/gamerule transitions и periodic events используют правильный clock.
- Приёмка: /time и /weather по effective source, смена мира, reload,
  день/ночь при остановке daylight cycle; тест не продвигает часы «похожего» мира.

### W04-110. Difficulty и gamerules на consumers

Эталон: `DifficultyInstance.java`, `GameRules.java` и конкретные consumers.

- Проверить storage/defaults/sync, затем эффекты на food, mobs, loot, fire, death.
- Не объявлять поддержку правила по наличию registration и setter.
- Приёмка: смена правила реально меняет выбранный игровой результат,
  reload сохраняет значение, другой world/domain не получает чужую настройку.

## L. Поздние системы и приёмка совместимости

### W04-111. End gateway, выход и сохранение прогресса End

Основание: существующие end portal/gateway BE. Эталон: `TheEndGatewayBlockEntity.java`, `EnderDragonFight.java`.

- Проверить teleport target/cooldown, поиск безопасного выхода и durable fight data.
- Если dragon/fight foundations отсутствуют, разделить действующий gateway и зависимый прогресс.
- Приёмка: повторный проход, unload gateway, return после save/reload,
  отсутствие duplicate exit structures; visual portal не равен готовому измерению.

### W04-112. End dragon: реальный первый законченный combat slice

Зависимость: 061/068/070 и нужные boss foundations. Эталон: `EnderDragon.java`, `EnderDragonFight.java`.

- До кода сверить multipart entities, phases, crystals, bossbar и damage routing.
- Не заменять dragon одним обычным Mob с большим health; отсутствие multipart — blocker foundation.
- Приёмка: один полный рабочий phase transition с damage и синхронизацией,
  затем death/fight progress; всю карточку держать частичной до настоящего цикла боя.

### W04-113. Торговля и жители

Эталон: `Villager.java`, `MerchantMenu.java`, `MerchantOffer.java`, `MerchantResultSlot.java`.

- Проверить existing NPC/Brain/POI/trade foundations; static NPC с окном не полный villager.
- Первый consumer: реальные offers, inputs, result take, uses/pricing и persistence.
- Приёмка: неверные items/components, shift trade, stock exhaustion, restock
  только при готовых условиях; профессии/распорядок отдельными завершёнными slices.

### W04-114. Raid, golem и связанные world events

Эталон: `Raid.java`, `Raider.java`, `IronGolem.java`.

- Сверить event lifecycle/POI, participants, spawning и saved state прежде чем писать.
- Подключить один настоящий consumer; отсутствующие patrol/village foundations не имитировать таймером.
- Приёмка: participant death/unload, отмена/завершение, повторная загрузка;
  rewards и world state выдаются один раз и только по реальному событию.

### W04-115. Поводки, посадка, питомцы и ездовые животные

Эталон: `Leashable.java`, `AbstractHorse.java`, `Entity.java` riding methods.

- Проверить leash holder, passenger order, tame/equipment/control и dismount placement.
- Concrete consumer выбирается из реально поддержанных species, без универсального no-op.
- Приёмка: holder/player disappears, unload пары, dimension transfer,
  obstacle при dismount и drop leash; owner/passenger refs не образуют вечный цикл.

### W04-116. Эффекты, attributes и equipment modifiers

Эталон: `MobEffectInstance.java`, `AttributeMap.java`, `LivingEntity.java`.

- Duration/amplifier/hidden effects и modifier identity/remove/update по source.
- Тик effect, снятие equipment и save/load не должны накапливать повторные modifiers.
- Приёмка: refresh/upgrade/downgrade effect, смерть, milk/clear consumer,
  смена брони, reload; packet state и фактический damage/speed совпадают.

### W04-117. Команды как диагностический игровой consumer

Эталон: `server/commands/ExecuteCommand.java`, `DataCommands.java`, `ScoreboardCommand.java`.

- Проверить existing selectors, source position/world, NBT queries и success/result.
- Исправлять конкретные несовместимые ветки, сохраняя Vecta permissions/admin UX.
- Приёмка: nested execute/store, два мира, source без entity, failed selector,
  component-rich entity/block data; ошибка не частично применяет предыдущее изменение.

### W04-118. Текст, таблички, книги и resolution regressions

Основание: работающие book/sign implementations. Эталон: `SignBlockEntity.java`, `WrittenBookContent.java`.

- Проверить filtered/raw text, edit permissions, waxed state и relevant serialization.
- Dynamic text resolution не выполнять под inventory/container mutex.
- Приёмка: self-NBT reference, valid multi-target score failure, Unicode,
  rejoin/reload; initial и live lectern load сохраняют разную vanilla semantics.

### W04-119. Неверные игровые пакеты и восстановление состояния

Эталон: `ServerGamePacketListenerImpl.java` и stream codecs затронутых пакетов.

- Применять vanilla validation по типам, длинам, indices/menu ids и состоянию игрока.
- Проверить error/disconnect/resync границы; не переводить любую ошибку в silent success.
- Приёмка: malformed/truncated payload, stale click, неверный entity/action,
  excessive index; не оставлять locks/menu взятыми и не обходить штатную security architecture.

### W04-120. Воспроизводимое сравнение и передача для ревью

Зависимость: только проверяемые реализованные consumers, не обязательное завершение 001–119.

- Выбрать одинаковые начальные states/seed/actions для Vecta и официального 26.3 сервера.
- Реальный сервер запускать только при явно принятой владельцем EULA;
  отсутствие этого разрешения не обходить и comparison честно отметить невыполненным.
- Приёмка: сравнимые item counts/components, events, packets/state, save/reload;
  каждый mismatch имеет reproduction и отдельный next task, без процента «готовности» по тестам.

## 5. Дополнительная приёмка по семействам

Для каждой механики выбирать **применимые** случаи, не создавать одинаковые
тесты на все пункты без смысла. Expected result брать из конкретной Java ветки.

### Инвентарь и контейнеры

- Пустой input, ровно один item, output > 1, почти полный/полный inventory.
- Одинаковый item с различными components, custom stack limit, damaged tool.
- Pickup, quick-move, swap, throw, clone — только разрешённые конкретному menu пути.
- Изменённый input между preview и take; external hopper изменяет содержимое.
- Возврат тары, drop при overflow, закрытие с предметом на cursor.
- Два игрока, stale state id, разрушение/перенос/выгрузка источника меню.
- Borrowed/owned/live NBT, implicit components, item→block→item round-trip.
- Сумма предметов и компонентов до/после измеряется по всем задействованным местам:
  grid, cursor, player, container, dropped entity, recipe consumption.

### Entities и бой

- Spawn через настоящий item/block/gameplay consumer, не только вручную в тесте.
- Block/entity collision, потеря owner/target, health/damage attribution.
- Entity metadata, equipment, pose и velocity обновляются на правильном канале.
- Removal vs death vs despawn, doEntityDrops, creative и наблюдаемые sounds/events.
- Passengers/owner UUID после reload, chunk граница, dimension transition.
- Probability/range из vanilla; seed там, где результат детерминирован или save-repeatable.
- Owner refs не держат Server/World/Player навсегда; reentrant callbacks не deadlock.

### World ticks, генерация и сохранение

- Отрицательные координаты, минимальная/максимальная допустимая высота,
  chunk boundaries и отсутствие соседнего chunk в нужном состоянии.
- Scheduled action до/после reload, изменение блока до наступления tick.
- Порядок генерации соседей и повторный запрос одного chunk.
- Успех операции отделён от публикации результата worker job; старый job не
  перезаписывает новое состояние и не заставляет tick ждать completion.
- Ошибка save/read/logging не игнорируется; несовпадение эталона не замазывается golden-файлом.

### Протокол и данные

- Поля, порядок, optional/ref/inline формы и limits соответствуют codec 26.3.
- Expected bytes выводятся из правила codec, а не round-trip того же writer.
- Malformed input проверяется по нужной границе ошибки, без общего catch-all.
- Clientbound пакет имеет настоящий consumer; serverbound проверяет текущую игровую фазу.
- Typed registry/refs и extractor provenance; plugin extension path не требует
  писать plugin loader или заявлять ABI-стабильность приватного Rust типа.

## 6. Где брать код, эталон и данные

Vecta-пути относительно `vecta-core/src/`, кроме явных crate prefixes:

| Область | Точки входа |
| --- | --- |
| Result/crafting | `inventory/slots/{slot,result_slot,result_handler,crafting_slots}.rs` |
| Menu consumers | `inventory/menu/kinds/`, `inventory/click.rs`, `inventory/menu/` |
| Recipe foundations | `inventory/recipe_manager.rs`, `vecta-registry/src/recipe/` |
| Packet handlers | `player/networking.rs`, `player/player_inventory/player_handlers.rs` |
| Block pick/placement | `player/game_mode/block_interaction.rs`, `behavior/blocks/`, `behavior/items/` |
| Block entities | `block_entity/mod.rs`, `block_entity/entities/`, `block_entity/registry.rs` |
| Load context | `block_entity/load_context.rs`, `command/execution/block.rs` |
| NBT mutations | `command/builtins/execute/store.rs`, прочие действующие command consumers |
| Containers/locks | `inventory/container/`, `inventory/lock.rs`, `inventory/slots/` |
| Entities/damage | `entity/entity/`, `entity/living_base/`, `entity/damage/`, `entity/entities/` |
| Mob/AI | `entity/mob/`, `entity/ai/`, concrete `entity/entities/mobs/` |
| World/ticks | `world/`, `chunk/`, `server/jobs/` и фактические job consumers |
| Worldgen | `worldgen/`, `vecta-worldgen/src/`, build scripts/assets |
| Packets | `vecta-protocol/src/packets/game/` |
| Codegen | `vecta-core/build/`, `vecta-registry/build/`, `vecta-worldgen/build/` |

Java имена в карточках — **точки входа**, не готовое описание всех методов.
Найти полный путь через `rg --files minecraft-src/minecraft/src/net/minecraft`.
В этой версии пакеты projectile/vehicle/NPC уже могут отличаться от старых версий.
При подготовке, например, вместо старого PotionBrewing найден typed `BrewingRecipe`
и его consumer в BrewingStandBlockEntity: не переносить код из воспоминаний 1.21.
Полные пути проверенных имён: [индекс локального эталона](planning/night-04/reference-index.md).

SteelExtractor checkout существует: `D:\CodexBuilds\SteelExtractor`.
Известная base версия 26.3: `40e72f994d54edd619e3338fe431d7e938bf0db0`.
Output по умолчанию: `run/steel_extractor_output/` внутри checkout.
Для slot displays уже есть `scripts/steel-extractor/README.md` и воспроизводимый patch.
При новом extractor change сохранить provenance, воспроизвести extraction и
скопировать только нужные produced assets без ручного исправления JSON.
Если checkout/output/source реально недоступен — точный blocker по AGENTS,
а не заполняющий пробел hardcoded registry/worldgen/protocol список.

## 7. Сборки на этой машине

Использовать существующие target/temp, **каждый Cargo build/test/check/clippy с `-j 1`**:

```powershell
$env:CARGO_TARGET_DIR = 'D:\CodexBuilds\VectaMC'
$env:CARGO_INCREMENTAL = '0'
$env:TMP = 'D:\CodexBuilds\tmp'
$env:TEMP = 'D:\CodexBuilds\tmp'
```

Проверить наличие путей и место. Изменять только env текущего процесса,
не системные переменные Windows. Логи — `D:\CodexBuilds\logs\`, отдельно на этап.
Проверять exit code cargo после перенаправления/tee, не только exit последнего Get-Content.

Предыдущий OOM больше не текущий блокер: узкие и полные тесты прошли без reboot.
Настройка D: pagefile 16 ГБ сохранена с согласия владельца; при последней проверке
активный файл ещё 1.33 ГБ и commit limit 33.23 ГБ. **Не считать настройки реестра
доказательством реально доступной памяти**. При новом OOM проверить commit/disk,
сохранить log и остановить повторный перебор по AGENTS. Не закрывать приложения,
не убивать все rustc/Java, не перезагружать ПК и не менять profiles/rustflags/toolchain
ради прохождения теста. Старый зависший rustc существовал до сеанса.

Baseline 3890 уже есть. Не пересобирать всё заново до первого изменения без причины.
После законченного изменения — **самый узкий реальный test filter**, затем relevant
consumers при shared-contract change. `0 tests` не считается проверкой.
На границе группы и перед итоговой передачей:

```powershell
cargo fmt --all --check
cargo check --locked --workspace -j 1
cargo test --locked --workspace -j 1
cargo clippy --locked --workspace --all-targets --all-features -j 1 -- -D warnings
python -m unittest discover -s scripts
python scripts/audit_vanilla.py
git diff --cached --check
```

Тяжёлые проверки запускать последовательно. Проверенный код не менять после final
suite без повторной relevant проверки. Если typos/prek доступны, соблюдать repo checks;
если нет — записать пропуск, не ставить pass. Для docs-only Rust suite не нужен:
проверить ссылки, задачи/зависимости, source names и diff.

## 8. Коммиты, сохранение прогресса и передача

- Продолжать существующий actual tip `codex/night-vanilla-03`; не создавать
  одноимённую ветку заново и не merge/force-push/reset/main/upstream.
- Не запускать дополнительных агентов, Gemini/Mistral CLI и не редактировать AGENTS.
- Один commit — один законченный проверяемый slice. Shared foundation с незакрытым
  обязательным consumer не выдавать за feature; неизвестную архитектуру согласовать.
- Stage явные свои paths/hunks. Прочитать staged diff; чужой night-01 report оставить.
- После собственных checks push в origin feature branch, проверить exit code.
- README менять кратко и по реальному gameplay, без «full vanilla», Paper совместимости
  и поддержанных версий, которые не тестировались. CHANGELOG — конкретное before/after.
- Локальные Minecraft sources/JAR, миры, кэши, build logs, Windows backups и secrets не коммитить.

Продолжать журнал в `planning/night-03/`, не создавать пачку пустых документов.
IDs W04 связывать с прежними W03, когда они разворачивают тот же пункт.
При реальном содержимом использовать `progress.md`, `blockers.md`, `scenarios.md`
и новый `report-next.md`; исторические отчёты не подменять новой историей.

Минимальная запись результата:

```text
W04-ID / связь с W03
Игровой trigger и изменение поведения
Проверенные Java методы + Vecta paths + extracted assets при необходимости
Затронутые общие contracts и их реальные consumers
Commit, команды, exit codes, passed/failed/ignored
Save/load/protocol/client/multiplayer границы фактической проверки
Незакрытая часть и ближайшая следующая задача
```

Блокер фиксировать конкретно: что проверено → что отсутствует → почему догадка
изменит vanilla → какой точный path/output/архитектурный ответ нужен → какие IDs ждут.
Не называть отсутствие пары локальных методов «архитектурой нет», если контракт
ясен из текущего кода/source и его можно закончить вместе с consumer.
И наоборот, большой список не даёт разрешения скрывать missing foundation заглушкой.

Перед лимитом сохранить actual git state и точный следующий шаг, не начинать огромную
новую систему. **Финал — обычный текстовый ответ**, а не только файлы:

1. Что теперь действительно делает игрок; сколько карточек закрыто и частично.
2. Branch, start/end SHA, commits/compare, фактический push и dirty paths.
3. Checks с результатами, ограничения среды и реальные непрошедшие проверки.
4. Source/extractor/codegen/dependency/persistence изменения и их границы.
5. Клиент, Linux, Mojang comparison, 2+ сетевых players: сделано или нет.
6. Открытые blockers и ближайший независимый ID для следующего сеанса.

## 9. Начать сейчас

1. Проверить branch/HEAD/status/version/reference и сохранить чужой dirty hash.
2. Прочитать актуальные ревью; **не повторять закрытые W03-01..05 или B-W03-07**.
3. Открыть текущий ResultSlot/CraftingHandler/click consumers и Java ResultSlot/Slot.
4. Выполнить **W04-001 → W04-002** с настоящим count consumer и regression.
5. Узкая проверка → staged diff → commit → origin push → W04-003 и далее.

Не останавливаться после одного пункта или потому, что прошёл час. Продолжать
пока есть ресурс и независимая обоснованная работа; при настоящем общем блокере
следовать AGENTS и не имитировать продвижение изменением статусов.
