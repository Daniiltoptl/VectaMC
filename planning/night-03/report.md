# Night 03 — отчёт о передаче

Сеанс: 30 октября 2026. Рабочая папка `C:\Users\My comp\Desktop\Vecta`.

## 1. Ветка, коммиты, состояние

| Параметр | Значение |
| --- | --- |
| Ветка | `codex/night-vanilla-03` |
| Стартовый HEAD | `e92ea3715b37d6b51c9e77a6d4e5f0f826f7236b` |
| Итоговый HEAD | `8e58b7a7e` |
| Push | выполнен, `e92ea3715..8e58b7a7e`, exit 0 |

Коммиты этого сеанса:

- `8e58b7a7e` — `feat(recipes): send the client recipe list on login (W03-01 client path)`

Compare: <https://github.com/Daniiltoptl/VectaMC/compare/e92ea3715...8e58b7a7e>

Незакоммиченные пути: `planning/night-01/report.md` — **чужое изменение**, не
трогалось, не стейджилось, не откатывалось. Своих грязных путей нет.

## 2. Статусы задач

| Задача | Статус |
| --- | --- |
| W03-01 | частично: серверный путь был готов раньше; клиентский список рецептов реализован сейчас, проверен на уровне пакета; клиентом не проверялся |
| W03-02 | реализовано и проверено (прошлый сеанс); клиентская зависимость снята — см. § 7 |
| W03-03 | реализовано и проверено (прошлый сеанс) |
| W03-04 | частично: pick-block содержимого блок-сущностей остаётся открытым (B-W03-04) |
| W03-05 | реализовано и проверено (прошлый сеанс) |
| W03-06 … W03-72 | не начато в этом сеансе |

Ревью Codex на W03-01…W03-05 по-прежнему отсутствует; их статусы взяты из
`progress.md` прошлого сеанса, а не подтверждены заново.

## 3. Что сделано в этом сеансе

### W03-01, клиентский путь

Действие игрока до: камнерез открывался, кнопки и результат работали
серверно, но у настоящего клиента сетка рецептов была пуста — список рецептов
никогда не отправлялся.

Действие игрока после: при входе клиент получает полный список рецептов
камнереза и наборы свойств рецептов; экран камнереза фильтрует список по
предмету в слоте и показывает варианты.

Ванильные методы: `ClientboundUpdateRecipesPacket.STREAM_CODEC`,
`SelectableRecipe.noRecipeCodec` / `SingleInputEntry.noRecipeCodec` /
`SingleInputSet.noRecipeCodec`, `Ingredient.CONTENTS_STREAM_CODEC`,
`SlotDisplay.STREAM_CODEC`, `SlotDisplays.bootstrap`, `RecipePropertySet`,
`RecipeManager.finalizeRecipeLoading` / `getSynchronizedItemProperties` /
`getSynchronizedStonecutterRecipes`, `StonecutterRecipe.resultDisplay`,
`PlayerList.placeNewPlayer`.

Пути Vecta:

- `vecta-registry/src/recipe/display.rs` (новый): `SlotDisplay` (11 видов),
  `SingleInputEntry`, `RecipePropertySet`, `RecipePropertySetKey`.
- `vecta-protocol/src/packets/game/c_update_recipes.rs` (новый): `CUpdateRecipes`.
- `vecta-core/src/inventory/recipe_manager.rs`:
  `synchronized_item_properties`, `synchronized_stonecutter_recipes`.
- `vecta-core/src/player/lifecycle/spawn_sync.rs`: отправка при `InitialJoin`.

Общие контракты: новых общих интерфейсов не вводилось. Использован уже
сгенерированный id пакета `C_UPDATE_RECIPES = 136`; правок generated Rust и
extracted JSON нет.

## 4. Проверки

| Команда | Результат |
| --- | --- |
| `cargo test -p vecta-protocol --lib c_update_recipes -j 1` | 4 passed, 0 failed |
| `cargo check -p vecta-core --lib --tests -j 1` | exit 0 |
| `cargo fmt --all --check` | exit 0 |
| `git diff --check` (мои файлы) | чисто; предупреждения только в чужом `planning/night-01/report.md` |
| `cargo test -p vecta-core --lib sync_tests -j 1` | **не выполнено**, см. B-W03-05 |
| `cargo test --locked --workspace -j 1` | **не выполнено** |
| `cargo clippy --locked --workspace …` | **не выполнено** |
| `python -m unittest discover -s scripts` | **не выполнено** |

