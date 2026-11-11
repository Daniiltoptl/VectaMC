# Агент A: игрок, крафт, предметы, бой

> В Arena локальные пути Windows, папки сборки на диске D и локальная сборка не применяются: см. `planning/agents/ARENA.md`.


Сначала прочитай `planning/agents/COMMON.md` (в этой же папке) и `AGENTS.md`.
Папка `C:\Users\My comp\Desktop\Vecta`, ветка `codex/night-vanilla-03`, tip
`b7706815b` (перенос SteelMC уже влит). Журнал `planning/night-06/` (progress, checkpoint, blockers,
route), отчёт `planning/night-06/report-next-12.md`.

Не коммить и не трогать: `planning/agents/AGENT_*.md`, `AGENT_CONTINUE_06_12.md`,
`WORKBUDDY_CONTINUE_06_10.md`, `WORKBUDDY_START_06.md`,
`planning/night-01/report.md` (это чужие правки пользователя).

## Срез 0: незакоммиченная работа (первым)

Прошлый исполнитель оставил незакоммиченную персистенцию recipe book в
`vecta-core/src/player/player_data.rs`, `player_data_storage/mod.rs`,
`player_data_storage/tests.rs` (`PLAYER_STORAGE_VERSION` 10, `RecipeBookFile`,
`PersistentRecipeBook`, `from_book`). При вливании SteelMC в те же файлы
пришло поле `advancements` (сохранение достижений): Claude совместил оба, но
не компилировал. Доведи:
сохранение через `ServerRecipeBook::pack`, загрузка через `load_untrusted` с
валидатором (неизвестные рецепты отбрасываются, как `ServerRecipeBook.loadUntrusted`),
сверка с `ServerPlayer.addAdditionalSaveData/readAdditionalSaveData`. Тест
round-trip + загрузка с неизвестным рецептом. Коммит, push.

## Очередь

Подробности групп — `NIGHT_TASK_06.md` и блоки B, C, D, I, J в
`AGENT_CONTINUE_06_12.md`/`WORKBUDDY_CONTINUE_06_10.md`.

1. Recipe book до конца: G034 place recipe (ghost recipe, `ServerPlaceRecipe`),
   G035 max craft и компоненты, G036 открытие рецептов (`RecipeUnlocked`
   триггеры, подбор предмета), G037 crafting callbacks. Stonecutter и loom
   через те же пути.
2. Станки: G038 наковальня, G039 точило, G040 стол зачарования (сид игрока,
   bookshelf power), затем G018 автокрафтер, G019 зельеварка, G020 автоматизация
   «воронка → станок → воронка».
3. Зачарования data-driven: движок `EnchantmentEffectComponents` и
   потребители (броня, Thorns, Efficiency, Silk Touch, Fortune, Looting,
   Unbreaking, Mending, Infinity, Multishot, Piercing, Quick Charge, Loyalty,
   Riptide, Channeling, Impaling, Density, Breach, Wind Burst, проклятия).
4. Игрок: G061 голод, G062 еда и `consumable`, G063 эффекты, G064 щит,
   G065 ближний бой, G066 урон среды, G067 смерть и respawn, G068 опыт.
   Damage pipeline — твоя зона; агент C (мобы) его не меняет.
5. Предметы: инструменты (мотыга, топор, лопата, ножницы, кисть и
   археология), вёдра (рыба, аксолотль, молоко), трезубец, булава, тотем,
   элитры, компасы, бирка, поводок, седло, отделка брони, яйца спавна,
   снежки, бутылка опыта, книга с пером.
6. Оружие: G041–G045 стрелы, лук, арбалет; G046 фейерверк; G047 fireball;
   G048 wind charge; G049 жемчуг; G050 рыбалка; G003–G005 зелья и
   AreaEffectCloud.
7. G051–G055 картины, стойка, end crystal; G056–G060 двери, кровати, якорь,
   лодки, вагонетки.
