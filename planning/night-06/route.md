# Маршрут пакета 06

Начинать с actual tip и checkpoint. Читать регламент большого ТЗ один раз, затем текущую группу.

G001–G003 выполнены и прошли ревью: [результат](../reviews/night-06-g001-g003-codex.md).
Dispenser consumers G006/G007/G009…G013 прошли
[ревью сеанса 2](../reviews/night-06-g006-g013-codex.md); открытые partial — в blockers.
Основной G021 item-data путь и authorization gate прошли
[ревью](../reviews/night-06-g021-codex.md); region I/O и attached lectern context
теперь реализованы в сеансе05. Live restart/tickets не проверены. B-06-05 снят.
G022 block/storage и общие NBT paths прошли [ревью](../reviews/night-06-g022-codex.md).
Supported G024 living load/save, Team/UUID и break callback реализованы и прошли
[ревью сеанса06](../reviews/night-06-session06-codex.md) с исправлениями codecs,
attributes, effects, сна и references. Brain/locator-bar icon ещё не готовы.
Typed ContextMap, generated parameter sets и миграция текущих loot callers
реализованы в [сеансе07](report-next-07.md). Не повторять этот срез.
Следующий slice: **providers/compute**, затем остальные направления
[запроса07](../../WORKBUDDY_CONTINUE_06_07.md). Пользователь одобрил общий
domain read context с shared Arc scoreboard/storage; он реализован в `70f27a545`
вместе с live Score/Storage/EntityScores (workspace4308/0/18). Actual
каталог извлечён:28float+23int=51тип, NumberDispatcher есть в обеих группах.
Не возвращаться к готовым рамкам вместо открытых shared consumers.
B-06-04 снят: новый apply hook на обычном reload не нужен.
Строки завершённых групп ниже сохраняют исторический порядок,
а не команду повторно реализовать уже завершённые consumers.

Код общей foundation и первый реальный consumer передавать вместе. Номера — обязательства поведения, не число commits.