Тесты пакета проверяют точные байты, выведенные из правил codec вручную:
пустой пакет (два нулевых счётчика), запись набора свойств (ключ, затем id
предметов), запись записи камнереза (именованный holder set → `0` + тег,
вид `item_stack` = 5, шаблон стака), композитный `SlotDisplay`.

Не выполненные проверки — не «pass»: причина в § 7, B-W03-05.

## 5. Codegen, extractor, зависимости, protocol, persistence

- Protocol: добавлен один clientbound пакет. Id взят из существующего
  `vanilla_packets.rs`, ничего не перегенерировалось.
- Extractor: не запускался, extracted JSON не менялся.
- Единственное место, где порядок встроенного реестра записан в Rust, —
  `SlotDisplayKind` (0…10) по `SlotDisplays.bootstrap`. Это не догадка, но и не
  extracted data: вывода `slot_display` в `vecta-registry/build_assets/` нет, а
  checkout SteelExtractor на машине отсутствует.
- Зависимости, toolchain, профили сборки не менялись.
- Persistence: изменений нет, формат сохранений не затронут.

## 6. Чего не проверяли

- Настоящий клиент 26.3 не запускался, EULA не принималась. Сетка камнереза
  у живого клиента не наблюдалась.
- Сравнение с Mojang-сервером не выполнялось.
- Два и более реальных игрока, disconnect/restart, unload чанков — не выполнялись.
- Linux — не выполнялся.
- Юнит-тесты пакета не заменяют клиентскую проверку.

## 7. Ограничения и открытые вопросы

- **B-W03-05 (среда).** Тестовые бинарники `vecta-core` не линкуются: rustc
  падает с `memory allocation of … failed`. Лимит commit ~47.5 ГБ занят на
  ~43–45 ГБ; ~16.5 ГБ держит `qemu-system-x86_64`. Перепробовано: `-Zthreads=2`
  (сборка дошла до 156 крейтов и зависла), `CARGO_PROFILE_TEST_DEBUG=0`,
  повтор после освобождения диска. Нужно освободить память (закрыть VM) или
  принять, что результаты этого сеанса опираются на type-check и suite
  `vecta-protocol`/`vecta-registry`.
- **B-W03-04.** Pick-block не собирает implicit components блок-сущностей;
  нужен хук `collect_implicit_components`/`apply_implicit_components` и его
  потребитель в `handle_pick_item_from_block`. Блокирует полную приёмку W03-04
  и тот же round trip для сундуков/воронок/выбрасывателей.
- **B-W03-01 (остаток).** Книга рецептов (W03-65) требует полного семейства
  `RecipeDisplay` и своих пакетов; сейчас на проводе только те виды, что несёт
  `update_recipes`.
- Список рецептов и наборы свойств пересчитываются на каждый вход, а не
  кэшируются при загрузке, как поля `RecipeManager`. Корректно, но расточительно.
- Наборы `brewing_input`/`brewing_reagent` уходят пустыми: Vecta не загружает
  рецепты зельеварения.
- Исправлено ошибочное утверждение прежнего `blockers.md`: ткацкий станок не
  зависит от пакета рецептов. `LoomMenu.selectablePatterns` строится из реестра
  и тегов `BANNER_PATTERN_REGISTRY`, а теги клиенту уже отправляются через
  `CUpdateTags` (`server/registry_cache.rs::build_tags_packet`).

## 8. Следующий шаг

Ближайший ID: **W03-06** (crafted count, `ITEM_CRAFTED`, item callbacks и
remainders). Основание уже есть: `ResultSlot::on_take` содержит TODO, меню
крафта и `assemble_recipe` работают.

Минимальные команды после освобождения памяти:

```powershell
$env:CARGO_TARGET_DIR = 'D:\CodexBuilds\VectaMC'
$env:CARGO_INCREMENTAL = '0'
$env:TMP = 'D:\CodexBuilds\tmp'
$env:TEMP = 'D:\CodexBuilds\tmp'
cargo test --locked -p vecta-core --lib result_slot -j 1
cargo test --locked --workspace -j 1 --no-fail-fast
```

Источники для W03-06: `world/inventory/ResultSlot.java`,
`world/inventory/CraftingMenu.java`, `ItemStack.onCraftedBy`,
`ResultSlot.checkTakeAchievements`.