| Очередь | Группа | Карточки | Направление |
| --- | --- | --- | --- |
| 1 | G001 | W06-001…005 | Пузырёк опыта |
| 2 | G002 | W06-006…010 | Три варианта яиц |
| 3 | G003 | W06-011…015 | Взрывное зелье |
| 4 | G006 | W06-026…030 | Наполненные вёдра |
| 5 | G007 | W06-031…035 | Подбор пустым ведром |
| 6 | G009 | W06-041…045 | Оставшиеся equippable случаи |
| 7 | G010 | W06-046…050 | Огниво и fire charge |
| 8 | G011 | W06-051…055 | Костная мука |
| 9 | G012 | W06-056…060 | Бутылочки |
| 10 | G013 | W06-061…065 | Ножницы |
| 11 | G021 | W06-101…105 | Implicit components round-trip |
| 12 | G022 | W06-106…110 | Живое /data |
| 13 | G024 | W06-116…120 | Постоянное living state |
| 14 | G025 | W06-121…125 | NBT раздатчика |
| 15 | G026 | W06-126…130 | Прочность и equip hooks |
| 16 | G027 | W06-131…135 | Печь: результаты и XP |
| 17 | G028 | W06-136…140 | Печь: автоматизация |
| 18 | G031 | W06-151…155 | RecipeDisplay |
| 19 | G032 | W06-156…160 | ServerRecipeBook |
| 20 | G033 | W06-161…165 | Recipe-book packets |
| 21 | G034 | W06-166…170 | Place recipe |
| 22 | G035 | W06-171…175 | Max craft и компоненты |
| 23 | G036 | W06-176…180 | Открытие рецептов |
| 24 | G037 | W06-181…185 | Crafting callbacks |
| 25 | G005 | W06-021…025 | Облако эффектов |
| 26 | G004 | W06-016…020 | Туманное зелье |
| 27 | G041 | W06-201…205 | AbstractArrow |
| 28 | G044 | W06-216…220 | Лук |
| 29 | G042 | W06-206…210 | Arrow и SpectralArrow |
| 30 | G043 | W06-211…215 | Стрелы с эффектами |
| 31 | G046 | W06-226…230 | Фейерверк |
| 32 | G045 | W06-221…225 | Арбалет |
| 33 | G047 | W06-231…235 | Small fireball |
| 34 | G048 | W06-236…240 | Wind charge |
| 35 | G049 | W06-241…245 | Жемчуг эндера |
| 36 | G050 | W06-246…250 | Рыбалка |
| 37 | G008 | W06-036…040 | Вёдра с существами |
| 38 | G014 | W06-066…070 | Головы и тыквы |
| 39 | G015 | W06-071…075 | Default-dispatch 26.3 |
| 40 | G016 | W06-076…080 | Лодки из раздатчика |
| 41 | G017 | W06-081…085 | Стойка из раздатчика |
| 42 | G018 | W06-086…090 | Автокрафтер |
| 43 | G019 | W06-091…095 | Зельеварка |
| 44 | G020 | W06-096…100 | Цепочка автоматизации |
| 45 | G023 | W06-111…115 | NBT рамок |
| 46 | G029 | W06-141…145 | Украшенный горшок |
| 47 | G030 | W06-146…150 | Shelf 26.3 |
| 48 | G038 | W06-186…190 | Наковальня |
| 49 | G039 | W06-191…195 | Точило |
| 50 | G040 | W06-196…200 | Зачаровательный стол |
| 51 | G051 | W06-251…255 | Картины |
| 52 | G052 | W06-256…260 | Установка стойки |
| 53 | G053 | W06-261…265 | Урон стойке |
| 54 | G054 | W06-266…270 | Cushion |
| 55 | G055 | W06-271…275 | End crystal |
| 56 | G056 | W06-276…280 | Двери и люки |
| 57 | G057 | W06-281…285 | Кровати |
| 58 | G058 | W06-286…290 | Якорь возрождения |
| 59 | G059 | W06-291…295 | Лодки и плоты |
| 60 | G060 | W06-296…300 | Вагонетки и рельсы |
| 61 | G061 | W06-301…305 | Голод |
| 62 | G062 | W06-306…310 | Еда и consumables |
| 63 | G063 | W06-311…315 | Эффекты |
| 64 | G064 | W06-316…320 | Щит |
| 65 | G065 | W06-321…325 | Ближний бой |
| 66 | G066 | W06-326…330 | Урон среды |
| 67 | G067 | W06-331…335 | Смерть и respawn |
| 68 | G068 | W06-336…340 | Опыт и mending |
| 69 | G069 | W06-341…345 | Natural spawning |
| 70 | G070 | W06-346…350 | Despawn и persistence |
| 71 | G071 | W06-351…355 | Зомби |
| 72 | G072 | W06-356…360 | Скелет |
| 73 | G073 | W06-361…365 | Крипер |
| 74 | G074 | W06-366…370 | Паук |
| 75 | G075 | W06-371…375 | Эндермен |
| 76 | G076 | W06-376…380 | Жители |
| 77 | G077 | W06-381…385 | Големы |
| 78 | G078 | W06-386…390 | Дракон и End fight |
| 79 | G079 | W06-391…395 | Водные существа |
| 80 | G080 | W06-396…400 | Приручаемые существа |
| 81 | G081 | W06-401…405 | Хранилище карт |
| 82 | G082 | W06-406…410 | Карты в руках и рамках |
| 83 | G083 | W06-411…415 | Картографический стол |
| 84 | G084 | W06-416…420 | Знамёна |
| 85 | G085 | W06-421…425 | Sculk |
| 86 | G086 | W06-426…430 | Teams и friendly fire |
| 87 | G087 | W06-431…435 | Advancements |
| 88 | G088 | W06-436…440 | Statistics |
| 89 | G089 | W06-441…445 | /loot, /item, /random |
| 90 | G090 | W06-446…450 | Gamerules |
| 91 | G091 | W06-451…455 | Расширенный Java golden взрывов |
| 92 | G092 | W06-456…460 | Собственный worldgen parity прогон |
| 93 | G093 | W06-461…465 | Биомы, noise, heightmaps |
| 94 | G094 | W06-466…470 | Структуры |
| 95 | G095 | W06-471…475 | Порталы и border |
| 96 | G096 | W06-476…480 | Chunk/entity lifecycle |
| 97 | G097 | W06-481…485 | Измеренная производительность |
| 98 | G099 | W06-491…495 | Полный Java golden лута |
| 99 | G101 | W06-501…505 | Редстоуновая пыль |
| 100 | G102 | W06-506…510 | Повторители и компараторы |
| 101 | G103 | W06-511…515 | Поршни |
| 102 | G104 | W06-516…520 | Рельсы и сигналы |
| 103 | G105 | W06-521…525 | Жидкости |
| 104 | G106 | W06-526…530 | Огонь, костры и свечи |
| 105 | G107 | W06-531…535 | Рост растений |
| 106 | G108 | W06-536…540 | Почва и компостер |
| 107 | G109 | W06-541…545 | Навигация и поиск пути |
| 108 | G110 | W06-546…550 | Разведение и outputs животных |
| 109 | G111 | W06-551…555 | Спавнеры, TrialSpawner и Vault |
| 110 | G112 | W06-556…560 | Рейды |
| 111 | G113 | W06-561…565 | Piglin и Hoglin |
| 112 | G114 | W06-566…570 | Пчёлы и ульи |
| 113 | G115 | W06-571…575 | Wither и Warden |
| 114 | G116 | W06-576…580 | Подводные блоки |
| 115 | G117 | W06-581…585 | Погода и молния |
| 116 | G118 | W06-586…590 | Сетевой lifecycle игрока |
| 117 | G119 | W06-591…595 | Правила измерений |
| 118 | G120 | W06-596…600 | Beacon и Conduit |
| 119 | G098 | W06-486…490 | Flint отдельным tooling slice |
| 120 | G100 | W06-496…500 | Длинный survival сценарий |

Не ждать блокированную группу, когда есть независимый slice. Не объявлять неизвестный контракт понятным из-за места в очереди.

## Обновление сеанса 8 (2026-11-04)

Приоритет 6 (§B файла 06_07) выполнен частично и проверенно:

- typed `FloatProvider`/`IntProvider` (27 + 22 типов) в
  `vecta-registry/src/loot_table/number/` с registry dispatch, configured
  values 4 float + 26 int из extractor output и тремя код-предикатами Java;
- потребитель: печь читает `minecraft:cooking_fuel` и вычисляет providers против
  block state (G027 частично).

Следующий маршрут: **`/data modify … compute`** (protocol ids 55/56, два
argument parser'а, три context source-ветки, проводка в существующий
manipulator), затем компостер (`compostable/*`), затем приоритеты 9–12 файла 05.
Точные границы — B-06-11.

## Обновление 2 сеанса 8 (2026-11-04)

Приоритет C (§C файла 06_07) выполнен: `/data modify … compute` работает
сквозным путём с `default`/`block`/`entity` context sources и настоящими
provider argument'ами (protocol 55/56). Проверено командными тестами на живом
сервере fixture: constants, configured resource, вложенная арифметика, score
provider, негативные пути без мутации.

Следующий маршрут: **компостер** (`compostable/*` — вертикальный срез
interaction/уровни/bone meal/dispenser-hopper), затем приоритеты 9–12 файла 05.

## Обновление сеанса 9 (2026-11-07)

Компостер закрыт вертикальным срезом: `WorldlyContainerHolder` — общий
фундамент (`container_at` у воронки спрашивает держателя до блок-энтити;
дроппер идёт через тот же путь), компостер — его первый consumer с
виртуальными Input/Output/Empty контейнерами без блок-энтити. Шанс компоста —
configured providers `compostable/*`. Workspace 4341/0/18.

Следующий маршрут: **Farmland (W06-536…538)** из той же группы G108, затем
хвосты ревью сеанса 8 (контекст печи без BLOCK_ENTITY/CONTAINER, inline-форма
ResolvableInt/ResolvableFloat в CookingFuel, `block_entity_loot_ref` в
compute) и далее раздел D файла 06_07 с пункта 29.

## Обновление сеанса 10 (2026-11-07)

Farmland закрыт (W06-536…538, W06-540): влага/дождь/высыхание, trampling
через level random, canSurvive/updateShape/tick, placement→dirt, тег
MAINTAINS_FARMLAND; мотыга — data-driven transformer, закреплена тестами,
не дублирована; рост crop переведён на level random
(`nextInt((int)(25/speed)+1)==0`). Workspace 4357/0/18.

Следующий маршрут: хвосты ревью при первом consumer (inline
ResolvableInt/ResolvableFloat у Compostable/CookingFuel, контекст печи без
BLOCK_ENTITY/CONTAINER, `block_entity_loot_ref`, B-06-11), bonemeal на
level random как самостоятельный маленький срез, затем раздел D файла 06_07
с пункта 29.

## Обновление сеанса 11 (2026-11-09)

Блок A закрыт: A1 Farmland принят (`bd82a24d9`, trampling-объём в f32),
A2 ResolvableInt/ResolvableFloat inline-формы (Compostable/CookingFuel/
BrewingFuel; Constant не тратит level random), A3 контекст печи получил
BLOCK_ENTITY, /data compute block_entity_loot_ref несёт имя и инвентарь.
CONTAINER (SlotProvider) — блокер до typed value family.

Следующий маршрут: блок B, пункт B1 — G031→G037 RecipeDisplay →
ServerRecipeBook → recipe-book packets → place recipe (G034) → max craft и
компоненты (G035) → открытие рецептов (G036) → crafting callbacks (G037),
реальный путь клиента. По мере потребителей — A4 (B-06-11).

G031 закрыт (коммит `e26c19aa9`). Следующий маршрут: G032 — связка
RecipeManager ↔ RecipeDisplay (построение RecipeDisplayEntry из
реестра рецептов: id/display/group/category/craftingRequirements),
затем ServerRecipeBook (G033) и recipe-book packets.

W06-153 закрыт (`7bab2d0ba`). G031: W06-151/152/153/155 закрыты; W06-154
(пакетный wire consumer) — частично, ждёт G033 (recipe-book packets).
Следующий маршрут: G032 ServerRecipeBook (known/highlight/settings/
persistence на игроке), затем G033 packets.

G032 закрыт (`63f06b151`). Следующий маршрут: G033 — recipe-book packets
(ClientboundRecipeBookAdd/Remove/Settings, sendInitialRecipeBook,
serverbound settings handlers) — закрывает W06-154 (пакетный wire
consumer). Затем G034 place recipe → G035 max craft → G036 discovery →
G037 crafting callbacks.
