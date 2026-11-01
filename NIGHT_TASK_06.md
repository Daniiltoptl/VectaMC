# VectaMC — расширенное ТЗ 06: 600 карточек

Это разрешение реализовывать и проверять код по очереди, не запрос составить ещё один план.
Здесь 120 направлений и 600 отдельных карточек: пять проверяемых частей каждого направления.
Первые 100 направлений раскрывают пакет 05, ещё 20 добавляют важные systems ваниллы.
Это очередь нескольких сеансов, не обещание уложиться за ночь и не список 600 доказанных багов.
Наличие карточки не означает, что функциональности нет: сначала проверить actual code и reviews.
Готовый путь не переписывать; закрывать конкретным доказательством, без tests констант ради счётчика.

## 1. Исходная точка

- Рабочий каталог `C:\Users\My comp\Desktop\Vecta`, repo `Daniiltoptl/VectaMC`.
- Продолжать actual tip `codex/night-vanilla-03`; не reset к историческим SHA.
- Текущий игровой import `3a1fd4815`, docs пакета 05 `7fb7c8bf0`.
- Последний проверенный Rust baseline: **4185 passed / 0 failed / 18 ignored**.
  Workspace Clippy all-targets/all-features -D warnings и fmt прошли. Это прежний
  результат игрового кода, а не новый прогон от создания этого документа.
- Target `Cargo.toml` — `0.16.1+mc26.3`; manifest 26.3, 5037 Java sources,
  SHA1 `33680f5f2ac32864d6d7cf5e56a705fdb3e05f4c`.
- Сейчас без plugins и multi-version. Не превращать задачу в Paper/Bukkit API или поддержку 1.21.
- Сначала прочитать AGENTS.md, checkpoint и reviews, перечисленные в стартовом запросе.
- Старые пакеты 04/05 — источники history/acceptance, не актуальные команды начать всё заново.

## 2. Что уже существует

Сохранить рамки обоих видов и pick, мешки, crafting/remainders/stats, рабочие меню
stonecutter/loom/lectern/smithing и initial stonecutter recipes. В crafting result ПКМ/Q
забирают весь результат; crafted count использует ванильный requested/min contract.
Сохранить implicit BE components, lock/name/lazy loot, chest variants/viewers,
hopper/sided transfers/dropper, menu closing/sync, snowball и generic equippable dispenser.
Потоковый loot evaluation и stack splitter исправлены, но все loot functions не объявлены готовыми.
Named RNG один на Server, default-world seed, persistent state через SavedDataManager TOML.
Владелец выбрал последовательные игровые тики миров и стабильный WorldMap insertion order.
Не возвращать parallel gameplay worlds или per-world named RNG banks.
Перенесены SteelMC TNT/explosions/Mth/hurt/splash/tab-list; их не реализовывать повторно.
Живой capability уже есть как `as_living_entity()`, game events наблюдаемы через listeners.
Кафедра не automation container, equip callbacks идут вне locks, pick components заменяются.
TNT engine не означает готовность creeper/crystal/wind-charge/bed/anchor и team-friendly-fire.
Upstream golden/benchmarks не считать выполненными Vecta прогонами.

## 3. Как читать большое ТЗ

Один раз прочитать этот регламент и `planning/night-06/route.md`.
Затем читать только текущую группу Gxxx, её пять карточек, Java sources и реальные callers.
Не перечитывать все 600 карточек на каждом turn и не тратить контекст на полный dump.
Искать `rg -n 'W06-001|G001' NIGHT_TASK_06.md`, читать нужный участок.
Проверенный source index — `planning/night-06/reference-index.json`.
Java paths ниже относительны `minecraft-src/minecraft/src/net/minecraft/`.
Source path существовал при подготовке; это навигация, не заявление о чтении каждого метода.
Перед реализацией читать методы, callers, codec и используемые extracted assets.

Приоритет: первые готовые dispenser consumers → components/NBT/menu gaps → book/recipes
и projectile foundations → остальной survival, mobs и cross-system acceptance.
Большая группа может выполняться несколькими вертикальными slices; номер не требует
закрыть всю группу прежде, чем взять независимую карточку другой группы.
Указаны подтверждённые зависимости; это не замена проверке actual foundations.
Если первый общий helper требует двух consumers, сделать проверяемый совместный slice.
Shared foundation не коммитить частично до первого реального consumer.

## 4. Обязательный результат карточки

1. Записать конкретный исходный сценарий и проверить, не закрыт ли он текущим кодом.
2. Прочитать Vecta caller, соответствующую Java и registry/extractor inputs.
3. Довести action/packet/use/tick → dispatch → state → sync → persistence, где это предусмотрено source.
4. Проверить основной результат и существенный отказ/повтор/границу, выбранные по контракту.
5. Сопоставить количество/компоненты/урон/RNG order/owner identity, а не только наличие entity.
6. Проверить реальные outgoing packets/events там, где это важно для поведения клиента.
7. Оставить reviewable diff, узкие meaningful tests, actual exit/count и commit/push feature branch.

Preview, registration, codec, NBT-only entity, stat counter и пустой hook сами по себе не закрывают механику.
DoD группы: все заявленные consumers функциональны либо оставшиеся parts явно помечены partial/blocked.
Не закрывать родителя по одному успешному подслучаю. Не закрывать всю vanilla по числу tests/markers.
Карточка «уже работает» закрывается проверкой non-obvious сценария; тест прямой константы не нужен.

## 5. Контракты, которые нельзя угадывать

AGENTS.md имеет приоритет: ASK/DON'T GUESS, VERIFY BEFORE ASKING и CHECK-IN DISCIPLINE.
Сначала Vecta/Java/extracted data/tests, затем узкий вопрос о реально недостающем факте.
Отсутствие готового Rust метода не блокер, если ясный contract можно закончить с consumer.
Неясная architecture/ownership/context — стоп для зависимой работы, независимую продолжить.
Молчание не согласие. Текст этого ТЗ не разрешает новый World→Server reference или новую scope scoreboard.
Особые gates: G086 runtime scoreboard/team access, любые новые async/threading решения,
официальный Minecraft server run/EULA и недоступные produced extractor outputs.
Формат blocker: что проверено → чего не хватает → риск divergence → точный path/output/decision.
После повторных ошибок одной категории не перебирать flags/обходы бесконечно.

Нельзя: speculative fallback, fake data, no-op foundation, direct generated Rust edits,
ручные protocol/registry IDs, отключение проверок, production unwrap/expect и borrow-checker clones.
Keyed DowncastType с уникальным owner key/Safety разрешён AGENTS, другой unsafe требует решения.
Capabilities — для общих интерфейсов, concrete keyed downcast — для точного Vecta-owned типа.
Seeded vanilla RNG обязателен для worldgen/loot/repeatable state; live randomness — по AGENTS.
Probability/range не заменять hardcoded средним результатом.
Не делать runtime datapack JSON loader вместо typed generated vanilla data.
Не расширять реестр ради алгоритма, который Java выражает фиксированным transform.
Client-local addParticle/playLocalSound не рассылать как новые server particles без основания.

## 6. Extractor и эталоны

SteelExtractor: `D:\CodexBuilds\SteelExtractor`; verify checkout/version и output,
обычно `run/steel_extractor_output/`. Workflow: `scripts/steel-extractor/README.md`.
Нужный extractor обновить/запустить; копировать только produced files с provenance.
Не считать checkout старой версии подходящим новому полю и не дописывать JSON вручную.
Java golden: version/seed/coordinates/inputs/command/raw output; Rust self-consistency недостаточна.
Нельзя принимать EULA, скачивать/запускать официальный server с её автоматическим принятием.
Доступный готовый fixture можно replay без заявления о новом Java run.

## 7. Инвентари и совместная работа

Server state авторитетен, client changed-slots не заменяют inventory. Проверять container/state IDs.
Amount+components должны сохраняться через hand/slot/cursor/world drop, включая полный inventory.
Старый viewer после break/close не может повторно выдать preview. Два viewers видят общий storage.
Remainders, dirty/comparator updates и callbacks выполняются в правильном порядке без lock reentry deadlock.
Save/load проверяет настоящий storage boundary, а не копирование Rust struct в тесте.
Owner references не держат умершую/отключённую entity бесконечно; replacement не сохраняет старую incarnation.

## 8. Сборка и проверки

Каждая тяжёлая Cargo-команда **-j 1**, `CARGO_INCREMENTAL=0`.
`CARGO_TARGET_DIR=D:\CodexBuilds\VectaMC`, `TMP=TEMP=D:\CodexBuilds\tmp`.
Logs: `D:\CodexBuilds\logs`, сохранять actual exit code, не только последний Get-Content.
Перед сборкой проверить активные processes и свободное место. Не запускать несколько Cargo builds одновременно.
Проверенный baseline без причины не пересобирать до первого изменения.
После slice — узкий relevant test; 0 выбранных tests не pass. Shared changes проверяют consumers.
Перед передачей группы — fmt, locked workspace tests и workspace Clippy all-targets/all-features -D warnings.
Python unittest/audit после relevant tooling/registry changes и при финальной передаче пакета.
Не полный suite после каждой строки; повтор оправдан новым изменением, failure или нерешённым concern.
Не менять toolchain/deps/flags/profiles для обхода сбоя, не закрывать VM, не убивать все rustc/Java,
не менять ОС, pagefile и не перезагружать ПК. Исторический OOM не считать текущим состоянием.
Game ticks не ждут disk/network async и не poll/sleep; использовать существующие jobs.

## 9. Git, журнал и окончание сеанса

Foreign `planning/night-01/report.md` не трогать, не stage, не stash/reset.
SHA256 `06680CBC971245B70C990D95C906F0F1E3685EFFDE61C13D562664B4B6B05222`.
Не менять AGENTS, не запускать дополнительных агентов/Gemini/Mistral CLI.
Stage только свои paths/hunks. Только текущая feature branch: без main/merge/force push/reset/clean.
Сохранять attribution/licence, README/CHANGELOG обновлять краткими реальными результатами.
Журнал: `planning/night-06/progress.md`, `blockers.md`, итоговый report.md.
На каждую карточку: ID/старый W05 ID, before/after, source methods, commands/count/exit,
commit/push, ограничения и следующий конкретный consumer. Сотни пустых report files не создавать.
Перед лимитом: actual HEAD/status, dirty paths, unfinished diff, активная сборка/session/log, next ID.
Непроверенную общую foundation не коммитить ради лимита. Закрытую проверяемую часть запушить.
В конце ОБЯЗАТЕЛЬНО обычный текстовый ответ, не только файлы: сделано/partial/tests/branch/push/report/next.
Client/Linux/Java run/два сетевых игрока — отдельно перечислить реально выполненное и невыполненное.

## 10. Общая матрица приёмки

| Вид | Обязательные измерения по применимости |
| --- | --- |
| Item use | обе руки, survival/creative/Adventure, empty/full inventory, success/failure |
| Container | cursor/slot/hotbar/shift/throw, old state ID, два viewers, close/break/rejoin |
| Entity | owner/no owner, loaded/unloaded chunk, death/removal/recreation, NBT/UUID |
| Projectile | block/entity hit, repeated callback, insertion rejection, components, damage attribution |
| Block | placement/support/neighbors/redstone/comparator, drop/gamerules, block-entity lifecycle |
| Determinism | same input/seed, next RNG draw, sequence save/reload, order между мирами |
| AI | реальные goals/navigation/attack, obstacle, target loss, species rules, persistence |
| Protocol | generated ID, source wire order, expected bytes, rejected client input |
| Runtime | freeze/step/sprint по source, active world boundaries, отсутствие tick I/O waits |

Матрица не требование бессмысленного декартова произведения для каждого helper.
Выбирать meaningful cases, способные поймать правдоподобную регрессию; неприменимое отметить.

## 11. Каталог направлений

- G001: Пузырёк опыта — W06-001…W06-005.
- G002: Три варианта яиц — W06-006…W06-010.
- G003: Взрывное зелье — W06-011…W06-015.
- G004: Туманное зелье — W06-016…W06-020.
- G005: Облако эффектов — W06-021…W06-025.
- G006: Наполненные вёдра — W06-026…W06-030.
- G007: Подбор пустым ведром — W06-031…W06-035.
- G008: Вёдра с существами — W06-036…W06-040.
- G009: Оставшиеся equippable случаи — W06-041…W06-045.
- G010: Огниво и fire charge — W06-046…W06-050.
- G011: Костная мука — W06-051…W06-055.
- G012: Бутылочки — W06-056…W06-060.
- G013: Ножницы — W06-061…W06-065.
- G014: Головы и тыквы — W06-066…W06-070.
- G015: Default-dispatch 26.3 — W06-071…W06-075.
- G016: Лодки из раздатчика — W06-076…W06-080.
- G017: Стойка из раздатчика — W06-081…W06-085.
- G018: Автокрафтер — W06-086…W06-090.
- G019: Зельеварка — W06-091…W06-095.
- G020: Цепочка автоматизации — W06-096…W06-100.
- G021: Implicit components round-trip — W06-101…W06-105.
- G022: Живое /data — W06-106…W06-110.
- G023: NBT рамок — W06-111…W06-115.
- G024: Постоянное living state — W06-116…W06-120.
- G025: NBT раздатчика — W06-121…W06-125.
- G026: Прочность и equip hooks — W06-126…W06-130.
- G027: Печь: результаты и XP — W06-131…W06-135.
- G028: Печь: автоматизация — W06-136…W06-140.
- G029: Украшенный горшок — W06-141…W06-145.
- G030: Shelf 26.3 — W06-146…W06-150.
- G031: RecipeDisplay — W06-151…W06-155.
- G032: ServerRecipeBook — W06-156…W06-160.
- G033: Recipe-book packets — W06-161…W06-165.
- G034: Place recipe — W06-166…W06-170.
- G035: Max craft и компоненты — W06-171…W06-175.
- G036: Открытие рецептов — W06-176…W06-180.
- G037: Crafting callbacks — W06-181…W06-185.
- G038: Наковальня — W06-186…W06-190.
- G039: Точило — W06-191…W06-195.
- G040: Зачаровательный стол — W06-196…W06-200.
- G041: AbstractArrow — W06-201…W06-205.
- G042: Arrow и SpectralArrow — W06-206…W06-210.
- G043: Стрелы с эффектами — W06-211…W06-215.
- G044: Лук — W06-216…W06-220.
- G045: Арбалет — W06-221…W06-225.
- G046: Фейерверк — W06-226…W06-230.
- G047: Small fireball — W06-231…W06-235.
- G048: Wind charge — W06-236…W06-240.
- G049: Жемчуг эндера — W06-241…W06-245.
- G050: Рыбалка — W06-246…W06-250.
- G051: Картины — W06-251…W06-255.
- G052: Установка стойки — W06-256…W06-260.
- G053: Урон стойке — W06-261…W06-265.
- G054: Cushion — W06-266…W06-270.
- G055: End crystal — W06-271…W06-275.
- G056: Двери и люки — W06-276…W06-280.
- G057: Кровати — W06-281…W06-285.
- G058: Якорь возрождения — W06-286…W06-290.
- G059: Лодки и плоты — W06-291…W06-295.
- G060: Вагонетки и рельсы — W06-296…W06-300.
- G061: Голод — W06-301…W06-305.
- G062: Еда и consumables — W06-306…W06-310.
- G063: Эффекты — W06-311…W06-315.
- G064: Щит — W06-316…W06-320.
- G065: Ближний бой — W06-321…W06-325.
- G066: Урон среды — W06-326…W06-330.
- G067: Смерть и respawn — W06-331…W06-335.
- G068: Опыт и mending — W06-336…W06-340.
- G069: Natural spawning — W06-341…W06-345.
- G070: Despawn и persistence — W06-346…W06-350.
- G071: Зомби — W06-351…W06-355.
- G072: Скелет — W06-356…W06-360.
- G073: Крипер — W06-361…W06-365.
- G074: Паук — W06-366…W06-370.
- G075: Эндермен — W06-371…W06-375.
- G076: Жители — W06-376…W06-380.
- G077: Големы — W06-381…W06-385.
- G078: Дракон и End fight — W06-386…W06-390.
- G079: Водные существа — W06-391…W06-395.
- G080: Приручаемые существа — W06-396…W06-400.
- G081: Хранилище карт — W06-401…W06-405.
- G082: Карты в руках и рамках — W06-406…W06-410.
- G083: Картографический стол — W06-411…W06-415.
- G084: Знамёна — W06-416…W06-420.
- G085: Sculk — W06-421…W06-425.
- G086: Teams и friendly fire — W06-426…W06-430.
- G087: Advancements — W06-431…W06-435.
- G088: Statistics — W06-436…W06-440.
- G089: /loot, /item, /random — W06-441…W06-445.
- G090: Gamerules — W06-446…W06-450.
- G091: Расширенный Java golden взрывов — W06-451…W06-455.
- G092: Собственный worldgen parity прогон — W06-456…W06-460.
- G093: Биомы, noise, heightmaps — W06-461…W06-465.
- G094: Структуры — W06-466…W06-470.
- G095: Порталы и border — W06-471…W06-475.
- G096: Chunk/entity lifecycle — W06-476…W06-480.
- G097: Измеренная производительность — W06-481…W06-485.
- G098: Flint отдельным tooling slice — W06-486…W06-490.
- G099: Полный Java golden лута — W06-491…W06-495.
- G100: Длинный survival сценарий — W06-496…W06-500.
- G101: Редстоуновая пыль — W06-501…W06-505.
- G102: Повторители и компараторы — W06-506…W06-510.
- G103: Поршни — W06-511…W06-515.
- G104: Рельсы и сигналы — W06-516…W06-520.
- G105: Жидкости — W06-521…W06-525.
- G106: Огонь, костры и свечи — W06-526…W06-530.
- G107: Рост растений — W06-531…W06-535.
- G108: Почва и компостер — W06-536…W06-540.
- G109: Навигация и поиск пути — W06-541…W06-545.
- G110: Разведение и outputs животных — W06-546…W06-550.
- G111: Спавнеры, TrialSpawner и Vault — W06-551…W06-555.
- G112: Рейды — W06-556…W06-560.
- G113: Piglin и Hoglin — W06-561…W06-565.
- G114: Пчёлы и ульи — W06-566…W06-570.
- G115: Wither и Warden — W06-571…W06-575.
- G116: Подводные блоки — W06-576…W06-580.
- G117: Погода и молния — W06-581…W06-585.
- G118: Сетевой lifecycle игрока — W06-586…W06-590.
- G119: Правила измерений — W06-591…W06-595.
- G120: Beacon и Conduit — W06-596…W06-600.

## 12. Подробные карточки

## G001. Пузырёк опыта

**Приоритет:** P0. **Карточки:** W06-001…W06-005.

**Связь:** W05-001.

**Сквозной объём направления:**

Эталон: ExperienceBottleItem, ThrownExperienceBottle. Связь: W04-032.
Подключить готовую сущность с её config, копией стака и spawn hooks. Импульс создаёт
летящий пузырёк, расходует один предмет, даёт верные events; rejected insertion не отменяет расход.

**Проверенные Java-точки входа:** `world/item/ExperienceBottleItem.java`; `world/entity/projectile/throwableitemprojectile/ThrownExperienceBottle.java`

**Базовые зависимости:** нет заранее подтверждённой межгрупповой зависимости; проверить actual consumers. Нумерация не разрешает fake foundation.

### W06-001. Запуск пузырька

**Сделать и доказать:** Считать position, power и uncertainty из Java config; импульс создаёт настоящий летящий ThrownExperienceBottle.

### W06-002. Компоненты снаряда

**Сделать и доказать:** Передать одно содержимое стака; custom components не пропадают и исходный stack теряет ровно одну единицу.

### W06-003. Разбивание и опыт

**Сделать и доказать:** Попадание по блоку и entity запускает реальное разбивание и XP через существующий consumer, без второго пути выдачи.

### W06-004. События и отказ insertion

**Сделать и доказать:** Правильные level events сохраняются при успешном запуске и rejected insertion; возврата потраченного предмета нет.

### W06-005. Сохранение полёта

**Сделать и доказать:** Save/load летящего пузырька сохраняет нужное состояние; последующее попадание выдаёт опыт один раз.

**Закрытие группы:** отчётить каждый из пяти результатов отдельно; partial/blocked не превращать в completed. Реально готовые ранее parts подтвердить, не переписывать.

## G002. Три варианта яиц

**Приоритет:** P0. **Карточки:** W06-006…W06-010.

**Связь:** W05-002.

**Сквозной объём направления:**

Эталон: EggItem, ThrownEgg. Связь: W04-032.
Проверить EGG/BLUE_EGG/BROWN_EGG по Java 26.3, вариант и настоящее вылупление.
Приёмка: обе руки при use, dispenser, попадание, расход и NBT; один alias без variant behavior не готов.

**Проверенные Java-точки входа:** `world/item/EggItem.java`; `world/entity/projectile/throwableitemprojectile/ThrownEgg.java`

**Базовые зависимости:** нет заранее подтверждённой межгрупповой зависимости; проверить actual consumers. Нумерация не разрешает fake foundation.

### W06-006. Общий launch яиц

**Сделать и доказать:** Зарегистрировать три item-варианта через общий consumer; позиция и velocity соответствуют Java dispenser config.

### W06-007. Передача варианта

**Сделать и доказать:** Цвет яйца и данные будущего существа проходят ItemStack→projectile→hit без подмены default variant.

### W06-008. Вылупление

**Сделать и доказать:** Удар выполняет реальный spawn и age/variant initialization; probabilities и число потомков сверены с Java.

### W06-009. Ручное использование

**Сделать и доказать:** Обе руки, creative/survival, cooldown и расход используют тот же предметный контракт, не копию dispenser кода.

### W06-010. NBT и повторное попадание

**Сделать и доказать:** Reload не меняет вариант; удалённый projectile не порождает второго потомка при повторном callback.

**Закрытие группы:** отчётить каждый из пяти результатов отдельно; partial/blocked не превращать в completed. Реально готовые ранее parts подтвердить, не переписывать.

## G003. Взрывное зелье

**Приоритет:** P0. **Карточки:** W06-011…W06-015.

**Связь:** W05-003.

**Сквозной объём направления:**

Эталон: SplashPotionItem, ThrownSplashPotion. Связь: W04-032.
Замкнуть launch/hit с potion components. Проверить две цели на разных расстояниях,
duration/instant effects, отсутствие owner у раздатчика и расход ровно одного стака unit.

**Проверенные Java-точки входа:** `world/item/SplashPotionItem.java`; `world/entity/projectile/throwableitemprojectile/ThrownSplashPotion.java`

**Базовые зависимости:** нет заранее подтверждённой межгрупповой зависимости; проверить actual consumers. Нумерация не разрешает fake foundation.

### W06-011. Dispenser launch зелья

**Сделать и доказать:** Запускать ThrownSplashPotion с правильной config и без придуманного владельца.

### W06-012. Potion contents

**Сделать и доказать:** Сохранить base potion, custom effects и разрешённые components во всём пути полёта.

### W06-013. Прямое попадание

**Сделать и доказать:** Instant и timed effects на прямой цели проходят живой effect consumer с правильной attribution.

### W06-014. Радиус воздействия

**Сделать и доказать:** Две цели на разных расстояниях получают ожидаемую интенсивность/длительность; неподходящие сущности отфильтрованы.

### W06-015. Распад и синхронизация

**Сделать и доказать:** При ударе отправляется правильный event, расход один; save/load и повторный hit не дублируют effects.

**Закрытие группы:** отчётить каждый из пяти результатов отдельно; partial/blocked не превращать в completed. Реально готовые ранее parts подтвердить, не переписывать.

## G004. Туманное зелье

**Приоритет:** P1. **Карточки:** W06-016…W06-020.

**Связь:** W05-004.

**Сквозной объём направления:**

Эталон: LingeringPotionItem, ThrownLingeringPotion. Зависимость: 005.
Попадание создаёт настоящее работающее облако с компонентами. Проверить повторное
наложение и owner attribution; визуальный explosion или NBT-only cloud недостаточны.

**Проверенные Java-точки входа:** `world/item/LingeringPotionItem.java`; `world/entity/projectile/throwableitemprojectile/ThrownLingeringPotion.java`

**Базовые зависимости:** G005. Нумерация не разрешает fake foundation.

### W06-016. Туманный projectile

**Сделать и доказать:** Связать LingeringPotionItem с настоящим projectile; пустое облако не заменяет отсутствующую механику.

### W06-017. Создание облака

**Сделать и доказать:** Hit создаёт AreaEffectCloud в правильной позиции с размерами и длительностью из Java.

### W06-018. Перенос эффектов

**Сделать и доказать:** Potion contents и owner переходят в cloud без потери custom effects и неправильного уменьшения длительности.

### W06-019. Dispenser route

**Сделать и доказать:** Раздатчик использует ту же цепочку создания облака; launch, расход и события соответствуют варианту предмета.

### W06-020. Завершение цепочки

**Сделать и доказать:** Projectile исчезает, cloud продолжает работать после reload; нет повторного создания облака при старом callback.

**Закрытие группы:** отчётить каждый из пяти результатов отдельно; partial/blocked не превращать в completed. Реально готовые ранее parts подтвердить, не переписывать.

## G005. Облако эффектов

**Приоритет:** P1. **Карточки:** W06-021…W06-025.

**Связь:** W05-005.

**Сквозной объём направления:**

Эталон: AreaEffectCloud. Связь: W04-064.
Реализовать wait/duration/radius/reapplication и save/load. Две цели входят и выходят:
effects не накладываются каждый tick вместо cooldown, истёкшее облако действительно удаляется.

**Проверенные Java-точки входа:** `world/entity/AreaEffectCloud.java`

**Базовые зависимости:** нет заранее подтверждённой межгрупповой зависимости; проверить actual consumers. Нумерация не разрешает fake foundation.

### W06-021. Жизненный цикл облака

**Сделать и доказать:** Реализовать wait time, duration и удаление; истёкший cloud не остаётся невидимым активным источником.

### W06-022. Радиус и размеры

**Сделать и доказать:** Изменение радиуса обновляет bounding box и синхронизируемые данные, включая уменьшение после успешного применения.

### W06-023. Отбор целей

**Сделать и доказать:** Две живые цели, мёртвая entity и неподходящий тип проходят ванильный predicate, не общий AABB-only фильтр.

### W06-024. Повторное применение

**Сделать и доказать:** Reapplication delay хранится на цель; выход/вход и несколько ticks не дают бесконечного наложения каждый tick.

### W06-025. Persistent state

**Сделать и доказать:** Owner, effects, radius и timers сохраняются; reload не обнуляет duration и не удерживает удалённых owners сильной ссылкой.

**Закрытие группы:** отчётить каждый из пяти результатов отдельно; partial/blocked не превращать в completed. Реально готовые ранее parts подтвердить, не переписывать.

## G006. Наполненные вёдра

**Приоритет:** P0. **Карточки:** W06-026…W06-030.

**Связь:** W05-006.

**Сквозной объём направления:**

Эталон: BucketItem, DispenseItemBehavior, LiquidBlock. Связь: W04-033.
Подключить воду, лаву и рыхлый снег, waterlogging и остаток тары. Проверить допустимую
и заблокированную цель, среду, полный dispenser и обычное использование предмета.

**Проверенные Java-точки входа:** `world/item/BucketItem.java`; `core/dispenser/DispenseItemBehavior.java`; `world/level/block/LiquidBlock.java`

**Базовые зависимости:** нет заранее подтверждённой межгрупповой зависимости; проверить actual consumers. Нумерация не разрешает fake foundation.

### W06-026. Размещение воды

**Сделать и доказать:** Полное ведро использует настоящий fluid container/waterlogging consumer и оставляет правильную тару.

### W06-027. Размещение лавы

**Сделать и доказать:** Проверить target replacement, reactions и dimension environment attributes, не копировать правила воды без сверки.

### W06-028. Рыхлый снег

**Сделать и доказать:** Powder-snow bucket ставит нужный блок с правильными sound/event и остатком, включая rejected target.

### W06-029. Полный dispenser

**Сделать и доказать:** Результирующая тара вставляется или выбрасывается по Java; полностью занятые слоты не теряют ведро.

### W06-030. Общий item consumer

**Сделать и доказать:** Ручное use и dispenser делят допустимую общую логику; creative, обе руки и forbidden target не расходятся.

**Закрытие группы:** отчётить каждый из пяти результатов отдельно; partial/blocked не превращать в completed. Реально готовые ранее parts подтвердить, не переписывать.

## G007. Подбор пустым ведром

**Приоритет:** P0. **Карточки:** W06-031…W06-035.

**Связь:** W05-007.

**Сквозной объём направления:**

Эталон: BucketPickup, DispenseItemBehavior. Зависимость: 006.
Связать pickup_block и заполненную тару; flow не подменять source fluid. Проверить
успех/отказ, полный inventory и выдачу лишней тары, без изменения блока до успешной операции.

**Проверенные Java-точки входа:** `world/level/block/BucketPickup.java`; `core/dispenser/DispenseItemBehavior.java`

**Базовые зависимости:** G006. Нумерация не разрешает fake foundation.

### W06-031. Pickup воды

**Сделать и доказать:** Только подходящий source block выдаёт water bucket; успешная операция удаляет ровно этот источник.

### W06-032. Pickup лавы

**Сделать и доказать:** Правильный lava source и невозможная цель проходят свой pickup contract, а не проверку одного block ID.

### W06-033. Pickup рыхлого снега

**Сделать и доказать:** Пустая тара извлекает powder snow через BlockBehavior capability и возвращает правильный предмет.

### W06-034. Слоты и остатки

**Сделать и доказать:** Одна тара в неполном/полном dispenser обрабатывается без дюпа и без перезаписи соседнего stack.

### W06-035. Последствия подбора

**Сделать и доказать:** После удаления обновляются соседи, fluid ticks, comparator и client state в требуемом Java порядке.

**Закрытие группы:** отчётить каждый из пяти результатов отдельно; partial/blocked не превращать в completed. Реально готовые ранее parts подтвердить, не переписывать.

## G008. Вёдра с существами

**Приоритет:** P1. **Карточки:** W06-036…W06-040.

**Связь:** W05-008.

**Сквозной объём направления:**

Эталон: MobBucketItem, Bucketable. Зависимости: 006, 079.
Выпускать настоящих существ с entity data/from-bucket состоянием. Проверить round-trip
подбора/выпуска, содержимое ведра и остаток тары; отсутствующий aquatic AI не скрывать.

**Проверенные Java-точки входа:** `world/item/MobBucketItem.java`; `world/entity/Bucketable.java`

**Базовые зависимости:** G006, G079. Нумерация не разрешает fake foundation.

### W06-036. Данные ведра существа

**Сделать и доказать:** MobBucketItem читает настоящие entity components; неизвестные или недопустимые данные обрабатываются по source contract.

### W06-037. Выпуск существа

**Сделать и доказать:** Успех создаёт реальную aquatic entity с from-bucket состоянием; отказ размещения не объявляется успешным spawn.

### W06-038. Подбор обратно

**Сделать и доказать:** Игрок подбирает живую допустимую entity; её состояние переносится в item и entity удаляется ровно один раз.

### W06-039. Persistence и despawn

**Сделать и доказать:** Released entity сохраняет owner-independent bucket flags, возраст и разрешённые species fields после reload.

### W06-040. Автоматизация

**Сделать и доказать:** Dispenser выпускает то же существо и правильно выдаёт пустое ведро; source fluid и полный inventory проверены.

**Закрытие группы:** отчётить каждый из пяти результатов отдельно; partial/blocked не превращать в completed. Реально готовые ранее parts подтвердить, не переписывать.

## G009. Оставшиеся equippable случаи

**Приоритет:** P0. **Карточки:** W06-041…W06-045.

**Связь:** W05-009.

**Сквозной объём направления:**

Эталон: EquipmentDispenseItemBehavior, LivingEntity. Связь: W04-034.
Сохранить готовый fallback; проверить saddle/body slots/allowed entities/занятый slot.
Player и mob получают нужное снаряжение, mob — guaranteed drop/persistence; dropper экипировку не надевает.

**Проверенные Java-точки входа:** `core/dispenser/EquipmentDispenseItemBehavior.java`; `world/entity/LivingEntity.java`

**Базовые зависимости:** нет заранее подтверждённой межгрупповой зависимости; проверить actual consumers. Нумерация не разрешает fake foundation.

### W06-041. Humanoid slots

**Сделать и доказать:** Armor/equippable item выбирает фактический допустимый slot игрока; занятый слот не перезаписывается.

### W06-042. Body и saddle slots

**Сделать и доказать:** Поддерживаемые мобы используют свои body/saddle возможности и allowed-entities predicate.

### W06-043. Приоритет dispatch

**Сделать и доказать:** Явное projectile behavior побеждает component fallback; stack с несколькими возможностями не выполняет две ветки.

### W06-044. Persistence экипировки

**Сделать и доказать:** Успешно оснащённый mob получает guaranteed drop/persistence и не теряет предмет при save/load.

### W06-045. Callbacks и отказ

**Сделать и доказать:** Equip sound/event вызываются после inventory lock; неподходящая цель даёт ванильный fallback, dropper ничего не надевает.

**Закрытие группы:** отчётить каждый из пяти результатов отдельно; partial/blocked не превращать в completed. Реально готовые ранее parts подтвердить, не переписывать.

## G010. Огниво и fire charge

**Приоритет:** P0. **Карточки:** W06-046…W06-050.

**Связь:** W05-010.

**Сквозной объём направления:**

Эталон: DispenseItemBehavior, TntBlock. Связь: W04-035.
Подключить TNT, campfire/candle/fire и success/failure. Проверить прочность/расход,
TNT_EXPLODES, blocked target и events; второй explosion engine не создавать.

**Проверенные Java-точки входа:** `core/dispenser/DispenseItemBehavior.java`; `world/level/block/TntBlock.java`

**Базовые зависимости:** нет заранее подтверждённой межгрупповой зависимости; проверить actual consumers. Нумерация не разрешает fake foundation.

### W06-046. TNT ignition

**Сделать и доказать:** Огниво и заряд используют существующий TntBlock prime, правильную attribution и TNT_EXPLODES.

### W06-047. Campfire и candle

**Сделать и доказать:** Подходящий незажжённый блок меняет LIT state; уже зажжённый не получает повторный success расход.

### W06-048. Создание огня

**Сделать и доказать:** Свободная и занятая цель проверяются через настоящий FireBlock consumer и survival predicate.

### W06-049. Прочность и charge

**Сделать и доказать:** Flint durability и одноразовый fire charge расходуются по своим правилам, включая creative и break event.

### W06-050. Failure и events

**Сделать и доказать:** Успешная/неуспешная попытка дают правильные звуки и smoke; запрещённая цель не меняет мир частично.

**Закрытие группы:** отчётить каждый из пяти результатов отдельно; partial/blocked не превращать в completed. Реально готовые ранее parts подтвердить, не переписывать.

## G011. Костная мука

**Приоритет:** P0. **Карточки:** W06-051…W06-055.

**Связь:** W05-011.

**Сквозной объём направления:**

Эталон: BoneMealItem, DispenseItemBehavior. Связь: W04-035.
Настоящие growable consumers для item/dispenser. Проверить crop, sapling и water plant,
расход, failure и existing worldgen features; hardcoded случайный блок не считается ростом.

**Проверенные Java-точки входа:** `world/item/BoneMealItem.java`; `core/dispenser/DispenseItemBehavior.java`

**Базовые зависимости:** нет заранее подтверждённой межгрупповой зависимости; проверить actual consumers. Нумерация не разрешает fake foundation.

### W06-051. Обычные crops

**Сделать и доказать:** Bone meal вызывает существующую crop growth логику; максимальный возраст и неподходящий блок проверены.

### W06-052. Sapling и feature

**Сделать и доказать:** Рост использует настоящую tree feature с её RNG и space checks; случайный hardcoded ствол запрещён.

### W06-053. Подводный рост

**Сделать и доказать:** Водные plants и соседние поверхности выбираются по Java, учитывая fluid и dimension rules.

### W06-054. Dispenser consumer

**Сделать и доказать:** Раздатчик выполняет тот же growth contract и правильные success/failure события.

### W06-055. Расход и повторение

**Сделать и доказать:** Расход, creative и failed growth не создают бесплатных попыток или лишних drops; сохранить seeded systems.

**Закрытие группы:** отчётить каждый из пяти результатов отдельно; partial/blocked не превращать в completed. Реально готовые ранее parts подтвердить, не переписывать.

## G012. Бутылочки

**Приоритет:** P0. **Карточки:** W06-056…W06-060.

**Связь:** W05-012.

**Сквозной объём направления:**

Эталон: BottleItem, BeehiveBlock, DispenseItemBehavior.
Набирать воду/мёд, изменять блок и выдавать тару. Проверить неподходящий блок,
полный улей и полный inventory без потери результата или повторной выдачи.

**Проверенные Java-точки входа:** `world/item/BottleItem.java`; `world/level/block/BeehiveBlock.java`; `core/dispenser/DispenseItemBehavior.java`

**Базовые зависимости:** нет заранее подтверждённой межгрупповой зависимости; проверить actual consumers. Нумерация не разрешает fake foundation.

### W06-056. Бутылочка воды

**Сделать и доказать:** Target water и ручной raycast определяют настоящую наполненную бутылку; flowing/source различаются как Java.

### W06-057. Мёд из улья

**Сделать и доказать:** Готовый honey level уменьшается нужным образом, выдаёт honey bottle и выполняет предусмотренную реакцию пчёл.

### W06-058. Остатки и полный inventory

**Сделать и доказать:** Filled bottle вставляется или выпадает без потери исходной пустой тары и без двойного результата.

### W06-059. Раздатчик бутылочек

**Сделать и доказать:** Автоматическая ветка выбирает допустимую цель в правильном приоритете и отправляет нужные events.

### W06-060. Отказ и reload

**Сделать и доказать:** Неподходящий target остаётся неизменным; сохранённый honey state не восстанавливает уже собранный мёд.

**Закрытие группы:** отчётить каждый из пяти результатов отдельно; partial/blocked не превращать в completed. Реально готовые ранее parts подтвердить, не переписывать.

## G013. Ножницы

**Приоритет:** P0. **Карточки:** W06-061…W06-065.

**Связь:** W05-013.

**Сквозной объём направления:**

Эталон: ShearsItem, Sheep, BeehiveBlock.
Использовать общие реальные shear consumers. Проверить wool/honeycomb, прочность,
дроп и недопустимое повторное срезание; gamerules/events совпадают с Java.

**Проверенные Java-точки входа:** `world/item/ShearsItem.java`; `world/entity/animal/sheep/Sheep.java`; `world/level/block/BeehiveBlock.java`

**Базовые зависимости:** нет заранее подтверждённой межгрупповой зависимости; проверить actual consumers. Нумерация не разрешает fake foundation.

### W06-061. Стрижка овцы

**Сделать и доказать:** Ножницы вызывают настоящий shearing consumer; цвет и число drops следуют данным и Java probability.

### W06-062. Соты из улья

**Сделать и доказать:** Допустимый honey level даёт соты и изменяет улей; дым и реакция пчёл учитываются настоящими consumers.

### W06-063. Отбор shearable

**Сделать и доказать:** Раздатчик ищет подходящую entity в правильном AABB и не стрижёт каждую entity из выборки.

### W06-064. Прочность инструмента

**Сделать и доказать:** Успех повреждает ножницы с нужными effects; creative и поломка обработаны отдельно от failure.

### W06-065. Events и повтор

**Сделать и доказать:** Играются правильные звук/game event; уже остриженная или неподходящая цель не даёт повторный дроп.

**Закрытие группы:** отчётить каждый из пяти результатов отдельно; partial/blocked не превращать в completed. Реально готовые ранее parts подтвердить, не переписывать.

## G014. Головы и тыквы

**Приоритет:** P1. **Карточки:** W06-066…W06-070.

**Связь:** W05-014.

**Сквозной объём направления:**

Эталон: CarvedPumpkinBlock, WitherSkullBlock. Зависимость: 077.
Специальные dispenser branches замкнуть на настоящие конструкции и spawning.
Неверная конструкция цела, успех удаляет ровно нужные блоки; NBT-маркер вместо моба запрещён.

**Проверенные Java-точки входа:** `world/level/block/CarvedPumpkinBlock.java`; `world/level/block/WitherSkullBlock.java`

**Базовые зависимости:** нет заранее подтверждённой межгрупповой зависимости; проверить actual consumers. Нумерация не разрешает fake foundation.

### W06-066. Снежная конструкция

**Сделать и доказать:** Carved pumpkin распознаёт snow golem pattern, направление и свободное место по Java.

**Дополнительно требуется:** G077. Остальную независимую часть группы не блокировать.

### W06-067. Железная конструкция

**Сделать и доказать:** Iron golem pattern учитывает допустимые состояния блоков и player-created происхождение.

**Дополнительно требуется:** G077. Остальную независимую часть группы не блокировать.

### W06-068. Конструкция визера

**Сделать и доказать:** Skull placement проверяет настоящий wither pattern и условия появления, не только три головы в линию.

**Дополнительно требуется:** G115. Остальную независимую часть группы не блокировать.

### W06-069. Dispenser placement

**Сделать и доказать:** Специальные branches ставят голову/тыкву либо надевают предмет в ванильном приоритете.

### W06-070. Атомарность результата

**Сделать и доказать:** Неверный pattern не разрушает опору; успех удаляет ровно pattern blocks и создаёт одну живую сущность.

**Закрытие группы:** отчётить каждый из пяти результатов отдельно; partial/blocked не превращать в completed. Реально готовые ранее parts подтвердить, не переписывать.

## G015. Default-dispatch 26.3

**Приоритет:** P1. **Карточки:** W06-071…W06-075.

**Связь:** W05-015.

**Сквозной объём направления:**

Эталон: DispenserBlock, DispenseItemBehavior.
Проверить explicit behavior/entity-data spawn egg/equippable/sulfur-cube swallowing
и их приоритет. ItemStack с несколькими подходящими компонентами выбирает ровно ванильную ветку.

**Проверенные Java-точки входа:** `world/level/block/DispenserBlock.java`; `core/dispenser/DispenseItemBehavior.java`

**Базовые зависимости:** нет заранее подтверждённой межгрупповой зависимости; проверить actual consumers. Нумерация не разрешает fake foundation.

### W06-071. Явные behaviors

**Сделать и доказать:** Registry dispatch сохраняет приоритет зарегистрированного действия перед общими component routes.

### W06-072. Entity-data eggs

**Сделать и доказать:** Spawn egg route получает тип из настоящего ItemStack entity data, не вручную составленного списка IDs.

### W06-073. Sulfur swallowing

**Сделать и доказать:** Подключить реальный consumer sulfur cube; при его отсутствии не выдавать swallow за успех.

### W06-074. Component fallback

**Сделать и доказать:** Equippable и остальные default cases проверяют тот же stack, без потери custom components.

### W06-075. Коллизия условий

**Сделать и доказать:** Один stack, подходящий нескольким веткам, выполняет только первый ванильный consumer и расходуется один раз.

**Закрытие группы:** отчётить каждый из пяти результатов отдельно; partial/blocked не превращать в completed. Реально готовые ранее parts подтвердить, не переписывать.

## G016. Лодки из раздатчика

**Приоритет:** P1. **Карточки:** W06-076…W06-080.

**Связь:** W05-016.

**Сквозной объём направления:**

Эталон: BoatDispenseItemBehavior, AbstractBoat. Зависимость: 059.
Подключить настоящие boat/raft/chest варианты. Проверить source water, воздух над
водой, сушу, положение/rotation и components; rejected placement не дублирует предмет.

**Проверенные Java-точки входа:** `core/dispenser/BoatDispenseItemBehavior.java`; `world/entity/vehicle/boat/AbstractBoat.java`

**Базовые зависимости:** G059. Нумерация не разрешает fake foundation.

### W06-076. Spawn над водой

**Сделать и доказать:** Boat dispenser проверяет воду перед блоком и пространство, создавая реальную лодку в правильной позиции.

### W06-077. Воздух над водой

**Сделать и доказать:** Отдельный случай воздуха перед dispenser и воды ниже выбирает правильный offset и success.

### W06-078. Дерево и плот

**Сделать и доказать:** Типизированно подключить поддерживаемые wood/raft варианты по generated refs и entity data.

### W06-079. Chest variants

**Сделать и доказать:** Chest boat/raft получает item components и настоящий container consumer; декоративная entity недостаточна.

### W06-080. Отказ и ориентация

**Сделать и доказать:** Суша, obstruction и rejected insertion проходят Java fallback; facing/yaw и расход не дублируются.

**Закрытие группы:** отчётить каждый из пяти результатов отдельно; partial/blocked не превращать в completed. Реально готовые ранее parts подтвердить, не переписывать.

## G017. Стойка из раздатчика

**Приоритет:** P1. **Карточки:** W06-081…W06-085.

**Связь:** W05-017.

**Сквозной объём направления:**

Эталон: ArmorStandItem, ArmorStand. Зависимость: 052.
Использовать общую установку живой стойки. Проверить координаты, направление,
entity components и отказ; placeholder ArmorStand factory не закрывает ветку.

**Проверенные Java-точки входа:** `world/item/ArmorStandItem.java`; `world/entity/decoration/ArmorStand.java`

**Базовые зависимости:** G052. Нумерация не разрешает fake foundation.

### W06-081. Установка стойки

**Сделать и доказать:** Dispenser создаёт настоящую ArmorStand с ванильными координатами, yaw и collision checks.

### W06-082. Entity components

**Сделать и доказать:** Стойка получает допустимые components/pose/flags из предмета без произвольного NBT merge.

### W06-083. Занятая клетка

**Сделать и доказать:** Нельзя создать второй объект внутри запрещённой collision области; item остаётся/выбрасывается по Java.

### W06-084. Ручной placement

**Сделать и доказать:** Обычный use и dispenser сохраняют общие правила, различая source player и права Adventure.

### W06-085. После установки

**Сделать и доказать:** Стойка доступна оборудованию, damage и persistence; наличие только factory не закрывает карточку.

**Закрытие группы:** отчётить каждый из пяти результатов отдельно; partial/blocked не превращать в completed. Реально готовые ранее parts подтвердить, не переписывать.

## G018. Автокрафтер

**Приоритет:** P1. **Карточки:** W06-086…W06-090.

**Связь:** W05-018.

**Сквозной объём направления:**

Эталон: CrafterBlock, CrafterBlockEntity, CrafterMenu. Связь: W04-030, 037.
Блок/BE/menu, togglable slots, redstone craft, output/remainders. Проверить вставку
результата или выброс, comparator, NBT и повторный импульс без дюпа.

**Проверенные Java-точки входа:** `world/level/block/CrafterBlock.java`; `world/level/block/entity/CrafterBlockEntity.java`; `world/inventory/CrafterMenu.java`

**Базовые зависимости:** нет заранее подтверждённой межгрупповой зависимости; проверить actual consumers. Нумерация не разрешает fake foundation.

### W06-086. Состояние crafter

**Сделать и доказать:** Block state, BE inventory и menu представляют disabled slots и active crafting без фиктивных полей.

### W06-087. Редстоун и delay

**Сделать и доказать:** Импульс запускает настоящий scheduled craft; удерживаемое питание и повторный импульс не клонируют рецепт.

### W06-088. Выбор рецепта

**Сделать и доказать:** Ingredients и disabled slots участвуют в настоящем recipe lookup, включая неполную сетку.

### W06-089. Результат и остатки

**Сделать и доказать:** Output/remainders вставляются в контейнер впереди либо выбрасываются в ванильном порядке.

**Точечная зависимость:** W06-182. Допускается совместный вертикальный slice с настоящим consumer.

### W06-090. Sync и NBT

**Сделать и доказать:** Slot toggles, comparator и animation видны клиенту; reload не меняет сетку и не повторяет завершённый craft.

**Закрытие группы:** отчётить каждый из пяти результатов отдельно; partial/blocked не превращать в completed. Реально готовые ранее parts подтвердить, не переписывать.

## G019. Зельеварка

**Приоритет:** P1. **Карточки:** W06-091…W06-095.

**Связь:** W05-019.

**Сквозной объём направления:**

Эталон: BrewingStandBlockEntity, BrewingStandMenu, BrewingRecipe. Связь: W04-036.
Использовать рецепты 26.3, топливо и brewing cycle. Три бутылки, смена ingredient,
hopper sides и reload сохраняют progress; старый PotionBrewing из другой версии не копировать.

**Проверенные Java-точки входа:** `world/level/block/entity/BrewingStandBlockEntity.java`; `world/inventory/BrewingStandMenu.java`; `world/item/crafting/BrewingRecipe.java`

**Базовые зависимости:** нет заранее подтверждённой межгрупповой зависимости; проверить actual consumers. Нумерация не разрешает fake foundation.

### W06-091. Brewing recipes 26.3

**Сделать и доказать:** Использовать настоящие BrewingRecipe/extracted refs; старую hardcoded PotionBrewing таблицу не переносить.

### W06-092. Топливо и ingredient

**Сделать и доказать:** Fuel и reagent расходуются в правильный момент; отмена при смене input не оставляет бесплатную варку.

### W06-093. Bottle slots

**Сделать и доказать:** Три входа и результат сохраняют potion components, включая custom effects и допустимость конкретного рецепта.

### W06-094. Menu и hopper

**Сделать и доказать:** Properties, sided access и полный output работают через реальные slots, не обходя inventory contract.

### W06-095. Reload посреди цикла

**Сделать и доказать:** Сохранённая варка продолжается с правильным remaining time и один раз выдаёт результат.

**Закрытие группы:** отчётить каждый из пяти результатов отдельно; partial/blocked не превращать в completed. Реально готовые ранее parts подтвердить, не переписывать.

## G020. Цепочка автоматизации

**Приоритет:** P1. **Карточки:** W06-096…W06-100.

**Связь:** W05-020.

**Сквозной объём направления:**

Эталон: HopperBlockEntity, DispenserBlock, TntBlock. Зависимости: готовые 001–019.
Собрать сундук→hopper→dropper/dispenser→действие. Несколько настоящих ticks,
reload и полный output сохраняют баланс предметов; direct helper call не заменяет цепочку.

**Проверенные Java-точки входа:** `world/level/block/entity/HopperBlockEntity.java`; `world/level/block/DispenserBlock.java`; `world/level/block/TntBlock.java`

**Базовые зависимости:** нет заранее подтверждённой межгрупповой зависимости; проверить actual consumers. Нумерация не разрешает fake foundation.

### W06-096. Подача предметов

**Сделать и доказать:** Сундук→hopper→dropper работает на настоящих ticks с cooldown и sided constraints.

### W06-097. Действие раздатчика

**Сделать и доказать:** Доставленный projectile/bucket/tool запускает свой consumer ровно на соответствующий redstone pulse.

### W06-098. Забитая линия

**Сделать и доказать:** Полный output и непригодная цель не уничтожают items и не сбрасывают cooldown ради обхода.

### W06-099. Два зрителя

**Сделать и доказать:** Изменения автоматизации видят оба menu viewers; клиентское старое preview не создаёт второй результат.

### W06-100. Reload цепочки

**Сделать и доказать:** Выгрузка и загрузка чанков сохраняют inventories/timers/RNG; баланс items после продолжения сходится.

**Закрытие группы:** отчётить каждый из пяти результатов отдельно; partial/blocked не превращать в completed. Реально готовые ранее parts подтвердить, не переписывать.

## G021. Implicit components round-trip

**Приоритет:** P0. **Карточки:** W06-101…W06-105.

**Связь:** W05-021.

**Сквозной объём направления:**

Эталон: BlockEntity. Связь: W04-014.
Довести disk/owned/borrowed/live base components. Save/load, item placement/pick
не теряют custom data и не возвращают удалённый компонент из старого NBT.

**Проверенные Java-точки входа:** `world/level/block/entity/BlockEntity.java`

**Базовые зависимости:** нет заранее подтверждённой межгрупповой зависимости; проверить actual consumers. Нумерация не разрешает fake foundation.

### W06-101. Сохранение base components

**Сделать и доказать:** Disk save содержит непрямые components отдельно от implicit полей и не записывает значение дважды.

### W06-102. Borrowed и owned load

**Сделать и доказать:** Оба NBT пути одинаково восстанавливают базовые и специализированные поля без потери unknown components.

### W06-103. Item placement

**Сделать и доказать:** Block item применяет components к новой BE после нужной initialization, а не к временной копии.

### W06-104. Pick и drop round-trip

**Сделать и доказать:** Block→item→block сохраняет разрешённые данные; удалённый component не воскресает из старого NBT.

### W06-105. Reload существующего мира

**Сделать и доказать:** Настоящий saved BE с именем, lock и contents проходит unload/load и остаётся функциональным.

**Закрытие группы:** отчётить каждый из пяти результатов отдельно; partial/blocked не превращать в completed. Реально готовые ранее parts подтвердить, не переписывать.

## G022. Живое /data

**Приоритет:** P0. **Карточки:** W06-106…W06-110.

**Связь:** W05-022.

**Сквозной объём направления:**

Эталон: DataCommands, BlockDataAccessor, BlockEntity. Зависимость: 021.
Обновить live BE и menu sync с существующим BlockEntityLoadContext. Проверить
comparator/ticker; первоначальный disk load не должен повторно разрешать книгу кафедры.

**Проверенные Java-точки входа:** `server/commands/data/DataCommands.java`; `server/commands/data/BlockDataAccessor.java`; `world/level/block/entity/BlockEntity.java`

**Базовые зависимости:** нет заранее подтверждённой межгрупповой зависимости; проверить actual consumers. Нумерация не разрешает fake foundation.

### W06-106. Контекст живой загрузки

**Сделать и доказать:** Live /data использует существующий явный read context; disk/worldgen load до setLevel его не имитирует.

### W06-107. Меню после изменения

**Сделать и доказать:** Открытый viewer получает новые contents/properties, не сохраняя отдельную старую inventory копию.

### W06-108. Соседние системы

**Сделать и доказать:** Изменение NBT обновляет comparator, scheduled/ticker state и dirty flag там, где Java требует.

### W06-109. Ошибочный NBT

**Сделать и доказать:** Недопустимый payload не оставляет половину старого и половину нового состояния; ошибки не проглатываются.

### W06-110. Книги кафедры

**Сделать и доказать:** Селекторы/score/storage разрешаются в нужном контексте; повторное chunk activation не делает новый resolve.

**Закрытие группы:** отчётить каждый из пяти результатов отдельно; partial/blocked не превращать в completed. Реально готовые ранее parts подтвердить, не переписывать.

## G023. NBT рамок

**Приоритет:** P1. **Карточки:** W06-111…W06-115.

**Связь:** W05-023.

**Сквозной объём направления:**

Эталон: ItemFrame, GlowItemFrame. Зависимость карт: 082.
Проверить Fixed/Invisible/Rotation/drop chance/components через все NBT пути.
Сохранить готовые placement/pick и explosion veto обеих рамок; карты не подменять пустым tracking hook.

**Проверенные Java-точки входа:** `world/entity/decoration/ItemFrame.java`; `world/entity/decoration/GlowItemFrame.java`

**Базовые зависимости:** нет заранее подтверждённой межгрупповой зависимости; проверить actual consumers. Нумерация не разрешает fake foundation.

### W06-111. Fixed и Invisible

**Сделать и доказать:** Оба вида рамки одинаково читают и сохраняют flags; Fixed ограничивает нужные interactions, не только rendering.

### W06-112. Предмет и поворот

**Сделать и доказать:** Item/rotation/components проходят save/load и command data без изменения количества или угла.

### W06-113. Drop chance

**Сделать и доказать:** Нулевая и ненулевая вероятность проверяются на actual damage/drop consumer с gamerules и creative.

### W06-114. Опора и explosion veto

**Сделать и доказать:** Новая loading ветка не отключает support checks и защиту обычной/glow рамки от недопустимого взрыва.

### W06-115. Map linkage

**Сделать и доказать:** После готовности map manager снятие/замена/разрушение рамки удаляет реальный framed-map tracking без утечки.

**Дополнительно требуется:** G082. Остальную независимую часть группы не блокировать.

**Закрытие группы:** отчётить каждый из пяти результатов отдельно; partial/blocked не превращать в completed. Реально готовые ранее parts подтвердить, не переписывать.

## G024. Постоянное living state

**Приоритет:** P0. **Карточки:** W06-116…W06-120.

**Связь:** W05-024.

**Сквозной объём направления:**

Эталон: LivingEntity, ServerPlayer.
Health/HurtTime/DeathTime/effects/equipment сверить на save/load/respawn. Новая жизнь
не сохраняет старый hurt timer/death guard; command NBT и игровой state согласованы.

**Проверенные Java-точки входа:** `world/entity/LivingEntity.java`; `server/level/ServerPlayer.java`

**Базовые зависимости:** нет заранее подтверждённой межгрупповой зависимости; проверить actual consumers. Нумерация не разрешает fake foundation.

### W06-116. Health и death state

**Сделать и доказать:** Disk/entity load восстанавливает здоровье и death-related state согласно Java, не считая command dump загрузчиком.

### W06-117. HurtTime и cooldown

**Сделать и доказать:** Новые timers живут и сохраняются в правильных полях; partial damage не начинает animation заново.

### W06-118. Effects и equipment

**Сделать и доказать:** Сохранённые effects/armor/components продолжают действовать после reload с правильным dirty sync.

### W06-119. Новая жизнь игрока

**Сделать и доказать:** Respawn не переносит старые hurt/death guards, pending callbacks и неактуальные references.

### W06-120. Cross-world recreation

**Сделать и доказать:** Пересоздание живой entity сохраняет допустимые поля и owner attribution без повторного death loot.

**Закрытие группы:** отчётить каждый из пяти результатов отдельно; partial/blocked не превращать в completed. Реально готовые ранее parts подтвердить, не переписывать.

## G025. NBT раздатчика

**Приоритет:** P0. **Карточки:** W06-121…W06-125.

**Связь:** W05-025.

**Сквозной объём направления:**

Эталон: DispenserBlockEntity, DropperBlockEntity. Зависимость: 021.
Проверить loot/name/lock/base components через open/hopper/explosion. Lock до открытия,
seed и содержимое не сбрасываются, повторный consumer не генерирует лут второй раз.

**Проверенные Java-точки входа:** `world/level/block/entity/DispenserBlockEntity.java`; `world/level/block/entity/DropperBlockEntity.java`

**Базовые зависимости:** нет заранее подтверждённой межгрупповой зависимости; проверить actual consumers. Нумерация не разрешает fake foundation.

### W06-121. Lazy dispenser loot

**Сделать и доказать:** Load и automation не раскрывают loot преждевременно; первый настоящий consumer делает один roll.

### W06-122. Name и lock

**Сделать и доказать:** Имя, lock и права открытия переживают item placement, сохранение и изменение через commands.

### W06-123. Contents и components

**Сделать и доказать:** Все девять slots сохраняют components и item count без наложения implicit inventory данных.

### W06-124. Break и explosion

**Сделать и доказать:** Дроп содержимого происходит ровно один раз, включая нераскрытую таблицу и закрытие старого меню.

### W06-125. Seed и lifecycle

**Сделать и доказать:** Ненулевой seed воспроизводится, named stream продолжается после restart; два consumers не генерируют двойной loot.

**Закрытие группы:** отчётить каждый из пяти результатов отдельно; partial/blocked не превращать в completed. Реально готовые ранее parts подтвердить, не переписывать.

## G026. Прочность и equip hooks

**Приоритет:** P0. **Карточки:** W06-126…W06-130.

**Связь:** W05-026.

**Сквозной объём направления:**

Эталон: LivingEntity, ItemStack.
Замкнуть armor/tool damage на настоящие use/hurt paths. Проверить break/components,
creative/unbreaking/обе руки; callbacks не вызываются под inventory mutex.

**Проверенные Java-точки входа:** `world/entity/LivingEntity.java`; `world/item/ItemStack.java`

**Базовые зависимости:** нет заранее подтверждённой межгрупповой зависимости; проверить actual consumers. Нумерация не разрешает fake foundation.

### W06-126. Armor damage

**Сделать и доказать:** Настоящий hurt path повреждает нужные armor slots с правильным округлением и damage tags.

### W06-127. Tool damage

**Сделать и доказать:** Mining/use/attack подключают durability к соответствующим предметам, не общему unconditional decrement.

### W06-128. Unbreaking и creative

**Сделать и доказать:** Вероятности и exemptions проверены на реальном consumer; все enchanted tools не считать одинаковыми.

### W06-129. Поломка предмета

**Сделать и доказать:** Break event, удаление stack и прекращение use выполняются в правильной руке и ровно один раз.

### W06-130. Equipment hooks

**Сделать и доказать:** Смена повреждённого/сломавшегося предмета вызывает нужный callback после locks, сохраняя component comparison.

**Закрытие группы:** отчётить каждый из пяти результатов отдельно; partial/blocked не превращать в completed. Реально готовые ранее parts подтвердить, не переписывать.

## G027. Печь: результаты и XP

**Приоритет:** P0. **Карточки:** W06-131…W06-135.

**Связь:** W05-027.

**Сквозной объём направления:**

Эталон: AbstractFurnaceBlockEntity, AbstractFurnaceMenu. Связь: W04-044.
Recipe usage/output callbacks/stats и накопленный XP. Take/shift/hopper/reload
не удваивают опыт; выдача обязана быть связана с реальным smelting consumer.

**Проверенные Java-точки входа:** `world/level/block/entity/AbstractFurnaceBlockEntity.java`; `world/inventory/AbstractFurnaceMenu.java`

**Базовые зависимости:** нет заранее подтверждённой межгрупповой зависимости; проверить actual consumers. Нумерация не разрешает fake foundation.

### W06-131. Recipe usage

**Сделать и доказать:** Smelting помнит использованные рецепты и количества через реальный BE output consumer.

### W06-132. Output take

**Сделать и доказать:** Обычный, shift и hotbar take корректно начисляют crafting callbacks/statistics.

### W06-133. Опыт печи

**Сделать и доказать:** Выдача XP следует использованным recipes и RNG; preview и повторное открытие опыт не создают.

### W06-134. Автоматический output

**Сделать и доказать:** Hopper извлекает результат, сохраняя предусмотренное Java накопление recipe usage/XP.

### W06-135. Сохранение печи

**Сделать и доказать:** Reload не забывает накопленный опыт и не позволяет получить его повторно после завершённой выдачи.

**Закрытие группы:** отчётить каждый из пяти результатов отдельно; partial/blocked не превращать в completed. Реально готовые ранее parts подтвердить, не переписывать.

## G028. Печь: автоматизация

**Приоритет:** P0. **Карточки:** W06-136…W06-140.

**Связь:** W05-028.

**Сквозной объём направления:**

Эталон: HopperBlockEntity, AbstractFurnaceBlockEntity. Зависимость: 027.
Сохранить сделанные sided slots. Длинный fuel/remainder цикл, смена рецепта и
chunk unload не теряют progress и не пропускают предмет в запрещённый слот.

**Проверенные Java-точки входа:** `world/level/block/entity/HopperBlockEntity.java`; `world/level/block/entity/AbstractFurnaceBlockEntity.java`

**Базовые зависимости:** нет заранее подтверждённой межгрупповой зависимости; проверить actual consumers. Нумерация не разрешает fake foundation.

### W06-136. Fuel slots

**Сделать и доказать:** Sided insertion принимает допустимое топливо и не позволяет hopper подменить вход/выход печи.

### W06-137. Остаток топлива

**Сделать и доказать:** Ведро и другая тара появляются в правильном slot и корректно извлекаются автоматизацией.

### W06-138. Смена smelting input

**Сделать и доказать:** Смена рецепта и неподходящий ingredient правильно меняют progress, не сохраняя чужой cooked result.

### W06-139. Варианты печи

**Сделать и доказать:** Smoker/blast furnace используют свои recipe types, speed и допустимые inputs поверх общей foundation.

### W06-140. Длинный цикл

**Сделать и доказать:** Несколько загрузок топлива, полный output и chunk unload сохраняют баланс вещей и время.

**Закрытие группы:** отчётить каждый из пяти результатов отдельно; partial/blocked не превращать в completed. Реально готовые ранее parts подтвердить, не переписывать.

## G029. Украшенный горшок

**Приоритет:** P1. **Карточки:** W06-141…W06-145.

**Связь:** W05-029.

**Сквозной объём направления:**

Эталон: DecoratedPotBlock, DecoratedPotBlockEntity.
Довести insertion/wobble/break modes/components. Полный горшок, projectile/tool
break, NBT и comparator; содержимое падает ровно один раз.

**Проверенные Java-точки входа:** `world/level/block/DecoratedPotBlock.java`; `world/level/block/entity/DecoratedPotBlockEntity.java`

**Базовые зависимости:** нет заранее подтверждённой межгрупповой зависимости; проверить actual consumers. Нумерация не разрешает fake foundation.

### W06-141. Вставка в горшок

**Сделать и доказать:** Игрок кладёт допустимый item с правильным count и components; полный горшок не съедает предмет.

### W06-142. Wobble и звук

**Сделать и доказать:** Успех/отказ запускает нужную animation/event без фиктивного изменения inventory.

### W06-143. Разные break modes

**Сделать и доказать:** Tool/projectile/ordinary break выбирают правильный item и состояние cracked decoration.

### W06-144. Comparator и automation

**Сделать и доказать:** Хранимое содержимое доступно предусмотренным Java consumers; неправильный sided доступ не добавляется.

### W06-145. Round-trip

**Сделать и доказать:** Decorations, contents и base components переживают placement/pick/drop/reload без двойного contents drop.

**Закрытие группы:** отчётить каждый из пяти результатов отдельно; partial/blocked не превращать в completed. Реально готовые ранее parts подтвердить, не переписывать.

## G030. Shelf 26.3

**Приоритет:** P1. **Карточки:** W06-146…W06-150.

**Связь:** W05-030.

**Сквозной объём направления:**

Эталон: ShelfBlock, ShelfBlockEntity.
Сверить swap/interaction и хранение. Полный/пустой slot, обе руки, save/load,
два игрока и предусмотренные Java redstone/comparator consumers работают сквозным путём.

**Проверенные Java-точки входа:** `world/level/block/ShelfBlock.java`; `world/level/block/entity/ShelfBlockEntity.java`

**Базовые зависимости:** нет заранее подтверждённой межгрупповой зависимости; проверить actual consumers. Нумерация не разрешает fake foundation.

### W06-146. Хранилище Shelf

**Сделать и доказать:** Реальные slots и allowed contents соответствуют Java 26.3, а не выдуманному generic chest.

### W06-147. Player interaction

**Сделать и доказать:** Обе руки, пустая рука и недопустимый предмет выполняют правильный swap/insert contract.

### W06-148. Redstone interaction

**Сделать и доказать:** Подключить только предусмотренные source сигналы и выбранные hotbar/slot операции.

### W06-149. Синхронизация зрителей

**Сделать и доказать:** Два игрока видят одно хранилище и актуальный результат своих действий без локальных копий.

### W06-150. BE lifecycle

**Сделать и доказать:** Contents/components сохраняются, break выдаёт один дроп; unload не оставляет старые active callbacks.

**Закрытие группы:** отчётить каждый из пяти результатов отдельно; partial/blocked не превращать в completed. Реально готовые ранее parts подтвердить, не переписывать.

## G031. RecipeDisplay

**Приоритет:** P0. **Карточки:** W06-151…W06-155.

**Связь:** W05-031.

**Сквозной объём направления:**

Эталон: RecipeDisplay, SlotDisplay, RecipeManager.
Добавить варианты с настоящими recipe-book consumers, extracted IDs и codec tests.
Stonecutter update-recipes не переписывать: полный RecipeDisplay ему не нужен.

**Проверенные Java-точки входа:** `world/item/crafting/display/RecipeDisplay.java`; `world/item/crafting/display/SlotDisplay.java`; `world/item/crafting/RecipeManager.java`

**Базовые зависимости:** нет заранее подтверждённой межгрупповой зависимости; проверить actual consumers. Нумерация не разрешает fake foundation.

### W06-151. RecipeDisplay variants

**Сделать и доказать:** Generated typed registry и payload покрывают реально нужные виды display без ручных IDs.

### W06-152. SlotDisplay composition

**Сделать и доказать:** Composite/ingredient/result displays несут правильные holder sets, components и вложенные payload.

### W06-153. Recipe identity

**Сделать и доказать:** Реальный RecipeManager связывает display с серверным рецептом, сохраняя нужный порядок registration.

### W06-154. Wire encoding

**Сделать и доказать:** Пакетный consumer использует codec 26.3; expected bytes выводятся из Java, не самим serializer.

### W06-155. Совместимость stonecutter

**Сделать и доказать:** Новый display foundation не переписывает уже верный noRecipeCodec камнереза и initial update-recipes.

**Закрытие группы:** отчётить каждый из пяти результатов отдельно; partial/blocked не превращать в completed. Реально готовые ранее parts подтвердить, не переписывать.

## G032. ServerRecipeBook

**Приоритет:** P0. **Карточки:** W06-156…W06-160.

**Связь:** W05-032.

**Сквозной объём направления:**

Эталон: ServerRecipeBook. Зависимость: 031.
Known/highlight recipes и settings сохраняются на игроке. Login/rejoin/respawn,
серверное удаление рецепта и повторное открытие не оставляют недействительные записи.

**Проверенные Java-точки входа:** `stats/ServerRecipeBook.java`

**Базовые зависимости:** G031. Нумерация не разрешает fake foundation.

### W06-156. Известные рецепты

**Сделать и доказать:** ServerRecipeBook хранит действительный набор известных recipes и корректно добавляет/удаляет их.

### W06-157. Highlight state

**Сделать и доказать:** Новые записи и seen/highlight переходы живут отдельно и не возвращаются после подтверждения клиентом.

### W06-158. Book settings

**Сделать и доказать:** Состояния вкладок/filtering применяются к нужному виду menu и сохраняются у игрока.

### W06-159. Player persistence

**Сделать и доказать:** Logout/rejoin/restart восстанавливают книгу, не привязывая её к временному menu экземпляру.

### W06-160. Изменение набора recipes

**Сделать и доказать:** Удалённый/недействительный recipe не остаётся доступным после server-side обновления book.

**Закрытие группы:** отчётить каждый из пяти результатов отдельно; partial/blocked не превращать в completed. Реально готовые ранее parts подтвердить, не переписывать.

## G033. Recipe-book packets

**Приоритет:** P0. **Карточки:** W06-161…W06-165.

**Связь:** W05-033.

**Сквозной объём направления:**

Эталон: ClientboundRecipeBookAddPacket, ServerGamePacketListenerImpl. Зависимость: 032.
Связать add/remove/settings с lifecycle. Unknown IDs, initial order относительно
tags/recipes и повторный login проверяются реальными пакетами, без ручных packet IDs.

**Проверенные Java-точки входа:** `network/protocol/game/ClientboundRecipeBookAddPacket.java`; `server/network/ServerGamePacketListenerImpl.java`

**Базовые зависимости:** G032. Нумерация не разрешает fake foundation.

### W06-161. Initial sync

**Сделать и доказать:** При входе клиент получает согласованные settings/known displays после необходимых tags и recipe данных.

### W06-162. Добавление recipes

**Сделать и доказать:** Сервер отправляет правильные entries/highlights; повторный award не дублирует клиентские записи.

### W06-163. Удаление recipes

**Сделать и доказать:** Remove packet и серверное состояние меняются вместе; неизвестный ID не повреждает книгу.

### W06-164. Serverbound settings

**Сделать и доказать:** Клиентские settings/seen packets валидируются и изменяют только допустимое состояние игрока.

### W06-165. Повторный lifecycle

**Сделать и доказать:** Respawn/world change/rejoin не вызывают лишний initial sync или потерю chat/recipe состояния.

**Закрытие группы:** отчётить каждый из пяти результатов отдельно; partial/blocked не превращать в completed. Реально готовые ранее parts подтвердить, не переписывать.

## G034. Place recipe

**Приоритет:** P0. **Карточки:** W06-166…W06-170.

**Связь:** W05-034.

**Сквозной объём направления:**

Эталон: ServerPlaceRecipe, RecipeBookMenu. Зависимости: 031–033.
Serverbound placement работает с авторитетным inventory и настоящей сеткой.
Shaped/shapeless, разные menu sizes и нехватка ingredients не создают дюп.

**Проверенные Java-точки входа:** `recipebook/ServerPlaceRecipe.java`; `world/inventory/RecipeBookMenu.java`

**Базовые зависимости:** G031, G032, G033. Нумерация не разрешает fake foundation.

### W06-166. Поиск ingredients

**Сделать и доказать:** Place recipe берёт ингредиенты из авторитетного inventory с учётом components и реально известного рецепта.

### W06-167. Shaped placement

**Сделать и доказать:** Положение и размеры shaped рецепта соответствуют menu grid, включая offset и очистку старых ingredients.

### W06-168. Shapeless placement

**Сделать и доказать:** Подходящие stacks распределяются без зависимости от произвольного порядка HashMap.

### W06-169. Недостаток материалов

**Сделать и доказать:** Ghost/failed placement и возврат старой сетки идут по Java, без создания недостающих items.

### W06-170. Разные меню

**Сделать и доказать:** Player crafting и crafting table используют общий contract с корректными grid/result boundaries.

**Закрытие группы:** отчётить каждый из пяти результатов отдельно; partial/blocked не превращать в completed. Реально готовые ранее parts подтвердить, не переписывать.

## G035. Max craft и компоненты

**Приоритет:** P0. **Карточки:** W06-171…W06-175.

**Связь:** W05-035.

**Сквозной объём направления:**

Эталон: StackedItemContents, RecipeBookMenu. Зависимость: 034.
Подбор ingredients и repeated craft учитывают stack limits/components.
Нестакаемые items, повреждённые tools и полный inventory не смешивают несовместимые stacks.

**Проверенные Java-точки входа:** `world/entity/player/StackedItemContents.java`; `world/inventory/RecipeBookMenu.java`

**Базовые зависимости:** G034. Нумерация не разрешает fake foundation.

### W06-171. Stack accounting

**Сделать и доказать:** Max-craft вычисляется по реальным лимитам и компонентной совместимости ingredients.

### W06-172. Выбор подходящих stacks

**Сделать и доказать:** Разные damaged/enchanted/custom stacks не сливаются в один воображаемый ingredient.

### W06-173. Повторное размещение

**Сделать и доказать:** Place-all и многократный запрос используют текущий inventory, не прошлый client preview.

### W06-174. Полный inventory

**Сделать и доказать:** Очистка и возврат сетки не перезаписывают занятые slots и не теряют overflow.

### W06-175. Согласование result

**Сделать и доказать:** После placement recipe lookup и output соответствуют новой реальной сетке, а не ожидаемому ID из пакета.

**Закрытие группы:** отчётить каждый из пяти результатов отдельно; partial/blocked не превращать в completed. Реально готовые ранее parts подтвердить, не переписывать.

## G036. Открытие рецептов

**Приоритет:** P0. **Карточки:** W06-176…W06-180.

**Связь:** W05-036.

**Сквозной объём направления:**

Эталон: KnowledgeBookItem, RecipeCommand, ServerRecipeBook. Зависимость: 032.
Awards/limited crafting/knowledge book и команды give/take работают сквозным путём.
Проверить права, неверный key и повторную выдачу; stat counter не равен advancement.

**Проверенные Java-точки входа:** `world/item/KnowledgeBookItem.java`; `server/commands/RecipeCommand.java`; `stats/ServerRecipeBook.java`

**Базовые зависимости:** G032. Нумерация не разрешает fake foundation.

### W06-176. Recipe awards

**Сделать и доказать:** Настоящие pickup/crafting/прочие Java consumers открывают нужные recipes ровно в предусмотренных местах.

### W06-177. Limited crafting

**Сделать и доказать:** Знание рецепта действительно ограничивает crafting, когда gamerule включён; preview не обходит проверку.

### W06-178. Knowledge book

**Сделать и доказать:** Использование переносит валидные recipe keys и выполняет расход/statistics согласно Java.

### W06-179. Recipe commands

**Сделать и доказать:** Give/take проверяют permissions, targets и отсутствующие keys без частично неправильной book state.

### W06-180. Persisted unlocks

**Сделать и доказать:** Award/revoke/rejoin сохраняют правильный набор; наличие statistic не выдаётся за работающий advancement.

**Закрытие группы:** отчётить каждый из пяти результатов отдельно; partial/blocked не превращать в completed. Реально готовые ранее parts подтвердить, не переписывать.

## G037. Crafting callbacks

**Приоритет:** P0. **Карточки:** W06-181…W06-185.

**Связь:** W05-037.

**Сквозной объём направления:**

Эталон: ItemStack, ResultSlot, CrafterBlock. Зависимости: 018, 081 для maps.
Различить onCraftedBy/System/postprocess в 26.3, сохранив исправленный ПКМ/Q.
Preview не меняет saved map, take и crafter не выполняют callback дважды.

**Проверенные Java-точки входа:** `world/item/ItemStack.java`; `world/inventory/ResultSlot.java`; `world/level/block/CrafterBlock.java`

**Базовые зависимости:** нет заранее подтверждённой межгрупповой зависимости; проверить actual consumers. Нумерация не разрешает fake foundation.

### W06-181. Player crafting callback

**Сделать и доказать:** ResultSlot и actual take вызывают onCraftedBy в правильном порядке с remainders и count.

### W06-182. System crafting callback

**Сделать и доказать:** Crafter использует onCraftedBySystem/postprocess, не фиктивного игрока и не двойное начисление stats.

### W06-183. Map postprocess

**Сделать и доказать:** Scale/lock/clone результата обновляют настоящие saved maps только после успешного take.

**Дополнительно требуется:** G081. Остальную независимую часть группы не блокировать.

### W06-184. Особые предметы

**Сделать и доказать:** Разобрать реальные component-based crafting hooks текущей версии, не добавляя пустую универсальную функцию.

### W06-185. Регрессии меню

**Сделать и доказать:** ПКМ/Q/shift/swap и повторный craft сохраняют исправленный whole-result/requested-count contract.

**Закрытие группы:** отчётить каждый из пяти результатов отдельно; partial/blocked не превращать в completed. Реально готовые ранее parts подтвердить, не переписывать.

## G038. Наковальня

**Приоритет:** P1. **Карточки:** W06-186…W06-190.

**Связь:** W05-038.

**Сквозной объём направления:**

Эталон: AnvilMenu.
Repair/combine/rename/enchantment limits и XP cost. Проверить too expensive,
creative, оба входа и пустое имя; XP списывается только за действительный take.

**Проверенные Java-точки входа:** `world/inventory/AnvilMenu.java`

**Базовые зависимости:** нет заранее подтверждённой межгрупповой зависимости; проверить actual consumers. Нумерация не разрешает fake foundation.

### W06-186. Repair и material cost

**Сделать и доказать:** Наковальня правильно комбинирует повреждения, repair materials и расход входных stacks.

### W06-187. Enchant combination

**Сделать и доказать:** Compatibility, levels, curses и prior-work cost берутся из реальных данных и source rules.

### W06-188. Rename

**Сделать и доказать:** Пустое/новое/прежнее имя изменяет result и стоимость по Java, не только display text.

### W06-189. Take и levels

**Сделать и доказать:** XP/materials списываются при действительном take; creative и too-expensive обработаны отдельно.

### W06-190. Закрытие и sync

**Сделать и доказать:** Data slots/result sync и возврат inputs при close/break сохраняют баланс, включая shift-click.

**Закрытие группы:** отчётить каждый из пяти результатов отдельно; partial/blocked не превращать в completed. Реально готовые ранее parts подтвердить, не переписывать.

## G039. Точило

**Приоритет:** P1. **Карточки:** W06-191…W06-195.

**Связь:** W05-039.

**Сквозной объём направления:**

Эталон: GrindstoneMenu.
Repair, curses и XP выдаются по Java. Curse-only item, два повреждённых предмета
и разные способы take не позволяют получать опыт за preview или повторно.

**Проверенные Java-точки входа:** `world/inventory/GrindstoneMenu.java`

**Базовые зависимости:** нет заранее подтверждённой межгрупповой зависимости; проверить actual consumers. Нумерация не разрешает fake foundation.

### W06-191. Очистка enchantments

**Сделать и доказать:** Grindstone снимает только допустимые enchantments и сохраняет curses/components.

### W06-192. Repair двух предметов

**Сделать и доказать:** Damage repair и бонус вычисляются по Java с правильным расходом обоих inputs.

### W06-193. Опыт

**Сделать и доказать:** XP зависит от реально снятых enchantments; curse-only/пустой результат не дают бесплатный опыт.

### W06-194. Result interactions

**Сделать и доказать:** Pickup/shift/hotbar/throw вызывают consumer один раз и не применяют postprocess в preview.

### W06-195. Lifecycle меню

**Сделать и доказать:** Close/break/reload не дублируют результат, а неудалённые inputs возвращаются правильному владельцу.

**Закрытие группы:** отчётить каждый из пяти результатов отдельно; partial/blocked не превращать в completed. Реально готовые ранее parts подтвердить, не переписывать.

## G040. Зачаровательный стол

**Приоритет:** P1. **Карточки:** W06-196…W06-200.

**Связь:** W05-040.

**Сквозной объём направления:**

Эталон: EnchantmentMenu.
Offers от player seed/bookshelf power/components. Одинаковый seed даёт те же offers,
lapis/levels списываются атомарно, новый seed появляется после успеха, не preview.

**Проверенные Java-точки входа:** `world/inventory/EnchantmentMenu.java`

**Базовые зависимости:** нет заранее подтверждённой межгрупповой зависимости; проверить actual consumers. Нумерация не разрешает fake foundation.

### W06-196. Bookshelf power

**Сделать и доказать:** Окружение стола и blocking positions реально влияют на допустимую силу offers.

### W06-197. Seeded offers

**Сделать и доказать:** Один player enchantment seed даёт те же cost/options; порядок RNG draws совпадает с Java.

### W06-198. Компоненты и eligibility

**Сделать и доказать:** Enchantable items, books, tags и conflicts выбираются по extracted typed data.

### W06-199. Подтверждение зачарования

**Сделать и доказать:** Lapis/levels/item/seed меняются атомарно после валидного menu action, не от client hints.

### W06-200. Sync и повторы

**Сделать и доказать:** Повтор старого button action и shift/close не создают второй enchantment или бесплатную смену seed.

**Закрытие группы:** отчётить каждый из пяти результатов отдельно; partial/blocked не превращать в completed. Реально готовые ранее parts подтвердить, не переписывать.

## G041. AbstractArrow

**Приоритет:** P1. **Карточки:** W06-201…W06-205.

**Связь:** W05-041.

**Сквозной объём направления:**

Эталон: AbstractArrow, BowItem.
Физика/collision/in-ground/owner с первым настоящим bow consumer. Стена, moving
entity и unload/reload; регистрировать пустую Arrow ради dispenser запрещено.

**Проверенные Java-точки входа:** `world/entity/projectile/arrow/AbstractArrow.java`; `world/item/BowItem.java`

**Базовые зависимости:** нет заранее подтверждённой межгрупповой зависимости; проверить actual consumers. Нумерация не разрешает fake foundation.

### W06-201. Flight foundation стрелы

**Сделать и доказать:** AbstractArrow выполняет настоящие physics и sweep collisions с первым реальным item consumer.

### W06-202. In-ground state

**Сделать и доказать:** Попадание в блок сохраняет stuck state, shake/lifetime и реакцию на изменение блока.

### W06-203. Entity hit pipeline

**Сделать и доказать:** Collision filter, pierce и owner immunity ведут к настоящему damage source, не прямому generic hurt.

### W06-204. Owner и recreation

**Сделать и доказать:** Ссылка owner сохраняется и разрешается после unload/teleport без удержания удалённой entity.

### W06-205. Persistence полёта

**Сделать и доказать:** Position/velocity/components/in-ground переживают save/load; projectile не попадает повторно из старого callback.

**Закрытие группы:** отчётить каждый из пяти результатов отдельно; partial/blocked не превращать в completed. Реально готовые ранее parts подтвердить, не переписывать.

## G042. Arrow и SpectralArrow

**Приоритет:** P1. **Карточки:** W06-206…W06-210.

**Связь:** W05-042.

**Сквозной объём направления:**

Эталон: Arrow, SpectralArrow. Зависимость: 041.
Damage/pickup/critical/piercing/glow и dispenser. Проверить creative-only pickup,
shield и застрявшую стрелу; damage идёт через общий hurt path.

**Проверенные Java-точки входа:** `world/entity/projectile/arrow/Arrow.java`; `world/entity/projectile/arrow/SpectralArrow.java`

**Базовые зависимости:** G041. Нумерация не разрешает fake foundation.

### W06-206. Обычная стрела

**Сделать и доказать:** Arrow использует общий flight consumer с правильным base damage и pickup item.

### W06-207. Spectral effect

**Сделать и доказать:** SpectralArrow реально применяет glowing на hit, сохраняя предусмотренную длительность.

### W06-208. Pickup policy

**Сделать и доказать:** Survival/creative-only/disallowed pickup различаются; повторная попытка не выдаёт второй stack.

### W06-209. Critical и piercing

**Сделать и доказать:** Дополнительный урон, список поражённых целей и продолжение полёта соответствуют Java.

### W06-210. Dispenser arrows

**Сделать и доказать:** Регистрировать arrows только после готовности consumers; launch/расход/events и origin pickup stack работают вместе.

**Закрытие группы:** отчётить каждый из пяти результатов отдельно; partial/blocked не превращать в completed. Реально готовые ранее parts подтвердить, не переписывать.

## G043. Стрелы с эффектами

**Приоритет:** P1. **Карточки:** W06-211…W06-215.

**Связь:** W05-043.

**Сквозной объём направления:**

Эталон: Arrow, PotionContents. Зависимости: 041, 042.
Potion/custom effects/color переживают flight/hit/pickup. Подобранный предмет
содержит допустимые компоненты; длительность эффектов совпадает с Java.

**Проверенные Java-точки входа:** `world/entity/projectile/arrow/Arrow.java`; `world/item/alchemy/PotionContents.java`

**Базовые зависимости:** G041, G042. Нумерация не разрешает fake foundation.

### W06-211. Potion arrow data

**Сделать и доказать:** Стрела получает base potion, custom effects и color из ItemStack components.

### W06-212. Hit effects

**Сделать и доказать:** Длительность и instant effects применяются с ванильным масштабированием и attribution.

### W06-213. Decay и длительный полёт

**Сделать и доказать:** Изменения potion state со временем и stuck состоянием выполняются только в предусмотренных Java случаях.

### W06-214. Pickup с данными

**Сделать и доказать:** Возвращённый item содержит актуальные допустимые components, не неизменённую копию исходного выстрела.

### W06-215. NBT и variants

**Сделать и доказать:** Tipped/spectral/обычная стрела после reload остаются правильными типами, без потери эффектов и pickup policy.

**Закрытие группы:** отчётить каждый из пяти результатов отдельно; partial/blocked не превращать в completed. Реально готовые ранее parts подтвердить, не переписывать.

## G044. Лук

**Приоритет:** P1. **Карточки:** W06-216…W06-220.

**Связь:** W05-044.

**Сквозной объём направления:**

Эталон: BowItem. Зависимость: 041.
Draw/release/ammo/enchants и статистика. Короткое натяжение, infinity/creative,
обе руки и отсутствие ammo; прямой hurt вместо летящей стрелы запрещён.

**Проверенные Java-точки входа:** `world/item/BowItem.java`

**Базовые зависимости:** G041. Нумерация не разрешает fake foundation.

### W06-216. Начало натяжения

**Сделать и доказать:** Bow use проверяет ammo и eligibility, запускает правильный use state для обеих рук.

### W06-217. Отпускание

**Сделать и доказать:** Draw duration даёт правильную скорость и critical state; слишком короткий draw не создаёт полноценный выстрел.

### W06-218. Ammo selection

**Сделать и доказать:** Inventory/off-hand/default creative ammo выбираются по source, сохраняя компоненты выбранного stack.

### W06-219. Enchantments

**Сделать и доказать:** Infinity/power/punch/flame и прочие реальные effects входят в общий projectile pipeline, не отдельные damage shortcuts.

### W06-220. Расход и статистика

**Сделать и доказать:** Ammo/durability/sound/statistics меняются один раз после реального выстрела; прерывание use не расходует их.

**Закрытие группы:** отчётить каждый из пяти результатов отдельно; partial/blocked не превращать в completed. Реально готовые ранее parts подтвердить, не переписывать.

## G045. Арбалет

**Приоритет:** P1. **Карточки:** W06-221…W06-225.

**Связь:** W05-045.

**Сквозной объём направления:**

Эталон: CrossbowItem. Зависимости: 041, 046.
Charging/charged projectiles/multishot/piercing/rockets. Прерывание и смена слота
не дублируют ammo; charged item сохраняется в NBT и стреляет один раз.

**Проверенные Java-точки входа:** `world/item/CrossbowItem.java`

**Базовые зависимости:** G041, G046. Нумерация не разрешает fake foundation.

### W06-221. Зарядка арбалета

**Сделать и доказать:** Use state, зарядные sounds и duration соответствуют Java и компонентам предмета.

### W06-222. Charged projectiles

**Сделать и доказать:** Снаряды сохраняются внутри компонента оружия, включая реальные arrows и rockets.

### W06-223. Выстрел и multishot

**Сделать и доказать:** Углы, число projectile и расход используют ванильный порядок; piercing передаётся самим стрелам.

### W06-224. Прерывание и смена слота

**Сделать и доказать:** Незавершённая зарядка не сохраняет бесплатный ammo и не продолжает старый use callback.

### W06-225. NBT заряженного оружия

**Сделать и доказать:** Rejoin/reload сохраняет законный charged state; повторный packet не стреляет уже израсходованным содержимым.

**Закрытие группы:** отчётить каждый из пяти результатов отдельно; partial/blocked не превращать в completed. Реально готовые ранее parts подтвердить, не переписывать.

## G046. Фейерверк

**Приоритет:** P1. **Карточки:** W06-226…W06-230.

**Связь:** W05-046.

**Сквозной объём направления:**

Эталон: FireworkRocketEntity, FireworkRocketItem.
Lifetime/elytra attachment/payload и dispenser. Проверить обычный запуск, полёт
игрока, damage через преграды и NBT; NBT-only ракета не закрывает задачу.

**Проверенные Java-точки входа:** `world/entity/projectile/FireworkRocketEntity.java`; `world/item/FireworkRocketItem.java`

**Базовые зависимости:** нет заранее подтверждённой межгрупповой зависимости; проверить actual consumers. Нумерация не разрешает fake foundation.

### W06-226. Самостоятельная ракета

**Сделать и доказать:** Rocket item создаёт настоящую entity с launch direction, lifetime и firework components.

### W06-227. Ракета на elytra

**Сделать и доказать:** Attached entity следует владельцу, даёт boost и корректно отсоединяется при изменении flight state.

### W06-228. Explosion payload

**Сделать и доказать:** Stars/effects и предусмотренный damage проходят реальные consumers, учитывая преграды и attribution.

### W06-229. Dispenser fireworks

**Сделать и доказать:** Launch config, shot-at-angle и расход используют тот же engine без декоративной substitute entity.

### W06-230. Save и завершение

**Сделать и доказать:** Reload сохраняет lifetime/attachment; одна ракета взрывается и наносит damage не более одного раза.

**Закрытие группы:** отчётить каждый из пяти результатов отдельно; partial/blocked не превращать в completed. Реально готовые ранее parts подтвердить, не переписывать.

## G047. Small fireball

**Приоритет:** P1. **Карточки:** W06-231…W06-235.

**Связь:** W05-047.

**Сквозной объём направления:**

Эталон: SmallFireball, FireChargeItem. Зависимость: 010.
Настоящий снаряд, dispenser config, hit и ignition. MobGriefing, горючая/негорючая
цель и owner attribution; обычный выброс fire charge предметом недостаточен.

**Проверенные Java-точки входа:** `world/entity/projectile/hurtingprojectile/SmallFireball.java`; `world/item/FireChargeItem.java`

**Базовые зависимости:** нет заранее подтверждённой межгрупповой зависимости; проверить actual consumers. Нумерация не разрешает fake foundation.

### W06-231. SmallFireball entity

**Сделать и доказать:** Настоящий projectile хранит acceleration/owner и выполняет движение с collision callbacks.

### W06-232. Попадание в entity

**Сделать и доказать:** Урон, поджигание и attribution учитывают immune targets и поддерживаемые effect hooks.

### W06-233. Попадание в блок

**Сделать и доказать:** Огонь ставится только в допустимой клетке с правильными mobGriefing и surface rules.

### W06-234. Fire-charge dispenser

**Сделать и доказать:** Item route использует свою config, consume/sound/events, а не обычный бросок предмета.

### W06-235. Persistence и отказы

**Сделать и доказать:** Reload/invalid owner/rejected insertion сохраняют предусмотренную семантику без второго fireball или возврата расхода.

**Закрытие группы:** отчётить каждый из пяти результатов отдельно; partial/blocked не превращать в completed. Реально готовые ранее parts подтвердить, не переписывать.

## G048. Wind charge

**Приоритет:** P1. **Карточки:** W06-236…W06-240.

**Связь:** W05-048.

**Сквозной объём направления:**

Эталон: WindCharge, AbstractWindCharge, ServerExplosion.
Добавить настоящий TRIGGER_BLOCK consumer в общую explosion систему. Knockback,
fall reset и активация блоков проверяются отдельно; TNT-разрушением не подменять.

**Проверенные Java-точки входа:** `world/entity/projectile/hurtingprojectile/windcharge/WindCharge.java`; `world/entity/projectile/hurtingprojectile/windcharge/AbstractWindCharge.java`; `world/level/ServerExplosion.java`

**Базовые зависимости:** нет заранее подтверждённой межгрупповой зависимости; проверить actual consumers. Нумерация не разрешает fake foundation.

### W06-236. Wind charge item

**Сделать и доказать:** Ручное use создаёт настоящий снаряд с cooldown, owner и components.

### W06-237. Trigger explosion mode

**Сделать и доказать:** Общий explosion engine получает самостоятельный TRIGGER_BLOCK contract, не маскируемый TNT Destroy.

### W06-238. Knockback и fall reset

**Сделать и доказать:** Живые цели и игрок получают нужный impulse и grace/fall-distance поведение.

### W06-239. Воздействие на блоки

**Сделать и доказать:** Doors/buttons/bells/прочие поддерживаемые цели проходят Java trigger consumers без разрушения terrain.

### W06-240. Варианты и dispenser

**Сделать и доказать:** Wind/Breeze variants и автоматический запуск различаются по source data; save/hit выполняются один раз.

**Закрытие группы:** отчётить каждый из пяти результатов отдельно; partial/blocked не превращать в completed. Реально готовые ранее parts подтвердить, не переписывать.

## G049. Жемчуг эндера

**Приоритет:** P1. **Карточки:** W06-241…W06-245.

**Связь:** W05-049.

**Сквозной объём направления:**

Эталон: ThrownEnderpearl, EnderpearlItem.
Owner/teleport/damage/tickets и сохранение. Смена мира/домена, смерть владельца,
restart; ссылка на владельца не удерживает disconnected player бесконечно.

**Проверенные Java-точки входа:** `world/entity/projectile/throwableitemprojectile/ThrownEnderpearl.java`; `world/item/EnderpearlItem.java`

**Базовые зависимости:** нет заранее подтверждённой межгрупповой зависимости; проверить actual consumers. Нумерация не разрешает fake foundation.

### W06-241. Throw и owner

**Сделать и доказать:** Ender pearl item и projectile используют настоящего владельца и корректный cooldown/consume.

### W06-242. Hit teleport

**Сделать и доказать:** Попадание вычисляет допустимую destination и выполняет реальный teleport, damage и effects.

### W06-243. Owner lifecycle

**Сделать и доказать:** Смерть/logout/смена мира владельца правильно влияют на pearl; сильных циклических ссылок нет.

### W06-244. Chunk tickets

**Сделать и доказать:** Активная pearl удерживает только нужную загрузку по ванильным условиям и освобождает tickets при завершении.

### W06-245. Save и domain transfer

**Сделать и доказать:** Reload и разрешённый cross-world переход сохраняют owner identity и не телепортируют чужого игрока.

**Закрытие группы:** отчётить каждый из пяти результатов отдельно; partial/blocked не превращать в completed. Реально готовые ранее parts подтвердить, не переписывать.

## G050. Рыбалка

**Приоритет:** P1. **Карточки:** W06-246…W06-250.

**Связь:** W05-050.

**Сквозной объём направления:**

Эталон: FishingHook, FishingRodItem.
Throw/retract/bobber/bite/seeded loot/tool effects. Вода/суша, пойманная entity,
обрыв owner и durability проходят реальные paths; лут берётся из общей системы.

**Проверенные Java-точки входа:** `world/entity/projectile/FishingHook.java`; `world/item/FishingRodItem.java`

**Базовые зависимости:** нет заранее подтверждённой межгрупповой зависимости; проверить actual consumers. Нумерация не разрешает fake foundation.

### W06-246. Cast состояния

**Сделать и доказать:** Fishing rod создаёт один bobber и переключает hand use между throw/retract, сохраняя owner.

### W06-247. Bobber physics

**Сделать и доказать:** Вода, воздух, земля и hooked entity дают реальные состояния движения и collision.

### W06-248. Поклёвка

**Сделать и доказать:** Wait/lure/approach timers и условия открытой воды работают на ticks с правильной вероятностью.

### W06-249. Reel и loot

**Сделать и доказать:** Вытягивание выдаёт actual loot/XP или притягивает entity, учитывая enchanted tool и seeded tables.

### W06-250. Разрыв lifecycle

**Сделать и доказать:** Owner logout/death, дистанция и reload не оставляют бесплатный bobber или повторный catch.

**Закрытие группы:** отчётить каждый из пяти результатов отдельно; partial/blocked не превращать в completed. Реально готовые ранее parts подтвердить, не переписывать.

## G051. Картины

**Приоритет:** P1. **Карточки:** W06-251…W06-255.

**Связь:** W05-051.

**Сквозной объём направления:**

Эталон: Painting, PaintingVariant, HangingEntity.
Variant/размер/место/опора/drop/pick/NBT. Проверить пересечение и creative/survival;
placeholder factory не равна установленной картине.

**Проверенные Java-точки входа:** `world/entity/decoration/painting/Painting.java`; `world/entity/decoration/painting/PaintingVariant.java`; `world/entity/decoration/HangingEntity.java`

**Базовые зависимости:** нет заранее подтверждённой межгрупповой зависимости; проверить actual consumers. Нумерация не разрешает fake foundation.

### W06-251. Выбор painting variant

**Сделать и доказать:** Предмет выбирает допустимый зарегистрированный вариант по свободной площади и source rules.

### W06-252. Размер и ориентация

**Сделать и доказать:** Bounding box и center соответствуют размерам варианта и направлению стены.

### W06-253. Размещение и опора

**Сделать и доказать:** Коллизия, неподходящая поверхность и пересечение с другими hanging entities запрещают placement.

### W06-254. Damage и drop

**Сделать и доказать:** Survival/creative/explosion/pick дают правильный предмет и сохраняют variant components.

### W06-255. Persistent painting

**Сделать и доказать:** После save/load painting остаётся тем же вариантом и немедленно использует настоящие support checks.

**Закрытие группы:** отчётить каждый из пяти результатов отдельно; partial/blocked не превращать в completed. Реально готовые ранее parts подтвердить, не переписывать.

## G052. Установка стойки

**Приоритет:** P1. **Карточки:** W06-256…W06-260.

**Связь:** W05-052.

**Сквозной объём направления:**

Эталон: ArmorStand, ArmorStandItem.
Размеры/marker/small/pose/hands и equipment access. Предмет ставит живую стойку,
обе руки работают со слотами; creative и permissions соответствуют Java.

**Проверенные Java-точки входа:** `world/entity/decoration/ArmorStand.java`; `world/item/ArmorStandItem.java`

**Базовые зависимости:** нет заранее подтверждённой межгрупповой зависимости; проверить actual consumers. Нумерация не разрешает fake foundation.

### W06-256. ArmorStand placement

**Сделать и доказать:** Предмет создаёт реальную стойку с проверкой места, yaw и entity components.

### W06-257. Размеры и flags

**Сделать и доказать:** Small/marker/invisible/show-arms/no-base-plate меняют actual collision и interaction, не только NBT.

### W06-258. Equipment access

**Сделать и доказать:** Слоты стойки доступны общим LivingEntity/equippable consumers с правильными restrictions.

### W06-259. Player interaction

**Сделать и доказать:** Handedness, hit location, empty hand и item exchange следуют Java без дюпа inventory.

### W06-260. Pose и sync

**Сделать и доказать:** Pose/flags/equipment сохраняются, отправляются клиенту и доступны command data после reload.

**Закрытие группы:** отчётить каждый из пяти результатов отдельно; partial/blocked не превращать в completed. Реально готовые ранее parts подтвердить, не переписывать.

## G053. Урон стойке

**Приоритет:** P1. **Карточки:** W06-261…W06-265.

**Связь:** W05-053.

**Сквозной объём направления:**

Эталон: ArmorStand. Зависимость: 052.
Damage exceptions, marker/invisible, break stages и equipment drops. Проверить
melee/projectile/fire/explosion/doEntityDrops; обычный mob death не заменяет контракт.

**Проверенные Java-точки входа:** `world/entity/decoration/ArmorStand.java`

**Базовые зависимости:** G052. Нумерация не разрешает fake foundation.

### W06-261. Damage predicate стойки

**Сделать и доказать:** Проверить invulnerable/marker/invisible и исключения damage sources до изменения equipment/state.

### W06-262. Удары игрока

**Сделать и доказать:** Первый/повторный удар, creative и время между ними выбирают правильное break поведение.

### W06-263. Projectile damage

**Сделать и доказать:** Arrow и другие projectile учитывают pierce/owner и не используют обычный mob death shortcut.

### W06-264. Fire и explosion

**Сделать и доказать:** Горение и взрыв обрабатывают health/drop/events согласно специальному ArmorStand contract.

### W06-265. Drop и removal

**Сделать и доказать:** Стойка и equipment выпадают ровно один раз, учитывая components, doEntityDrops и повторный callback.

**Закрытие группы:** отчётить каждый из пяти результатов отдельно; partial/blocked не превращать в completed. Реально готовые ранее parts подтвердить, не переписывать.

## G054. Cushion

**Приоритет:** P1. **Карточки:** W06-266…W06-270.

**Связь:** W05-054.

**Сквозной объём направления:**

Эталон: Cushion, CushionItem.
Placement/seating/passenger positions/опора. Mount/dismount двух игроков,
collision/save/load/drop; наличие сущности не считается реализацией посадки.

**Проверенные Java-точки входа:** `world/entity/decoration/Cushion.java`; `world/item/CushionItem.java`

**Базовые зависимости:** нет заранее подтверждённой межгрупповой зависимости; проверить actual consumers. Нумерация не разрешает fake foundation.

### W06-266. Cushion placement

**Сделать и доказать:** Предмет устанавливает реальную cushion entity с variant/components и опорой.

### W06-267. Посадка

**Сделать и доказать:** Interaction выполняет настоящий passenger mount с допустимостью цели и занятостью.

### W06-268. Положение пассажира

**Сделать и доказать:** Offset/pose и ориентация обновляются при движении и не проходят через collision блоков.

### W06-269. Высадка и damage

**Сделать и доказать:** Dismount, разрушение cushion и потеря опоры безопасно освобождают пассажира и создают нужный drop.

### W06-270. Persistence посадки

**Сделать и доказать:** Save/load/rejoin и два игрока не дают двойного rider или зависшей посадки в удалённую entity.

**Закрытие группы:** отчётить каждый из пяти результатов отдельно; partial/blocked не превращать в completed. Реально готовые ранее parts подтвердить, не переписывать.

## G055. End crystal

**Приоритет:** P1. **Карточки:** W06-271…W06-275.

**Связь:** W05-055.

**Сквозной объём направления:**

Эталон: EndCrystal, EndCrystalItem. Fight callbacks зависят от 078.
Замкнуть placement на настоящий explosion consumer и damage attribution.
Проверить требуемый блок, свободное место, chain explosion; fight hook не делать пустым.

**Проверенные Java-точки входа:** `world/entity/boss/enderdragon/EndCrystal.java`; `world/item/EndCrystalItem.java`

**Базовые зависимости:** нет заранее подтверждённой межгрупповой зависимости; проверить actual consumers. Нумерация не разрешает fake foundation.

### W06-271. Crystal placement

**Сделать и доказать:** Проверить требуемую поверхность, пространство и entity obstruction через actual item use.

### W06-272. Hit и detonation

**Сделать и доказать:** Damage уничтожает кристалл и вызывает общий explosion с корректным direct/indirect source.

### W06-273. Chain reaction

**Сделать и доказать:** Близкие кристаллы и TNT реагируют один раз; recursive callbacks не умножают drops или score credit.

### W06-274. Dragon fight linkage

**Сделать и доказать:** Реальный fight manager получает callbacks destruction/healing/ritual; без него эта часть остаётся открытой.

**Дополнительно требуется:** G078. Остальную независимую часть группы не блокировать.

### W06-275. Crystal persistence

**Сделать и доказать:** Beam target, flags и допустимый state переживают reload; command-spawned и item-placed кристаллы не путаются.

**Закрытие группы:** отчётить каждый из пяти результатов отдельно; partial/blocked не превращать в completed. Реально готовые ранее parts подтвердить, не переписывать.

## G056. Двери и люки

**Приоритет:** P1. **Карточки:** W06-276…W06-280.

**Связь:** W05-056.

**Сквозной объём направления:**

Эталон: DoorBlock, TrapDoorBlock.
Player/redstone use, половины, waterlogging, neighbors. Слом половины не оставляет
дюп, закрывание меняет collision и настоящий путь AI.

**Проверенные Java-точки входа:** `world/level/block/DoorBlock.java`; `world/level/block/TrapDoorBlock.java`

**Базовые зависимости:** нет заранее подтверждённой межгрупповой зависимости; проверить actual consumers. Нумерация не разрешает fake foundation.

### W06-276. Две половины двери

**Сделать и доказать:** Placement/rotation/hinge согласованы; missing half и break не оставляют самостоятельную копию предмета.

### W06-277. Ручное открытие

**Сделать и доказать:** Hand use учитывает материал двери, права и ожидаемые sound/game events.

### W06-278. Redstone control

**Сделать и доказать:** Смена signal обновляет OPEN/POWERED обеих половин в правильном порядке.

### W06-279. Trapdoor и вода

**Сделать и доказать:** Люк использует свои support/collision/waterlogging и сохраняет scheduled fluid updates.

### W06-280. Collision и pathfinding

**Сделать и доказать:** Открытое/закрытое состояние реально меняет проход entity и доступность пути моба.

**Закрытие группы:** отчётить каждый из пяти результатов отдельно; partial/blocked не превращать в completed. Реально готовые ранее parts подтвердить, не переписывать.

## G057. Кровати

**Приоритет:** P1. **Карточки:** W06-281…W06-285.

**Связь:** W05-057.

**Сквозной объём направления:**

Эталон: BedBlock, ServerPlayer.
Sleep/night skip/respawn и dimension explosion. Занятое/сломанное ложе,
безопасный respawn и отсутствие двойного взрыва двух половин.

**Проверенные Java-точки входа:** `world/level/block/BedBlock.java`; `server/level/ServerPlayer.java`

**Базовые зависимости:** нет заранее подтверждённой межгрупповой зависимости; проверить actual consumers. Нумерация не разрешает fake foundation.

### W06-281. Sleep interaction

**Сделать и доказать:** Кровать проверяет условия сна, occupancy и положение игрока в обеих половинах.

### W06-282. Ночь и погода

**Сделать и доказать:** Подходящая доля спящих игроков вызывает предусмотренный skip, сохраняя game time/domain ownership.

### W06-283. Respawn point

**Сделать и доказать:** Кровать устанавливает корректную точку возрождения и направление с проверкой safe destination.

### W06-284. Недопустимое измерение

**Сделать и доказать:** Use взрывает кровать через общий explosion consumer, без второго взрыва другой половины.

### W06-285. Слом и rejoin

**Сделать и доказать:** Destroyed/occupied bed, logout спящего и restart не оставляют ложный occupied state или неверный respawn.

**Закрытие группы:** отчётить каждый из пяти результатов отдельно; partial/blocked не превращать в completed. Реально готовые ранее parts подтвердить, не переписывать.

## G058. Якорь возрождения

**Приоритет:** P1. **Карточки:** W06-286…W06-290.

**Связь:** W05-058.

**Сквозной объём направления:**

Эталон: RespawnAnchorBlock, ServerPlayer.
Charge/use/respawn и explosion вне допустимого измерения. Off-hand glowstone,
max charge, расход и заблокированный respawn не теряют/дублируют заряд.

**Проверенные Java-точки входа:** `world/level/block/RespawnAnchorBlock.java`; `server/level/ServerPlayer.java`

**Базовые зависимости:** нет заранее подтверждённой межгрупповой зависимости; проверить actual consumers. Нумерация не разрешает fake foundation.

### W06-286. Charge interaction

**Сделать и доказать:** Glowstone увеличивает charge только до разрешённого предела и расходуется по source rules.

### W06-287. Use и respawn binding

**Сделать и доказать:** Main/off-hand приоритет выбирает зарядку либо установку respawn, не оба действия одновременно.

### W06-288. Расход после смерти

**Сделать и доказать:** Успешное возрождение уменьшает заряд в нужный момент; blocked spawn не даёт бесплатной телепортации.

### W06-289. Wrong-dimension explosion

**Сделать и доказать:** Неправильное измерение использует настоящий explosion calculator с нужными fluid особенностями.

### W06-290. Сохранение якоря

**Сделать и доказать:** Charge, client state и player respawn linkage переживают reload и разрушение блока.

**Закрытие группы:** отчётить каждый из пяти результатов отдельно; partial/blocked не превращать в completed. Реально готовые ранее parts подтвердить, не переписывать.

## G059. Лодки и плоты

**Приоритет:** P1. **Карточки:** W06-291…W06-295.

**Связь:** W05-059.

**Сквозной объём направления:**

Эталон: AbstractBoat, Boat.
Fluid states/buoyancy/rowing/passengers/collision с первым вариантом. Вода/суша,
падение, два пассажира и NBT; затем типизированно подключить остальные и chest variants.

**Проверенные Java-точки входа:** `world/entity/vehicle/boat/AbstractBoat.java`; `world/entity/vehicle/boat/Boat.java`

**Базовые зависимости:** нет заранее подтверждённой межгрупповой зависимости; проверить actual consumers. Нумерация не разрешает fake foundation.

### W06-291. Boat fluid states

**Сделать и доказать:** Buoyancy, submerged/underwater/land состояния определяются реальными fluid/block queries.

### W06-292. Управление и rowing

**Сделать и доказать:** Input пассажира влияет на acceleration/yaw/rowing, учитывая authoritative server state.

### W06-293. Passengers и collision

**Сделать и доказать:** Два rider, dismount position и столкновения с блоками/entities используют общие capabilities.

### W06-294. Варианты и chest boat

**Сделать и доказать:** Тип/предмет/components сохраняются; chest variant имеет живой menu/container, не декоративный field.

### W06-295. Damage и save

**Сделать и доказать:** Fall/break/drop и unload/load не теряют passengers/contents и не дублируют лодку.

**Закрытие группы:** отчётить каждый из пяти результатов отдельно; partial/blocked не превращать в completed. Реально готовые ранее parts подтвердить, не переписывать.

## G060. Вагонетки и рельсы

**Приоритет:** P1. **Карточки:** W06-296…W06-300.

**Связь:** W05-060.

**Сквозной объём направления:**

Эталон: AbstractMinecart, BaseRailBlock.
Rails/slopes/collision и существующий chest-container. Powered/activator rails,
unload/NBT; TNT/hopper/furnace variants требуют собственных consumers.

**Проверенные Java-точки входа:** `world/entity/vehicle/minecart/AbstractMinecart.java`; `world/level/block/BaseRailBlock.java`

**Базовые зависимости:** нет заранее подтверждённой межгрупповой зависимости; проверить actual consumers. Нумерация не разрешает fake foundation.

### W06-296. Rail traversal

**Сделать и доказать:** Minecart следует straight/curved/sloped rails с правильным position и velocity.

### W06-297. Powered и brake

**Сделать и доказать:** Powered rails, остановка и запуск меняют движение по Java, а не постоянной скоростью на всех tracks.

### W06-298. Activator/detector consumers

**Сделать и доказать:** Rail events влияют на поддерживаемые variants и comparator/redstone callbacks.

### W06-299. Container variants

**Сделать и доказать:** Chest/hopper/TNT/furnace варианты получают собственные реальные consumers; обычная minecart не подменяет их.

### W06-300. Persistence движения

**Сделать и доказать:** Chunk boundary, unload/load, passengers и breaking сохраняют содержимое и не создают duplicate UUID.

**Закрытие группы:** отчётить каждый из пяти результатов отдельно; partial/blocked не превращать в completed. Реально готовые ранее parts подтвердить, не переписывать.

## G061. Голод

**Приоритет:** P1. **Карточки:** W06-301…W06-305.

**Связь:** W05-061.

**Сквозной объём направления:**

Эталон: FoodData, Player.
Exhaustion/saturation/food/regen/starvation/sprint. Проверить difficulty,
freeze и сохранение; sync не отправляет отрицательные/переполненные значения.

**Проверенные Java-точки входа:** `world/food/FoodData.java`; `world/entity/player/Player.java`

**Базовые зависимости:** нет заранее подтверждённой межгрупповой зависимости; проверить actual consumers. Нумерация не разрешает fake foundation.

### W06-301. Food accounting

**Сделать и доказать:** Food level, saturation и exhaustion изменяются с правильными пределами и ванильным порядком.

### W06-302. Источники exhaustion

**Сделать и доказать:** Sprint/jump/swim/mining/attack и другие реальные actions вызывают свои consumers, не общий timer.

### W06-303. Natural regeneration

**Сделать и доказать:** Health восстанавливается с нужными food/saturation затратами и gamerule gate.

### W06-304. Starvation и difficulty

**Сделать и доказать:** Голод наносит допустимый урон с пределами по сложности и не работает вопреки frozen simulation.

### W06-305. Sync и save

**Сделать и доказать:** Player packets, respawn и disk persistence сохраняют согласованное food state без отрицательных значений.

**Закрытие группы:** отчётить каждый из пяти результатов отдельно; partial/blocked не превращать в completed. Реально готовые ранее parts подтвердить, не переписывать.

## G062. Еда и consumables

**Приоритет:** P1. **Карточки:** W06-306…W06-310.

**Связь:** W05-062.

**Сквозной объём направления:**

Эталон: Consumable, ItemStack.
Start/finish/cancel и реальные consume effects. Обе руки, тара, creative,
full hunger и смена слота; preview не применяет effects.

**Проверенные Java-точки входа:** `world/item/component/Consumable.java`; `world/item/ItemStack.java`

**Базовые зависимости:** нет заранее подтверждённой межгрупповой зависимости; проверить actual consumers. Нумерация не разрешает fake foundation.

### W06-306. Use eligibility

**Сделать и доказать:** Consumable проверяет hunger/always-consumable и выбранную руку до начала использования.

### W06-307. Duration и animation

**Сделать и доказать:** Tick/use/release и анимация соответствуют item components, а смена слота прекращает старый consumer.

### W06-308. Effects consumption

**Сделать и доказать:** Сытость, эффекты и реальные consume effects применяются при завершении, не при preview.

### W06-309. Remainder item

**Сделать и доказать:** Тара и consume remainder вставляются/выпадают корректно при полном inventory и creative.

### W06-310. Повтор packet и logout

**Сделать и доказать:** Finish callback, повторный use и disconnect не дают бесплатный effect или двойной расход.

**Закрытие группы:** отчётить каждый из пяти результатов отдельно; partial/blocked не превращать в completed. Реально готовые ранее parts подтвердить, не переписывать.

## G063. Эффекты

**Приоритет:** P1. **Карточки:** W06-311…W06-315.

**Связь:** W05-063.

**Сквозной объём направления:**

Эталон: MobEffectInstance, LivingEntity.
Instant/stacking/duration/milk/honey и повторное наложение. Weaker/stronger,
immunity/death/reload и particles; custom effects не теряются в components.

**Проверенные Java-точки входа:** `world/effect/MobEffectInstance.java`; `world/entity/LivingEntity.java`

**Базовые зависимости:** нет заранее подтверждённой межгрупповой зависимости; проверить actual consumers. Нумерация не разрешает fake foundation.

### W06-311. Effect merge

**Сделать и доказать:** Stronger/weaker/same-amplifier эффекты соединяются по Java, включая скрытую цепочку durations.

### W06-312. Tick effects

**Сделать и доказать:** Poison/regeneration/прочие periodic consumers вызываются с правильной частотой и eligibility.

### W06-313. Instant effects

**Сделать и доказать:** Instant heal/harm и attribution применяются один раз в нужном масштабе для конкретного source.

### W06-314. Removal effects

**Сделать и доказать:** Milk/honey/totem/death снимают только предусмотренные effects и обновляют attributes.

### W06-315. Persistence и sync

**Сделать и доказать:** Effect particles/ambient/invisibility и attributes остаются согласованы после reload и повторного наложения.

**Закрытие группы:** отчётить каждый из пяти результатов отдельно; partial/blocked не превращать в completed. Реально готовые ранее parts подтвердить, не переписывать.

## G064. Щит

**Приоритет:** P1. **Карточки:** W06-316…W06-320.

**Связь:** W05-064.

**Сквозной объём направления:**

Эталон: ShieldItem, LivingEntity, ItemStack.
BLOCKS_ATTACKS текущей версии, угол, damage tags, disable cooldown, обе руки,
durability и порядок reductions; старые константы 1.21 не переносить из памяти.

**Проверенные Java-точки входа:** `world/item/ShieldItem.java`; `world/entity/LivingEntity.java`; `world/item/ItemStack.java`

**Базовые зависимости:** нет заранее подтверждённой межгрупповой зависимости; проверить actual consumers. Нумерация не разрешает fake foundation.

### W06-316. BlocksAttacks data

**Сделать и доказать:** Shield behavior читает настоящие components/tags 26.3, не старый список фиксированных исключений.

### W06-317. Положение и угол

**Сделать и доказать:** Incoming source position, направление взгляда и use hand определяют допустимое блокирование.

### W06-318. Damage reduction order

**Сделать и доказать:** Блокирование входит в верный этап до остальных reductions, сохраняя источник и фактически принятый урон.

### W06-319. Disable и durability

**Сделать и доказать:** Axe/effect cooldown, wear и break events работают на реальном shield consumer.

### W06-320. Пограничные hits

**Сделать и доказать:** Projectile/melee/explosion, rear hit, creative и interrupted use проверяются отдельными игровыми сценариями.

**Закрытие группы:** отчётить каждый из пяти результатов отдельно; partial/blocked не превращать в completed. Реально готовые ранее parts подтвердить, не переписывать.

## G065. Ближний бой

**Приоритет:** P1. **Карточки:** W06-321…W06-325.

**Связь:** W05-065.

**Сквозной объём направления:**

Эталон: Player, LivingEntity. Зависимость: 064.
Cooldown/crit/sweep/knockback/armor/enchants/source. Sprint attack, creative,
spectator, teammate policy и два игрока; общий hurt_server не обходить.

**Проверенные Java-точки входа:** `world/entity/player/Player.java`; `world/entity/LivingEntity.java`

**Базовые зависимости:** нет заранее подтверждённой межгрупповой зависимости; проверить actual consumers. Нумерация не разрешает fake foundation.

### W06-321. Attack cooldown

**Сделать и доказать:** Сила удара определяется настоящим attack speed/cooldown, а не каждым packet как полным ударом.

### W06-322. Critical и sweeping

**Сделать и доказать:** Условия crit/sweep и список вторичных целей соответствуют movement/equipment состоянию.

### W06-323. Knockback

**Сделать и доказать:** Sprint/enchant/attribute сопротивления меняют impulse по Java, включая прекращение sprint.

### W06-324. Armor и enchants

**Сделать и доказать:** Damage source проходит общий reduction/effects pipeline и повреждает правильную экипировку.

### W06-325. Targets и attribution

**Сделать и доказать:** Spectator/creative/allied targets, kill credit и два игрока проверены без прямого bypass hurt_server.

**Дополнительно требуется:** G086. Остальную независимую часть группы не блокировать.

**Закрытие группы:** отчётить каждый из пяти результатов отдельно; partial/blocked не превращать в completed. Реально готовые ранее parts подтвердить, не переписывать.

## G066. Урон среды

**Приоритет:** P1. **Карточки:** W06-326…W06-330.

**Связь:** W05-066.

**Сквозной объём направления:**

Эталон: LivingEntity, Entity.
Fall/fire/lava/drown/freeze/suffocate и порядок ticks. Иммунитеты/totem/armor,
вход в воду; upstream splash/hurt timers сохраняются, прямой periodic hurt недостаточен.

**Проверенные Java-точки входа:** `world/entity/LivingEntity.java`; `world/entity/Entity.java`

**Базовые зависимости:** нет заранее подтверждённой межгрупповой зависимости; проверить actual consumers. Нумерация не разрешает fake foundation.

### W06-326. Fall damage

**Сделать и доказать:** Накопление fall distance, landing block и water entry формируют правильный fall damage и particles.

### W06-327. Fire и lava

**Сделать и доказать:** Fire timer, extinguish, lava contact и immunity дают верный порядок damage/effects.

### W06-328. Drowning и suffocation

**Сделать и доказать:** Eye-fluid state, air supply и blocked breathing работают через реальные environmental queries.

### W06-329. Freezing

**Сделать и доказать:** Powder snow, protective equipment и freeze ticks учитывают допустимые entity tags.

### W06-330. Totem и lifecycle

**Сделать и доказать:** Environment lethal damage использует тот же death protection, death loot и reset при новой жизни.

**Закрытие группы:** отчётить каждый из пяти результатов отдельно; partial/blocked не превращать в completed. Реально готовые ранее parts подтвердить, не переписывать.

## G067. Смерть и respawn

**Приоритет:** P1. **Карточки:** W06-331…W06-335.

**Связь:** W05-067.

**Сквозной объём направления:**

Эталон: ServerPlayer, PlayerList. Зависимости: 024, 049.
KeepInventory/XP/drop/messages/cleanup menus/pearls. Две смерти и смена мира
не дублируют loot; новая жизнь не сохраняет death guard или старые callbacks.

**Проверенные Java-точки входа:** `server/level/ServerPlayer.java`; `server/players/PlayerList.java`

**Базовые зависимости:** нет заранее подтверждённой межгрупповой зависимости; проверить actual consumers. Нумерация не разрешает fake foundation.

### W06-331. Death transition

**Сделать и доказать:** Летальный hurt запускает death side effects один раз, с правильными messages, stats и attribution.

### W06-332. Inventory и keepInventory

**Сделать и доказать:** Предметы/equipment/XP выпадают либо сохраняются по gamerules без копии живого inventory в drop.

### W06-333. Menu cleanup

**Сделать и доказать:** При смерти закрываются все active menus, cursor/input leftovers возвращаются или выпадают один раз.

### W06-334. Respawn recreation

**Сделать и доказать:** Новая жизнь получает допустимые поля и clearing старых hurt/death/use/pearl callbacks.

### W06-335. Повторная смерть

**Сделать и доказать:** Death→respawn→death и world/domain transfer не дублируют сохранённый loot и не используют старые owners.

**Закрытие группы:** отчётить каждый из пяти результатов отдельно; partial/blocked не превращать в completed. Реально готовые ранее parts подтвердить, не переписывать.

## G068. Опыт и mending

**Приоритет:** P1. **Карточки:** W06-336…W06-340.

**Связь:** W05-068.

**Сквозной объём направления:**

Эталон: ExperienceOrb, Player.
Merge/pickup delay/mending/level conversion/orb values. Несколько игроков,
полный ремонт и остаток XP; сохранить проверенный разброс опыта бутылки.

**Проверенные Java-точки входа:** `world/entity/ExperienceOrb.java`; `world/entity/player/Player.java`

**Базовые зависимости:** нет заранее подтверждённой межгрупповой зависимости; проверить actual consumers. Нумерация не разрешает fake foundation.

### W06-336. Orb values и spawn

**Сделать и доказать:** XP amounts раскладываются на реальные orb values и motion по Java, сохраняя предыдущие bottle fixes.

### W06-337. Merge behavior

**Сделать и доказать:** Подходящие nearby orbs объединяются по нужным правилам identity/value/age, не все XP в радиусе безусловно.

### W06-338. Pickup и delay

**Сделать и доказать:** Player pickup учитывает cooldown, eligibility и правильную выдачу опыта двум конкурентным игрокам.

### W06-339. Mending

**Сделать и доказать:** Выбор repairable equipment, repair amount и остаток XP следуют enchant consumer и RNG order.

### W06-340. Levels и persistence

**Сделать и доказать:** XP points/levels/progress и orb state переживают save/rejoin без потери или повторного получения.

**Закрытие группы:** отчётить каждый из пяти результатов отдельно; partial/blocked не превращать в completed. Реально готовые ранее parts подтвердить, не переписывать.

## G069. Natural spawning

**Приоритет:** P2. **Карточки:** W06-341…W06-345.

**Связь:** W05-069.

**Сквозной объём направления:**

Эталон: NaturalSpawner. Зависимости: реальные mobs 071–075.
Caps/distance/light/biome/structure rules. Passive/hostile группы в загруженных
чанках; отсутствие AI не скрывать успешным spawn count.

**Проверенные Java-точки входа:** `world/level/NaturalSpawner.java`

**Базовые зависимости:** нет заранее подтверждённой межгрупповой зависимости; проверить actual consumers. Нумерация не разрешает fake foundation.

### W06-341. Mob categories и caps

**Сделать и доказать:** NaturalSpawner считает правильные группы и population caps по активным чанкам/игрокам.

### W06-342. Spawn placement

**Сделать и доказать:** Biome, block/fluid/light/height и entity-specific placement predicates используют реальные extracted rules.

### W06-343. Расстояния игроков

**Сделать и доказать:** Spawn exclusion, spectator и несколько игроков не превращаются в один произвольный radius check.

### W06-344. Group initialization

**Сделать и доказать:** Pack/group data, difficulty и initial equipment/state передаются настоящим mobs.

### W06-345. Доказуемый consumer

**Сделать и доказать:** Пассивная и hostile группы реально появляются и ведут себя в мире; пустой AI не скрывается числом spawns.

**Закрытие группы:** отчётить каждый из пяти результатов отдельно; partial/blocked не превращать в completed. Реально готовые ранее parts подтвердить, не переписывать.

## G070. Despawn и persistence

**Приоритет:** P2. **Карточки:** W06-346…W06-350.

**Связь:** W05-070.

**Сквозной объём направления:**

Эталон: Mob.
Distance/no-action/named/tamed/from-bucket. Два игрока, spectator, unload/reload
и mob с equipment из раздатчика; persistent mob не исчезает из-за общего despawn helper.

**Проверенные Java-точки входа:** `world/entity/Mob.java`

**Базовые зависимости:** нет заранее подтверждённой межгрупповой зависимости; проверить actual consumers. Нумерация не разрешает fake foundation.

### W06-346. Despawn predicates

**Сделать и доказать:** Расстояния, randomness и no-action time различают immediate и probabilistic removal по source.

### W06-347. Persistent exemptions

**Сделать и доказать:** Named/tamed/player-created/from-bucket и equipped mobs не удаляются вопреки своим flags.

### W06-348. Player selection

**Сделать и доказать:** Несколько игроков, spectators и пустой мир учитываются Java способом, а не первым online player.

### W06-349. Unload и retention

**Сделать и доказать:** Chunk unload сохраняет entity вместо ошибочного killed removal; загрузка не сбрасывает persistence.

### W06-350. Длительный прогон

**Сделать и доказать:** Despawn/reload/save и исчезновение ближайшего игрока не создают ghosts или duplicate UUID.

**Закрытие группы:** отчётить каждый из пяти результатов отдельно; partial/blocked не превращать в completed. Реально готовые ранее parts подтвердить, не переписывать.

## G071. Зомби

**Приоритет:** P2. **Карточки:** W06-351…W06-355.

**Связь:** W05-071.

**Сквозной объём направления:**

Эталон: Zombie. Зависимость: 065.
Movement/target/melee/sun/baby/NBT. Реальное преследование, преграда,
daylight и death loot; constructor/TODO-only factory не готова.

**Проверенные Java-точки входа:** `world/entity/monster/zombie/Zombie.java`

**Базовые зависимости:** нет заранее подтверждённой межгрупповой зависимости; проверить actual consumers. Нумерация не разрешает fake foundation.

### W06-351. Zombie locomotion

**Сделать и доказать:** Реальный zombie использует navigation/movement controllers, размеры и obstacles.

### W06-352. Target и melee

**Сделать и доказать:** Выбор допустимого игрока и attack goal приводят к настоящему cooldown/damage consumer.

### W06-353. Sun и environment

**Сделать и доказать:** Daylight burning, helmet/equipment и water-related состояния сверены с Java.

### W06-354. Возраст и variants

**Сделать и доказать:** Baby/variant/поддерживаемые conversion fields сохраняются и влияют на actual gameplay.

### W06-355. Death и reload

**Сделать и доказать:** Loot/equipment/attributes и AI state переживают lifecycle; factory-only zombie не закрывает карточку.

**Закрытие группы:** отчётить каждый из пяти результатов отдельно; partial/blocked не превращать в completed. Реально готовые ранее parts подтвердить, не переписывать.

## G072. Скелет

**Приоритет:** P2. **Карточки:** W06-356…W06-360.

**Связь:** W05-072.

**Сквозной объём направления:**

Эталон: Skeleton, AbstractSkeleton. Зависимости: 041, 044.
Ranged AI с настоящими arrows/equipment/sensing. Line of sight, дистанция,
смена оружия, owner/source и NBT; стрелу не имитировать прямым hurt.

**Проверенные Java-точки входа:** `world/entity/monster/skeleton/Skeleton.java`; `world/entity/monster/skeleton/AbstractSkeleton.java`

**Базовые зависимости:** нет заранее подтверждённой межгрупповой зависимости; проверить actual consumers. Нумерация не разрешает fake foundation.

### W06-356. Skeleton navigation

**Сделать и доказать:** Дистанция до цели и line of sight регулируют движение и ranged goal.

### W06-357. Настоящий выстрел

**Сделать и доказать:** Bow goal создаёт Arrow с owner, скоростью и source scaling, а не direct hurt.

**Дополнительно требуется:** G041, G044. Остальную независимую часть группы не блокировать.

### W06-358. Смена оружия

**Сделать и доказать:** Melee/ranged goal обновляется при изменении equipment и не оставляет два одновременно активных attack consumers.

### W06-359. Species state

**Сделать и доказать:** Поддерживаемые variants/environment effects используют собственные данные и rules, не только skin.

### W06-360. Persistence и бой

**Сделать и доказать:** Reload, target loss, daylight и kill attribution сохраняют корректное поведение.

**Закрытие группы:** отчётить каждый из пяти результатов отдельно; partial/blocked не превращать в completed. Реально готовые ранее parts подтвердить, не переписывать.

## G073. Крипер

**Приоритет:** P2. **Карточки:** W06-361…W06-365.

**Связь:** W05-073.

**Сквозной объём направления:**

Эталон: Creeper.
Swelling/cancel/ignition/charged/explosion. MobGriefing, дистанция, смерть до
detonation и special drops; TNT tick не заменяет creeper AI.

**Проверенные Java-точки входа:** `world/entity/monster/Creeper.java`

**Базовые зависимости:** нет заранее подтверждённой межгрупповой зависимости; проверить actual consumers. Нумерация не разрешает fake foundation.

### W06-361. Swelling state

**Сделать и доказать:** Creeper начинает, продолжает и сбрасывает fuse по distance/goal/ignition условиям.

### W06-362. Detonation

**Сделать и доказать:** Завершение fuse вызывает общий explosion engine с правильными radius, source и mobGriefing.

### W06-363. Charged state

**Сделать и доказать:** Lightning/charged flags реально меняют blast и разрешённые special drops.

### W06-364. Прерывание

**Сделать и доказать:** Смерть, удаление и потеря цели до detonation не вызывают отложенный лишний взрыв.

### W06-365. Save и special loot

**Сделать и доказать:** Fuse/ignition/charged state и kill-based drops сохраняются и не выполняются дважды.

**Закрытие группы:** отчётить каждый из пяти результатов отдельно; partial/blocked не превращать в completed. Реально готовые ранее parts подтвердить, не переписывать.

## G074. Паук

**Приоритет:** P2. **Карточки:** W06-366…W06-370.

**Связь:** W05-074.

**Сквозной объём направления:**

Эталон: Spider.
Climbing/jump/target/daylight/sizes. Стена, потолок, узкий проход и прекращение
преследования; движение через общий collision, без teleport к игроку.

**Проверенные Java-точки входа:** `world/entity/monster/spider/Spider.java`

**Базовые зависимости:** нет заранее подтверждённой межгрупповой зависимости; проверить actual consumers. Нумерация не разрешает fake foundation.

### W06-366. Climbing

**Сделать и доказать:** Spider collision со стеной обновляет climbing state и реальное перемещение вверх.

### W06-367. Jump и attack

**Сделать и доказать:** Прыжок/attack goals используют navigation и reachable target, без teleport через препятствие.

### W06-368. Daylight targeting

**Сделать и доказать:** Условия освещения влияют на приобретение/потерю цели как в Java.

### W06-369. Размеры и path

**Сделать и доказать:** Низкие/узкие проходы, потолок и entity bounding box действительно ограничивают маршрут.

### W06-370. Persistence и variants

**Сделать и доказать:** Save/load/effects и поддерживаемые subtype rules не теряются в generic spider factory.

**Закрытие группы:** отчётить каждый из пяти результатов отдельно; partial/blocked не превращать в completed. Реально готовые ранее parts подтвердить, не переписывать.

## G075. Эндермен

**Приоритет:** P2. **Карточки:** W06-371…W06-375.

**Связь:** W05-075.

**Сквозной объём направления:**

Эталон: Enderman.
Взгляд/агрессия/teleport/вода/held block. MobGriefing, запрещённые блоки,
safe destination, damage и NBT; не ставить произвольный блок вместо extracted rules.

**Проверенные Java-точки входа:** `world/entity/monster/Enderman.java`

**Базовые зависимости:** нет заранее подтверждённой межгрупповой зависимости; проверить actual consumers. Нумерация не разрешает fake foundation.

### W06-371. Aggro от взгляда

**Сделать и доказать:** Enderman проверяет взгляд, equipment exemptions и target acquisition через реальное зрение.

### W06-372. Teleport destinations

**Сделать и доказать:** Допустимая поверхность, world bounds и space/fluid checks проверяются до перемещения.

### W06-373. Water и damage reactions

**Сделать и доказать:** Вода/projectile/другие источники вызывают свои реакции и не обходят damage attribution.

### W06-374. Перенос блоков

**Сделать и доказать:** Pickup/place используют реальные allowed block tags и mobGriefing, не ручной список из памяти.

### W06-375. Persistent held state

**Сделать и доказать:** Held block, target/anger и reload не создают бесплатный второй блок или неправильный owner.

**Закрытие группы:** отчётить каждый из пяти результатов отдельно; partial/blocked не превращать в completed. Реально готовые ранее parts подтвердить, не переписывать.

## G076. Жители

**Приоритет:** P2. **Карточки:** W06-376…W06-380.

**Связь:** W05-076.

**Сквозной объём направления:**

Эталон: Villager, MerchantMenu.
Profession/POI/trades/demand/restock и merchant inventory. Два игрока,
исчерпание сделки, репутация/NBT; сначала закончить нужные POI/Brain foundations.

**Проверенные Java-точки входа:** `world/entity/npc/villager/Villager.java`; `world/inventory/MerchantMenu.java`

**Базовые зависимости:** нет заранее подтверждённой межгрупповой зависимости; проверить actual consumers. Нумерация не разрешает fake foundation.

### W06-376. Profession и POI

**Сделать и доказать:** Villager связывает реальный profession/job-site/дом и допустимые Brain consumers.

### W06-377. Trade offers

**Сделать и доказать:** Offers содержат правильные items/components, counts, uses и merchant-specific условия.

### W06-378. Merchant menu

**Сделать и доказать:** Два игрока и смена offer проходят авторитетный inventory path, не client-calculated result.

### W06-379. Demand и restock

**Сделать и доказать:** Использование trades, timing и доступ к job-site реально обновляют цены/uses.

### W06-380. Save и reputation

**Сделать и доказать:** Profession/experience/offers/reputation переживают reload; отсутствующий Brain не заменять пустыми callbacks.

**Закрытие группы:** отчётить каждый из пяти результатов отдельно; partial/blocked не превращать в completed. Реально готовые ранее parts подтвердить, не переписывать.

## G077. Големы

**Приоритет:** P2. **Карточки:** W06-381…W06-385.

**Связь:** W05-077.

**Сквозной объём направления:**

Эталон: IronGolem, SnowGolem. Связь: 014.
Создание конструкцией и полноценный первый gameplay slice каждого вида.
Player-created flag, targets, снег/среда и death loot; constructor-only mob недостаточен.

**Проверенные Java-точки входа:** `world/entity/animal/golem/IronGolem.java`; `world/entity/animal/golem/SnowGolem.java`

**Базовые зависимости:** нет заранее подтверждённой межгрупповой зависимости; проверить actual consumers. Нумерация не разрешает fake foundation.

### W06-381. Snow golem creation

**Сделать и доказать:** Pattern consumer создаёт одну живую сущность и удаляет ровно нужные блоки.

### W06-382. Snow golem gameplay

**Сделать и доказать:** Movement/targets/projectiles/snow trail и environment damage используют настоящие consumers.

### W06-383. Iron golem creation

**Сделать и доказать:** Player-created flag и допустимый pattern/state передаются реальному spawn.

### W06-384. Iron golem combat

**Сделать и доказать:** Target selection, melee, knockback и village/player-created exceptions соответствуют Java.

### W06-385. Persistence големов

**Сделать и доказать:** Save/load, kill attribution и drops сохраняют происхождение и не повторяют pattern consumption.

**Закрытие группы:** отчётить каждый из пяти результатов отдельно; partial/blocked не превращать в completed. Реально готовые ранее parts подтвердить, не переписывать.

## G078. Дракон и End fight

**Приоритет:** P2. **Карточки:** W06-386…W06-390.

**Связь:** W05-078.

**Сквозной объём направления:**

Эталон: EnderDragonFight, EnderDragon.
Manager/crystals/ritual/bossbar/gateways с настоящими consumers. Death/rejoin/
restart/repeat ritual; отсутствующая фаза не имитируется пустым hook ради EndCrystal.

**Проверенные Java-точки входа:** `world/level/dimension/end/EnderDragonFight.java`; `world/entity/boss/enderdragon/EnderDragon.java`

**Базовые зависимости:** нет заранее подтверждённой межгрупповой зависимости; проверить actual consumers. Нумерация не разрешает fake foundation.

### W06-386. Fight manager

**Сделать и доказать:** Реальный EnderDragonFight хранит dragon/crystals/portal/respawn stage, не набор disconnected flags.

### W06-387. Dragon phases

**Сделать и доказать:** Поддерживаемые фазы управляют настоящим движением, multipart damage и attack consumers.

### W06-388. Crystals и healing

**Сделать и доказать:** Crystal damage/healing/beam и уничтожение проходят связь с manager и dragon.

### W06-389. Ritual и gateways

**Сделать и доказать:** Respawn ritual, exit portal и gateway creation реально изменяют мир и имеют правильные условия.

### W06-390. Restart и повторный fight

**Сделать и доказать:** Reload/rejoin/death и второй ritual не создают второго boss или потерянную bossbar.

**Закрытие группы:** отчётить каждый из пяти результатов отдельно; partial/blocked не превращать в completed. Реально готовые ранее parts подтвердить, не переписывать.

## G079. Водные существа

**Приоритет:** P2. **Карточки:** W06-391…W06-395.

**Связь:** W05-079.

**Сквозной объём направления:**

Эталон: AbstractFish, Squid. Связь: 008, 069.
Bucket species и aquatic movement/spawn/air. Выпуск/подбор, суша,
player distance/NBT; видимая неподвижная сущность не равна работающему AI.

**Проверенные Java-точки входа:** `world/entity/animal/fish/AbstractFish.java`; `world/entity/animal/squid/Squid.java`

**Базовые зависимости:** нет заранее подтверждённой межгрупповой зависимости; проверить actual consumers. Нумерация не разрешает fake foundation.

### W06-391. Aquatic locomotion

**Сделать и доказать:** Выбранные fish/squid species используют настоящие water/air/movement rules.

### W06-392. Spawn predicates

**Сделать и доказать:** Fluid/height/biome/light условия и caps подключены к real spawner.

### W06-393. Species lifecycle

**Сделать и доказать:** Возраст, variant и species-specific states влияют на поведение, не только на metadata.

### W06-394. Bucket consumer

**Сделать и доказать:** Поддерживаемые виды реально подбираются и выпускаются с собственным NBT/from-bucket состоянием.

### W06-395. Persistence и суша

**Сделать и доказать:** Damage на суше, despawn и reload сохраняют правильные exemptions и contents ведра.

**Закрытие группы:** отчётить каждый из пяти результатов отдельно; partial/blocked не превращать в completed. Реально готовые ранее parts подтвердить, не переписывать.

## G080. Приручаемые существа

**Приоритет:** P2. **Карточки:** W06-396…W06-400.

**Связь:** W05-080.

**Сквозной объём направления:**

Эталон: TamableAnimal, Wolf, Cat.
Tame/owner/sit/follow/breed/species equipment. Offline owner, смена мира,
friendly damage и NBT; UUID owner не превращать в сильную циклическую ссылку.

**Проверенные Java-точки входа:** `world/entity/TamableAnimal.java`; `world/entity/animal/wolf/Wolf.java`; `world/entity/animal/feline/Cat.java`

**Базовые зависимости:** нет заранее подтверждённой межгрупповой зависимости; проверить actual consumers. Нумерация не разрешает fake foundation.

### W06-396. Taming

**Сделать и доказать:** Предмет/interaction/RNG приводят к реальному tame state с owner identity.

### W06-397. Sit и follow

**Сделать и доказать:** Приказ сидеть и following owner реально переключают goals и navigation.

### W06-398. Breeding

**Сделать и доказать:** Food/love/age/offspring используют общую animal foundation с species-specific rules.

### W06-399. Equipment и combat

**Сделать и доказать:** Wolf/cat variants и допустимая экипировка, owner/allied damage и targeting используют реальные capabilities.

**Дополнительно требуется:** G086. Остальную независимую часть группы не блокировать.

### W06-400. Owner lifecycle

**Сделать и доказать:** Offline owner, respawn, domain transfer и reload не создают сильный reference cycle или чужого владельца.

**Закрытие группы:** отчётить каждый из пяти результатов отдельно; partial/blocked не превращать в completed. Реально готовые ранее parts подтвердить, не переписывать.

## G081. Хранилище карт

**Приоритет:** P2. **Карточки:** W06-401…W06-405.

**Связь:** W05-081.

**Сквозной объём направления:**

Эталон: MapItemSavedData, MapItem.
Manager/IDs/dimensions/colors/dirty regions и реальные map-data packets.
Два items одного ID, отдельные карты и reload сохраняют правильное общее состояние.

**Проверенные Java-точки входа:** `world/level/saveddata/maps/MapItemSavedData.java`; `world/item/MapItem.java`

**Базовые зависимости:** нет заранее подтверждённой межгрупповой зависимости; проверить actual consumers. Нумерация не разрешает fake foundation.

### W06-401. Map IDs и storage

**Сделать и доказать:** Настоящий manager создаёт уникальные map IDs и saved data в нужной scope.

### W06-402. Terrain colors

**Сделать и доказать:** Обновление colors читает реальные biome/block/fluid данные с Java sampling rules.

### W06-403. Dirty regions

**Сделать и доказать:** Изменения ограничивают отправляемую область и не теряют пиксели при последовательных updates.

### W06-404. Map-data packets

**Сделать и доказать:** Реальный clientbound consumer передаёт colors/decorations/scale/locked с extracted protocol refs.

### W06-405. Общее состояние

**Сделать и доказать:** Два items одного ID делят одну карту; разные IDs и restart не смешивают содержимое.

**Закрытие группы:** отчётить каждый из пяти результатов отдельно; partial/blocked не превращать в completed. Реально готовые ранее parts подтвердить, не переписывать.

## G082. Карты в руках и рамках

**Приоритет:** P2. **Карточки:** W06-406…W06-410.

**Связь:** W05-082.

**Сквозной объём направления:**

Эталон: ItemFrame, MapItem. Зависимость: 081; связь N18.
Decorations/tracking/rotation/remove/лимит рамок. Обычная/glow рамка,
удар/разрушение, несколько игроков и restart; пустой tracking hook запрещён.

**Проверенные Java-точки входа:** `world/entity/decoration/ItemFrame.java`; `world/item/MapItem.java`

**Базовые зависимости:** G081. Нумерация не разрешает fake foundation.

### W06-406. Карта в руке

**Сделать и доказать:** Отслеживание holder и обновления соответствуют dimension/scale и inventory presence.

### W06-407. Обычная рамка

**Сделать и доказать:** ItemFrame создаёт настоящую framed decoration и tracking entry при помещении карты.

### W06-408. Glow рамка

**Сделать и доказать:** Subtype использует тот же manager с правильными remove/rotation callbacks.

### W06-409. Лимит и replacement

**Сделать и доказать:** Ограничение framed maps и замена item работают без утечки старых entries.

### W06-410. Break и restart

**Сделать и доказать:** Удар, разрушение, unload/load и несколько игроков не оставляют phantom decorations или дубликаты.

**Закрытие группы:** отчётить каждый из пяти результатов отдельно; partial/blocked не превращать в completed. Реально готовые ранее parts подтвердить, не переписывать.

## G083. Картографический стол

**Приоритет:** P2. **Карточки:** W06-411…W06-415.

**Связь:** W05-083.

**Сквозной объём направления:**

Эталон: CartographyTableMenu, MapItem. Зависимости: 081, 037.
Scale/lock/clone/postprocess на настоящих данных. Output/shift/remainders,
общее ID у копий; preview не изменяет saved map.

**Проверенные Java-точки входа:** `world/inventory/CartographyTableMenu.java`; `world/item/MapItem.java`

**Базовые зависимости:** G081. Нумерация не разрешает fake foundation.

### W06-411. Cartography inputs

**Сделать и доказать:** Menu выбирает scale/clone/lock только для допустимых combinations и настоящей map data.

### W06-412. Scale result

**Сделать и доказать:** Успешный take создаёт/обновляет допустимый scaled map с правильными center и ID rules.

### W06-413. Lock и clone

**Сделать и доказать:** Lock сохраняет terrain snapshot; копии делят или получают ID ровно по Java.

### W06-414. Crafting postprocess

**Сделать и доказать:** Result callbacks выполняются после actual take, preview не мутирует saved maps.

### W06-415. Sync и overflow

**Сделать и доказать:** Shift/hotbar/close/full inventory не дублируют map или consume inputs повторно.

**Закрытие группы:** отчётить каждый из пяти результатов отдельно; partial/blocked не превращать в completed. Реально готовые ранее parts подтвердить, не переписывать.

## G084. Знамёна

**Приоритет:** P2. **Карточки:** W06-416…W06-420.

**Связь:** W05-084.

**Сквозной объём направления:**

Эталон: BannerBlockEntity, BannerItem, LoomMenu.
Placement/components/patterns/washing/NBT. Item→block→pick/drop и pattern limits;
сохранить loom selection/tags, не придумывать зависимость loom от recipe book.

**Проверенные Java-точки входа:** `world/level/block/entity/BannerBlockEntity.java`; `world/item/BannerItem.java`; `world/inventory/LoomMenu.java`

**Базовые зависимости:** нет заранее подтверждённой межгрупповой зависимости; проверить actual consumers. Нумерация не разрешает fake foundation.

### W06-416. Banner placement

**Сделать и доказать:** Item components переходят в block entity с правильной базой, rotation/facing и support.

### W06-417. Patterns

**Сделать и доказать:** Pattern list, colors и limits используют generated registry/tags и компоненты предмета.

### W06-418. Loom result

**Сделать и доказать:** Сохранить сделанный selection; применять pattern и расход inputs только при настоящем take.

### W06-419. Washing и interaction

**Сделать и доказать:** Cauldron/прочие предусмотренные source consumers изменяют нужный последний pattern, не весь предмет.

### W06-420. Round-trip

**Сделать и доказать:** Placement/pick/drop/reload сохраняют banner data и не возвращают старые implicit components.

**Закрытие группы:** отчётить каждый из пяти результатов отдельно; partial/blocked не превращать в completed. Реально готовые ранее parts подтвердить, не переписывать.

## G085. Sculk

**Приоритет:** P2. **Карточки:** W06-421…W06-425.

**Связь:** W05-085.

**Сквозной объём направления:**

Эталон: SculkSensorBlockEntity, CalibratedSculkSensorBlock, VibrationSystem.
Delay/occlusion/frequency поверх существующих game events. Wool, расстояние,
calibrated side/cooldown; listener без настоящего tick/delay не закрывает систему.

**Проверенные Java-точки входа:** `world/level/block/entity/SculkSensorBlockEntity.java`; `world/level/block/CalibratedSculkSensorBlock.java`; `world/level/gameevent/vibrations/VibrationSystem.java`

**Базовые зависимости:** нет заранее подтверждённой межгрупповой зависимости; проверить actual consumers. Нумерация не разрешает fake foundation.

### W06-421. Game-event intake

**Сделать и доказать:** Sculk consumer принимает только подходящие events и source context через существующий dispatcher.

### W06-422. Propagation delay

**Сделать и доказать:** Distance/tick delay и destination остаются настоящим состоянием, не немедленным callback.

### W06-423. Occlusion

**Сделать и доказать:** Wool/shape/path проверки блокируют нужные вибрации, сохраняя позицию и source.

### W06-424. Sensor state

**Сделать и доказать:** Frequency, power, active/cooldown и calibrated input управляют реальным redstone output.

### W06-425. Persistent vibration

**Сделать и доказать:** Reload и source removal не создают повторную доставку или постоянный active sensor.

**Закрытие группы:** отчётить каждый из пяти результатов отдельно; partial/blocked не превращать в completed. Реально готовые ранее parts подтвердить, не переписывать.

## G086. Teams и friendly fire

**Приоритет:** P0. **Карточки:** W06-426…W06-430.

**Связь:** W05-086.

**Сквозной объём направления:**

Эталон: Scoreboard, ServerScoreboard, Entity.
Довести team properties и runtime gameplay read access к доменной scoreboard.
FriendlyFire=false защищает союзника от blast/melee, knockback идёт по Java; domains не смешиваются.
Архитектуру доступа согласовать по AGENTS после проверки существующих command contexts.

**Проверенные Java-точки входа:** `world/scores/Scoreboard.java`; `server/ServerScoreboard.java`; `world/entity/Entity.java`

**Базовые зависимости:** нет заранее подтверждённой межгрупповой зависимости; проверить actual consumers. Нумерация не разрешает fake foundation.

### W06-426. Team properties

**Сделать и доказать:** Scoreboard хранит friendlyFire и другие нужные gameplay properties с корректным persistence.

### W06-427. Read architecture

**Сделать и доказать:** После проверки command contexts согласовать runtime доступ к доменной scoreboard без World→Server shortcut.

**Decision gate:** runtime read architecture не согласована этим ТЗ. Сначала проверить существующий доступ и задать узкий вопрос по AGENTS.

### W06-428. Alliance predicate

**Сделать и доказать:** Same/allied/no-team cases дают реальный doTeamsAllowDamage результат для нужных source entities.

### W06-429. Combat consumers

**Сделать и доказать:** Melee/projectile/explosion учитывают predicate в Java месте; knockback не отключать вместе с damage без сверки.

### W06-430. Domains и reload

**Сделать и доказать:** Разные scoreboard scopes, player transfer и restart не смешивают командные ограничения.

**Закрытие группы:** отчётить каждый из пяти результатов отдельно; partial/blocked не превращать в completed. Реально готовые ранее parts подтвердить, не переписывать.

## G087. Advancements

**Приоритет:** P2. **Карточки:** W06-431…W06-435.

**Связь:** W05-087.

**Сквозной объём направления:**

Эталон: PlayerAdvancements, InventoryChangeTrigger, RecipeCraftedTrigger.
Настоящие progress/listeners для inventory_changed/crafting/unlock по extracted
data. Award once/revoke/rejoin; stat counter и no-op hook не считаются критериями.

**Проверенные Java-точки входа:** `server/PlayerAdvancements.java`; `advancements/triggers/InventoryChangeTrigger.java`; `advancements/triggers/RecipeCraftedTrigger.java`

**Базовые зависимости:** нет заранее подтверждённой межгрупповой зависимости; проверить actual consumers. Нумерация не разрешает fake foundation.

### W06-431. Progress storage

**Сделать и доказать:** PlayerAdvancements хранит реальные criteria progress и persisted completion, не один boolean на событие.

### W06-432. Inventory criteria

**Сделать и доказать:** Слушатели реагируют на фактически изменившиеся slots во всех нужных inventory paths.

### W06-433. Crafting criteria

**Сделать и доказать:** RecipeCrafted и item callbacks передают правильный recipe/ingredients/result без фиктивного игрока.

### W06-434. Awards и revoke

**Сделать и доказать:** Награда/recipe unlock выполняются один раз, revoke возвращает нужную возможность повторного completion.

### W06-435. Data provenance

**Сделать и доказать:** Definitions/IDs/conditions берутся из produced data; login/sync/reload проверяют настоящее progress state.

**Закрытие группы:** отчётить каждый из пяти результатов отдельно; partial/blocked не превращать в completed. Реально готовые ранее parts подтвердить, не переписывать.

## G088. Statistics

**Приоритет:** P2. **Карточки:** W06-436…W06-440.

**Связь:** W05-088.

**Сквозной объём направления:**

Эталон: StatsCounter, ServerStatsCounter.
Сверить реальные item/block/entity actions и persistent stats. Обе руки,
creative exceptions/reload/relogin; регистрация stat без consumer не готова.

**Проверенные Java-точки входа:** `stats/StatsCounter.java`; `stats/ServerStatsCounter.java`

**Базовые зависимости:** нет заранее подтверждённой межгрупповой зависимости; проверить actual consumers. Нумерация не разрешает fake foundation.

### W06-436. Item statistics

**Сделать и доказать:** Use/break/craft/pickup/drop начисляются реальными consumers с правильными creative exceptions.

### W06-437. Block statistics

**Сделать и доказать:** Mining/interaction/container открытие учитывают фактический успех действия.

### W06-438. Entity statistics

**Сделать и доказать:** Damage/kill/death/breeding и species-specific действия используют правильные stat refs.

### W06-439. Persistent counters

**Сделать и доказать:** Save/rejoin/reset и commands сохраняют точные counts без повторного join начисления.

### W06-440. Scoreboard integration

**Сделать и доказать:** Criteria-linked objectives получают нужные updates, не подменяя advancements счётчиком статистики.

**Закрытие группы:** отчётить каждый из пяти результатов отдельно; partial/blocked не превращать в completed. Реально готовые ранее parts подтвердить, не переписывать.

## G089. /loot, /item, /random

**Приоритет:** P2. **Карточки:** W06-441…W06-445.

**Связь:** W05-089.

**Сквозной объём направления:**

Эталон: LootCommand, ItemCommands, RandomCommand.
Подключить к настоящим loot/components/server RandomSequences. Reset/defaults,
seed flags/permissions/domain/save; второй RNG bank для команды не создавать.

**Проверенные Java-точки входа:** `server/commands/LootCommand.java`; `server/commands/item/ItemCommands.java`; `server/commands/RandomCommand.java`

**Базовые зависимости:** нет заранее подтверждённой межгрупповой зависимости; проверить actual consumers. Нумерация не разрешает fake foundation.

### W06-441. Loot commands

**Сделать и доказать:** /loot использует настоящий context, источники и inventory insertion/drop policies.

### W06-442. Item commands

**Сделать и доказать:** /item читает и меняет реальные slots/components с нужными equipment notifications.

### W06-443. Random commands

**Сделать и доказать:** /random обращается к существующему server bank и реальному selected stream.

### W06-444. Seed defaults и reset

**Сделать и доказать:** Reset/clear/salt/flags изменяют persistent state по Java и не создают второй RNG owner.

### W06-445. Permissions и targets

**Сделать и доказать:** Недопустимый key/slot/target и domain resolution не оставляют частично выполненные changes.

**Закрытие группы:** отчётить каждый из пяти результатов отдельно; partial/blocked не превращать в completed. Реально готовые ранее parts подтвердить, не переписывать.

## G090. Gamerules

**Приоритет:** P2. **Карточки:** W06-446…W06-450.

**Связь:** W05-090.

**Сквозной объём направления:**

Эталон: GameRules, MinecraftServer.
Закрыть подтверждённые отсутствующие gameplay consumers и persistence.
Live change/rejoin/restart меняют поведение; сохранённый boolean без consumer не готов.

**Проверенные Java-точки входа:** `world/level/gamerules/GameRules.java`; `server/MinecraftServer.java`

**Базовые зависимости:** нет заранее подтверждённой межгрупповой зависимости; проверить actual consumers. Нумерация не разрешает fake foundation.

### W06-446. Gameplay gamerules

**Сделать и доказать:** Для подтверждённо отсутствующих consumers правило реально меняет outcome, не просто сохранённое поле.

### W06-447. Live changes

**Сделать и доказать:** Новый value учитывается без restart и отправляет необходимые client updates.

### W06-448. Scope правил

**Сделать и доказать:** World/domain/server ownership сохраняет согласованную архитектуру; не расширять scope по догадке.

### W06-449. Persistence

**Сделать и доказать:** Reload/rejoin сохраняют заданные значения и их влияние на последующее действие.

### W06-450. Комбинации правил

**Сделать и доказать:** Death/drop/spawn/explosion при конфликтующих правилах следуют Java приоритетам без hardcoded blanket disable.

**Закрытие группы:** отчётить каждый из пяти результатов отдельно; partial/blocked не превращать в completed. Реально готовые ранее parts подтвердить, не переписывать.

## G091. Расширенный Java golden взрывов

**Приоритет:** P2. **Карточки:** W06-451…W06-455.

**Связь:** W05-091.

**Сквозной объём направления:**

Эталон: ServerExplosion, ExplosionDamageCalculator.
Сохранить перенесённые ray/cache/TNT tests, расширить produced Java fixtures для
boundaries/resistance/fluid/blocklike/callbacks. Уже совпадающий engine заново не переписывать.

**Проверенные Java-точки входа:** `world/level/ServerExplosion.java`; `world/level/ExplosionDamageCalculator.java`

**Базовые зависимости:** нет заранее подтверждённой межгрупповой зависимости; проверить actual consumers. Нумерация не разрешает fake foundation.

### W06-451. Resistance golden

**Сделать и доказать:** Получить дополнительные produced Java cases для fluids/resistant/air/missing chunks.

### W06-452. Entity exposure golden

**Сделать и доказать:** Сверить direct/partial/blocked exposure на реальных collision shapes и разных entity sizes.

### W06-453. Order и callbacks

**Сделать и доказать:** Affected block order и мутирующие callbacks совпадают между compatibility и immutable lanes.

### W06-454. Special consumers

**Сделать и доказать:** Проверить blocklike entities, containers, TNT chains и gamerules, не переписывая уже верный engine.

### W06-455. Fixture provenance

**Сделать и доказать:** Сохранять версию, seed, command и raw outputs; Rust self-consistency не выдавать за Java comparison.

**Закрытие группы:** отчётить каждый из пяти результатов отдельно; partial/blocked не превращать в completed. Реально готовые ранее parts подтвердить, не переписывать.

## G092. Собственный worldgen parity прогон

**Приоритет:** P2. **Карточки:** W06-456…W06-460.

**Связь:** W05-092.

**Сквозной объём направления:**

Эталон: ChunkGenerator.
Воспроизводимый набор чанков трёх измерений с Java seed/coordinate manifest.
Сохранять первый diff и hashes; upstream 7500 chunks не выдавать за собственную проверку Vecta.

**Проверенные Java-точки входа:** `world/level/chunk/ChunkGenerator.java`

**Базовые зависимости:** нет заранее подтверждённой межгрупповой зависимости; проверить actual consumers. Нумерация не разрешает fake foundation.

### W06-456. Manifest прогонов

**Сделать и доказать:** Зафиксировать dimension/seed/coordinates/generator settings и команды воспроизведения своего набора.

### W06-457. Overworld comparison

**Сделать и доказать:** Сравнить blocks/biomes/heightmaps на выбранных сложных и случайных чанках.

### W06-458. Nether comparison

**Сделать и доказать:** Отдельно проверить Nether generator, carvers и structures с тем же Java target.

### W06-459. End comparison

**Сделать и доказать:** Сверить End terrain/islands/biomes и relevant structure placements.

### W06-460. Первый diff и отчёт

**Сделать и доказать:** Сохранять координату первого различия и source evidence; upstream 7500 chunks не считать своим результатом.

**Закрытие группы:** отчётить каждый из пяти результатов отдельно; partial/blocked не превращать в completed. Реально готовые ранее parts подтвердить, не переписывать.

## G093. Биомы, noise, heightmaps

**Приоритет:** P2. **Карточки:** W06-461…W06-465.

**Связь:** W05-093.

**Сквозной объём направления:**

Эталон: NoiseBasedChunkGenerator, NoiseChunk. Зависимость: 092.
Сложные координаты/seeds после Mth/section переноса. Сравнивать block/biome/
heightmap outputs, не один hash; fixtures генерировать, JSON руками не исправлять.

**Проверенные Java-точки входа:** `world/level/levelgen/NoiseBasedChunkGenerator.java`; `world/level/levelgen/NoiseChunk.java`

**Базовые зависимости:** нет заранее подтверждённой межгрупповой зависимости; проверить actual consumers. Нумерация не разрешает fake foundation.

### W06-461. Noise sampling

**Сделать и доказать:** Проверить координатные и seeded math пути после Mth изменений на Java outputs.

### W06-462. Biome assignment

**Сделать и доказать:** Сверить climate/biome selection по измерениям, не только итоговую block картинку.

### W06-463. Heightmaps

**Сделать и доказать:** Все используемые heightmap types обновляются при generation и gameplay changes по собственным predicates.

### W06-464. Carvers и surfaces

**Сделать и доказать:** Cave/canyon/surface правила и RNG order подтверждены произведёнными fixtures.

### W06-465. Boundary cases

**Сделать и доказать:** Negative/large coordinates, section edges и несколько seeds дают воспроизводимый первый diff, не случайный допуск.

**Закрытие группы:** отчётить каждый из пяти результатов отдельно; partial/blocked не превращать в completed. Реально готовые ранее parts подтвердить, не переписывать.

## G094. Структуры

**Приоритет:** P2. **Карточки:** W06-466…W06-470.

**Связь:** W05-094.

**Сквозной объём направления:**

Эталон: StructureTemplate, StructureTemplatePool.
Processors/block entities/structure entities с lazy loot. Одинаковый seed
даёт те же structure contents после загрузки; entity NBT не теряется в template placement.

**Проверенные Java-точки входа:** `world/level/levelgen/structure/templatesystem/StructureTemplate.java`; `world/level/levelgen/structure/pools/StructureTemplatePool.java`

**Базовые зависимости:** нет заранее подтверждённой межгрупповой зависимости; проверить actual consumers. Нумерация не разрешает fake foundation.

### W06-466. Template transforms

**Сделать и доказать:** Rotation/mirror корректно меняют block states, entity facing и позиционные offsets.

### W06-467. Processors

**Сделать и доказать:** Jigsaw/age/gravity/loot processors выполняют настоящие data-driven правила в правильном порядке.

### W06-468. Structure block entities

**Сделать и доказать:** Lazy loot и components создаются без преждевременного unpack или потери NBT.

### W06-469. Structure entities

**Сделать и доказать:** Spawned entities получают real factories, position/owner/data и корректный lifecycle.

### W06-470. Placement persistence

**Сделать и доказать:** Один seed после reload даёт те же структуры и contents, без повторного marker consumer.

**Закрытие группы:** отчётить каждый из пяти результатов отдельно; partial/blocked не превращать в completed. Реально готовые ранее parts подтвердить, не переписывать.

## G095. Порталы и border

**Приоритет:** P2. **Карточки:** W06-471…W06-475.

**Связь:** W05-095.

**Сквозной объём направления:**

Эталон: NetherPortalBlock, EndPortalBlock, PortalForcer.
Search/create/scaling/safe placement/border. Linked worlds/domains, unload,
passengers/owner/возврат; сохранить согласованные domain defaults.

**Проверенные Java-точки входа:** `world/level/block/NetherPortalBlock.java`; `world/level/block/EndPortalBlock.java`; `world/level/portal/PortalForcer.java`

**Базовые зависимости:** нет заранее подтверждённой межгрупповой зависимости; проверить actual consumers. Нумерация не разрешает fake foundation.

### W06-471. Portal search

**Сделать и доказать:** Найти допустимый существующий portal с correct scaling/distance/border constraints.

### W06-472. Portal creation

**Сделать и доказать:** При отсутствии создать настоящий frame/portal и safe destination в нужном world.

### W06-473. Entity transfer

**Сделать и доказать:** Passengers, velocity, facing и owners проходят общий teleport/recreation contract.

### W06-474. Domain routing

**Сделать и доказать:** Configured linked-world targets сохраняются; разные domains не склеиваются по одному dimension ID.

### W06-475. Reload и return

**Сделать и доказать:** Tickets, portal cooldown и обратный переход после restart не дают duplicate entity или неверный spawn.

**Закрытие группы:** отчётить каждый из пяти результатов отдельно; partial/blocked не превращать в completed. Реально готовые ранее parts подтвердить, не переписывать.

## G096. Chunk/entity lifecycle

**Приоритет:** P2. **Карточки:** W06-476…W06-480.

**Связь:** W05-096.

**Сквозной объём направления:**

Эталон: PersistentEntitySectionManager, ServerChunkCache.
Длинный unload/load/save с entities/containers/RNG. Нет duplicate UUID/повторного
loot; потоки продолжаются в стабильном порядке миров, как в принятой архитектуре.

**Проверенные Java-точки входа:** `world/level/entity/PersistentEntitySectionManager.java`; `server/level/ServerChunkCache.java`

**Базовые зависимости:** нет заранее подтверждённой межгрупповой зависимости; проверить actual consumers. Нумерация не разрешает fake foundation.

### W06-476. Entity unload

**Сделать и доказать:** Entity sections правильно переходят между ticking/loaded/unloaded и persisted состояниями.

### W06-477. UUID identity

**Сделать и доказать:** Сохранение и recreation не регистрируют один UUID в двух местах и не удерживают старый instance.

### W06-478. Container lifecycle

**Сделать и доказать:** Unload/break/reload закрывают или восстанавливают menu viewers без устаревшей inventory копии.

### W06-479. Shared RNG replay

**Сделать и доказать:** Named streams продолжают ту же последовательность при стабильном world order после сохранения.

### W06-480. Concurrent background I/O

**Сделать и доказать:** Фоновые saves/generation не заставляют игровой tick ждать disk/network и не подтверждают неуспешную запись.

**Закрытие группы:** отчётить каждый из пяти результатов отдельно; partial/blocked не превращать в completed. Реально готовые ранее parts подтвердить, не переписывать.

## G097. Измеренная производительность

**Приоритет:** P2. **Карточки:** W06-481…W06-485.

**Связь:** W05-097.

**Сквозной объём направления:**

Эталон: ServerExplosion, ChunkMap.
Замерить TNT/containers/entities на одинаковом workload с сырыми результатами.
Parity до/после; не ускорять удалением gameplay, сменой profiles/flags или отключением проверок.

**Проверенные Java-точки входа:** `world/level/ServerExplosion.java`; `server/level/ChunkMap.java`

**Базовые зависимости:** нет заранее подтверждённой межгрупповой зависимости; проверить actual consumers. Нумерация не разрешает fake foundation.

### W06-481. Повторяемый workload

**Сделать и доказать:** Подготовить измеримый сценарий TNT/containers/entities с фиксированными settings и warmup.

### W06-482. Профилирование

**Сделать и доказать:** Найти bottleneck по реальным timing/allocation данным, не по предположению о медленном helper.

### W06-483. Изменение алгоритма

**Сделать и доказать:** Оптимизация сохраняет gameplay order, seeded draws и callbacks, подтверждённые regression tests.

### W06-484. Сравнение результатов

**Сделать и доказать:** До/после измеряется на том же workload/hardware с raw logs, без смены build profiles ради цифры.

### W06-485. Performance handoff

**Сделать и доказать:** Публиковать пределы измерения и parity evidence; microbenchmark не равен full-server TPS.

**Закрытие группы:** отчётить каждый из пяти результатов отдельно; partial/blocked не превращать в completed. Реально готовые ранее parts подтвердить, не переписывать.

## G098. Flint отдельным tooling slice

**Приоритет:** P3 tooling. **Карточки:** W06-486…W06-490.

**Связь:** W05-098.

**Сквозной объём направления:**

Эталон: upstream steel-flint на проверенном SHA; Java entry point не применим.
Адаптировать bridge на реальные World/Player APIs и pinned dependency. Не копировать
глобальные lint allow/production unwrap. Specs/filtering проверить; PR auto-comments без запроса не включать.
Связь: upstream 01f29bb86, 2f4d13e88, d4655730d, 1a74818e0. Это не gameplay feature.

**Проверенные Java-точки входа:** upstream Flint bridge; перед port прочитать pinned API/specs.

**Базовые зависимости:** нет заранее подтверждённой межгрупповой зависимости; проверить actual consumers. Нумерация не разрешает fake foundation.

### W06-486. Flint source audit

**Сделать и доказать:** Прочитать pinned upstream bridge/API/spec format и определить требуемые реальные Vecta consumers.

### W06-487. World adapter

**Сделать и доказать:** Подключить настоящий World/tick/block lifecycle; fake world или no-op methods не принимаются.

### W06-488. Player adapter

**Сделать и доказать:** Действия Flint используют real inventory/use/network contracts и recorded packets.

### W06-489. Specs и reporting

**Сделать и доказать:** Запуск/фильтры/implemented IDs имеют понятный provenance; внешние PR auto-comments не включать без запроса.

### W06-490. Build discipline

**Сделать и доказать:** Не импортировать broad lint allow/production unwrap; dependency/tooling changes выделить отдельно и честно проверить.

**Закрытие группы:** отчётить каждый из пяти результатов отдельно; partial/blocked не превращать в completed. Реально готовые ранее parts подтвердить, не переписывать.

## G099. Полный Java golden лута

**Приоритет:** P0. **Карточки:** W06-491…W06-495.

**Связь:** W05-099.

**Сквозной объём направления:**

Эталон: LootTable, LootPool, LootItemFunction.
Получить produced {table, seed, slots, nextDraw} и сверить inventory fill/nested
tables. Версия/команда/provenance обязательны; повторяемость Rust сама по себе не доказывает vanilla parity.

**Проверенные Java-точки входа:** `world/level/storage/loot/LootTable.java`; `world/level/storage/loot/LootPool.java`; `world/level/storage/loot/functions/LootItemFunction.java`

**Базовые зависимости:** нет заранее подтверждённой межгрупповой зависимости; проверить actual consumers. Нумерация не разрешает fake foundation.

### W06-491. Java extraction

**Сделать и доказать:** Получить produced table/seed/slots/nextDraw fixtures с version и воспроизводимой командой.

### W06-492. Table evaluation

**Сделать и доказать:** Сверить entry→pool→table modifiers и nested tables на фактических Java outputs.

### W06-493. Container filling

**Сделать и доказать:** Занятые slots, shuffle/splitting/overflow дают тот же полный inventory layout.

### W06-494. Named sequence state

**Сделать и доказать:** Ненулевой seed и общий named stream правильно продолжаются после reload и между worlds.

### W06-495. Error cases

**Сделать и доказать:** Recursion/missing references/unsupported contexts не подменяются пустым loot; blockers описаны по реальным данным.

**Закрытие группы:** отчётить каждый из пяти результатов отдельно; partial/blocked не превращать в completed. Реально готовые ранее parts подтвердить, не переписывать.

## G100. Длинный survival сценарий

**Приоритет:** P2. **Карточки:** W06-496…W06-500.

**Связь:** W05-100.

**Сквозной объём направления:**

Эталон: MinecraftServer, ServerPlayer. Зависимости: выбранные готовые slices.
Добыча→крафт→контейнеры→бой→смерть→respawn→save/rejoin. Два настоящих клиента при
доступности; иначе client validation остаётся открытой, EULA самовольно не принимается.

**Проверенные Java-точки входа:** `server/MinecraftServer.java`; `server/level/ServerPlayer.java`

**Базовые зависимости:** нет заранее подтверждённой межгрупповой зависимости; проверить actual consumers. Нумерация не разрешает fake foundation.

### W06-496. Начальный survival

**Сделать и доказать:** Игрок добывает ресурсы, крафтит и пользуется существующими меню без командной выдачи результата.

### W06-497. База и автоматизация

**Сделать и доказать:** Контейнеры, hopper и redstone chain работают несколько игровых циклов с сохранением баланса.

### W06-498. Бой и смерть

**Сделать и доказать:** Настоящий mob/player combat проходит armor/effects/death/respawn и правильный drop.

### W06-499. Save и rejoin

**Сделать и доказать:** Остановка/сохранение/rejoin восстанавливают player/world/entities/recipes/RNG без скрытого reset.

### W06-500. Два клиента

**Сделать и доказать:** При доступности проверить реальных игроков; без разрешения EULA и client setup оставить этот уровень в отчёте непроверенным.

**External gate:** реальный client/server run и EULA требуют доступной среды и явного согласия; harness не закрывает этот пункт.

**Закрытие группы:** отчётить каждый из пяти результатов отдельно; partial/blocked не превращать в completed. Реально готовые ранее parts подтвердить, не переписывать.

## G101. Редстоуновая пыль

**Приоритет:** P1. **Карточки:** W06-501…W06-505.

**Связь:** дополнительное направление пакета 06.

**Сквозной объём направления:**

Эталон: RedstoneWireBlock.
Настоящая сеть проводов: topology, signal, update order и изменение consumers.

**Проверенные Java-точки входа:** `world/level/block/RedstoneWireBlock.java`

**Базовые зависимости:** нет заранее подтверждённой межгрупповой зависимости; проверить actual consumers. Нумерация не разрешает fake foundation.

### W06-501. Wire topology

**Сделать и доказать:** Connections и shape redstone dust обновляются при размещении и соседних изменениях.

### W06-502. Signal propagation

**Сделать и доказать:** Power level вычисляется настоящими source/signal queries, включая вертикальные переходы.

### W06-503. Update order

**Сделать и доказать:** Порядок neighbor updates сверить с Java; не добавлять асинхронное распространение ради скорости.

### W06-504. Indirect consumers

**Сделать и доказать:** Пыль реально питает соседние blocks, не только меняет свой POWER field.

### W06-505. Boundary и reload

**Сделать и доказать:** Chunk edges, breaking loops и reload не оставляют stale signal или бесконечную рекурсию.

**Закрытие группы:** отчётить каждый из пяти результатов отдельно; partial/blocked не превращать в completed. Реально готовые ранее parts подтвердить, не переписывать.

## G102. Повторители и компараторы

**Приоритет:** P1. **Карточки:** W06-506…W06-510.

**Связь:** дополнительное направление пакета 06.

**Сквозной объём направления:**

Эталон: RepeaterBlock, ComparatorBlock, DiodeBlock.
Delay, locking, compare/subtract и analog signals от реальных контейнеров.

**Проверенные Java-точки входа:** `world/level/block/RepeaterBlock.java`; `world/level/block/ComparatorBlock.java`; `world/level/block/DiodeBlock.java`

**Базовые зависимости:** нет заранее подтверждённой межгрупповой зависимости; проверить actual consumers. Нумерация не разрешает fake foundation.

### W06-506. Repeater delay

**Сделать и доказать:** Scheduled ticks и delay states дают правильный rising/falling pulse.

### W06-507. Repeater locking

**Сделать и доказать:** Боковое питание блокирует изменение выхода по настоящим source conditions.

### W06-508. Comparator modes

**Сделать и доказать:** Compare/subtract используют задние и боковые сигналы с правильными block queries.

### W06-509. Container analog input

**Сделать и доказать:** Изменение живого inventory обновляет comparator без необходимости повторного открытия меню.

### W06-510. Pulse regression

**Сделать и доказать:** Короткий pulse, repeated toggle, break и reload проверены на реальной redstone цепочке.

**Закрытие группы:** отчётить каждый из пяти результатов отдельно; partial/blocked не превращать в completed. Реально готовые ранее parts подтвердить, не переписывать.

## G103. Поршни

**Приоритет:** P1. **Карточки:** W06-511…W06-515.

**Связь:** дополнительное направление пакета 06.

**Сквозной объём направления:**

Эталон: PistonBaseBlock, PistonMovingBlockEntity, PistonStructureResolver.
Реальный push graph, sticky groups, moving state и displacement entities.

**Проверенные Java-точки входа:** `world/level/block/piston/PistonBaseBlock.java`; `world/level/block/piston/PistonMovingBlockEntity.java`; `world/level/block/piston/PistonStructureResolver.java`

**Базовые зависимости:** нет заранее подтверждённой межгрупповой зависимости; проверить actual consumers. Нумерация не разрешает fake foundation.

### W06-511. Push resolution

**Сделать и доказать:** PistonStructureResolver определяет перемещаемые blocks, ограничения и blocked result.

### W06-512. Sticky groups

**Сделать и доказать:** Slime/honey и sticky piston корректно связывают/разделяют группы по Java.

### W06-513. Moving state

**Сделать и доказать:** Реальный moving block entity хранит progress/state/direction и синхронизируется с клиентом.

### W06-514. Entity displacement

**Сделать и доказать:** Продвигаемые entities сталкиваются с правильными shapes, не проходят через piston head.

### W06-515. Finalization и NBT

**Сделать и доказать:** Завершение, interruption/break и reload не теряют blocks, inventories или moving state.

**Закрытие группы:** отчётить каждый из пяти результатов отдельно; partial/blocked не превращать в completed. Реально готовые ранее parts подтвердить, не переписывать.

## G104. Рельсы и сигналы

**Приоритет:** P1. **Карточки:** W06-516…W06-520.

**Связь:** дополнительное направление пакета 06.

**Сквозной объём направления:**

Эталон: DetectorRailBlock, PoweredRailBlock, RailState.
Shape connections и redstone consumers рельсов, связанные с настоящими вагонетками.

**Проверенные Java-точки входа:** `world/level/block/DetectorRailBlock.java`; `world/level/block/PoweredRailBlock.java`; `world/level/block/RailState.java`

**Базовые зависимости:** нет заранее подтверждённой межгрупповой зависимости; проверить actual consumers. Нумерация не разрешает fake foundation.

### W06-516. Rail connections

**Сделать и доказать:** Shape rails меняется по соседям и поддержке, включая повороты и slopes.

### W06-517. Power propagation

**Сделать и доказать:** Powered rail signal распространяется с правильными ограничениями и направлением.

### W06-518. Detector rail

**Сделать и доказать:** Реальная minecart presence активирует output и comparator по нужным consumer данным.

### W06-519. Activator rail

**Сделать и доказать:** Поддерживаемые cart variants получают реальные activation callbacks.

### W06-520. Break и boundaries

**Сделать и доказать:** Отсутствующая опора, chunk edges и reload не оставляют несогласованную shape или signal.

**Закрытие группы:** отчётить каждый из пяти результатов отдельно; partial/blocked не превращать в completed. Реально готовые ранее parts подтвердить, не переписывать.

## G105. Жидкости

**Приоритет:** P1. **Карточки:** W06-521…W06-525.

**Связь:** дополнительное направление пакета 06.

**Сквозной объём направления:**

Эталон: FlowingFluid, WaterFluid, LavaFluid.
Scheduled spreading, source rules, mixing, containers и currents.

**Проверенные Java-точки входа:** `world/level/material/FlowingFluid.java`; `world/level/material/WaterFluid.java`; `world/level/material/LavaFluid.java`

**Базовые зависимости:** нет заранее подтверждённой межгрупповой зависимости; проверить actual consumers. Нумерация не разрешает fake foundation.

### W06-521. Water ticks

**Сделать и доказать:** Scheduled fluid updates вычисляют levels, source conversion и direction по Java.

### W06-522. Lava ticks

**Сделать и доказать:** Lava использует свои delays, environment rules и spreading conditions.

### W06-523. Mixing

**Сделать и доказать:** Water/lava reactions создают предусмотренные blocks в правильном порядке, не blanket replacement.

### W06-524. Fluid containers

**Сделать и доказать:** Waterlogging, pickup, sponge и bucket consumers согласованы с обычным fluid tick.

### W06-525. Currents и persistence

**Сделать и доказать:** Entity currents, neighbor updates и reload сохраняют корректное состояние потока.

**Закрытие группы:** отчётить каждый из пяти результатов отдельно; partial/blocked не превращать в completed. Реально готовые ранее parts подтвердить, не переписывать.

## G106. Огонь, костры и свечи

**Приоритет:** P1. **Карточки:** W06-526…W06-530.

**Связь:** дополнительное направление пакета 06.

**Сквозной объём направления:**

Эталон: FireBlock, CampfireBlock, CandleBlock.
Survival/spread/burn, ignition consumers, cooking и extinguish.

**Проверенные Java-точки входа:** `world/level/block/FireBlock.java`; `world/level/block/CampfireBlock.java`; `world/level/block/CandleBlock.java`

**Базовые зависимости:** нет заранее подтверждённой межгрупповой зависимости; проверить actual consumers. Нумерация не разрешает fake foundation.

### W06-526. Fire survival

**Сделать и доказать:** FireBlock выбирает placement/age/extinguish с настоящими support и environment checks.

### W06-527. Spread и burn

**Сделать и доказать:** Random/scheduled consumers соблюдают flammability, humidity и gamerules.

### W06-528. TNT burning

**Сделать и доказать:** Горение вызывает настоящий TntBlock prime и расход блока, а не второй независимый TNT factory.

### W06-529. Campfire consumers

**Сделать и доказать:** Lit state, food cooking, damage и extinguish работают с реальными slots и timers.

### W06-530. Candle lifecycle

**Сделать и доказать:** Lighting/extinguish/waterlogging и candle variants меняют actual light/events и сохраняются.

**Закрытие группы:** отчётить каждый из пяти результатов отдельно; partial/blocked не превращать в completed. Реально готовые ранее parts подтвердить, не переписывать.

## G107. Рост растений

**Приоритет:** P1. **Карточки:** W06-531…W06-535.

**Связь:** дополнительное направление пакета 06.

**Сквозной объём направления:**

Эталон: CropBlock, StemBlock, SaplingBlock.
Random-tick growth и реальный feature/fruit/harvest pipeline.

**Проверенные Java-точки входа:** `world/level/block/CropBlock.java`; `world/level/block/StemBlock.java`; `world/level/block/SaplingBlock.java`

**Базовые зависимости:** нет заранее подтверждённой межгрупповой зависимости; проверить actual consumers. Нумерация не разрешает fake foundation.

### W06-531. Crop aging

**Сделать и доказать:** Random tick и условия света/почвы реально меняют возраст культур.

### W06-532. Stem fruits

**Сделать и доказать:** Stem выбирает допустимую клетку и создаёт правильный плод/attached state.

### W06-533. Sapling stages

**Сделать и доказать:** Stage и tree generation учитывают пространство и generated feature refs.

### W06-534. Harvest и replant

**Сделать и доказать:** Breaking/mature drops и player/automation actions сохраняют components и seed balance.

### W06-535. Seeds и replay

**Сделать и доказать:** Seeded growth/worldgen callbacks используют нужный RNG; не подменять probabilities ускоряющим timer.

**Закрытие группы:** отчётить каждый из пяти результатов отдельно; partial/blocked не превращать в completed. Реально готовые ранее parts подтвердить, не переписывать.

## G108. Почва и компостер

**Приоритет:** P1. **Карточки:** W06-536…W06-540.

**Связь:** дополнительное направление пакета 06.

**Сквозной объём направления:**

Эталон: FarmlandBlock, ComposterBlock.
Moisture/trampling/hoe и item→compost→output без дюпа.

**Проверенные Java-точки входа:** `world/level/block/FarmlandBlock.java`; `world/level/block/ComposterBlock.java`

**Базовые зависимости:** нет заранее подтверждённой межгрупповой зависимости; проверить actual consumers. Нумерация не разрешает fake foundation.

### W06-536. Farmland moisture

**Сделать и доказать:** Близость воды, дождь и укрытие обновляют moisture по source rules.

### W06-537. Trampling

**Сделать и доказать:** Player/mob fall, gamerules и entity predicate управляют превращением почвы.

### W06-538. Hoe consumer

**Сделать и доказать:** Item use преобразует допустимые блоки с правильными conditions и durability.

### W06-539. Composter

**Сделать и доказать:** Допустимые items, RNG, levels и готовый output проходят настоящий inventory/automation consumer.

### W06-540. Farm lifecycle

**Сделать и доказать:** Crops/support/fluid changes и reload не дают лишнего урожая или бесконечной готовой костной муки.

**Закрытие группы:** отчётить каждый из пяти результатов отдельно; partial/blocked не превращать в completed. Реально готовые ранее parts подтвердить, не переписывать.

## G109. Навигация и поиск пути

**Приоритет:** P1. **Карточки:** W06-541…W06-545.

**Связь:** дополнительное направление пакета 06.

**Сквозной объём направления:**

Эталон: PathNavigation, PathFinder, WalkNodeEvaluator.
Настоящие path requests, nodes, movement controllers и dynamic obstacles.

**Проверенные Java-точки входа:** `world/entity/ai/navigation/PathNavigation.java`; `world/level/pathfinder/PathFinder.java`; `world/level/pathfinder/WalkNodeEvaluator.java`

**Базовые зависимости:** нет заранее подтверждённой межгрупповой зависимости; проверить actual consumers. Нумерация не разрешает fake foundation.

### W06-541. Navigation requests

**Сделать и доказать:** Настоящие goals запрашивают путь и получают корректный результат либо failure.

### W06-542. Node evaluation

**Сделать и доказать:** Walkable/open/blocked/water/hazard nodes учитывают entity dimensions и capabilities.

### W06-543. Динамические препятствия

**Сделать и доказать:** Door/trapdoor/fence/block changes обновляют доступность пути в нужные моменты.

### W06-544. Movement control

**Сделать и доказать:** Following path реально управляет movement/jump/look, не teleport между nodes.

### W06-545. Scope оптимизации

**Сделать и доказать:** Согласовать threading при необходимости; tick не ждёт async job, parity важнее произвольного ускорения.

**Закрытие группы:** отчётить каждый из пяти результатов отдельно; partial/blocked не превращать в completed. Реально готовые ранее parts подтвердить, не переписывать.

## G110. Разведение и outputs животных

**Приоритет:** P1. **Карточки:** W06-546…W06-550.

**Связь:** дополнительное направление пакета 06.

**Сквозной объём направления:**

Эталон: Animal, AgeableMob, Chicken, Sheep, Cow.
Age/love/birth и species-specific egg/milk/wool consumers.

**Проверенные Java-точки входа:** `world/entity/animal/Animal.java`; `world/entity/AgeableMob.java`; `world/entity/animal/chicken/Chicken.java`; `world/entity/animal/sheep/Sheep.java`; `world/entity/animal/cow/Cow.java`

**Базовые зависимости:** нет заранее подтверждённой межгрупповой зависимости; проверить actual consumers. Нумерация не разрешает fake foundation.

### W06-546. Возраст животных

**Сделать и доказать:** Baby/adult timers и dimensions обновляются с правильным simulation gating.

### W06-547. Love и food

**Сделать и доказать:** Species food predicates, расход и love mode вызываются реальными hand interactions.

### W06-548. Birth consumer

**Сделать и доказать:** Два подходящих родителя создают один offspring с допустимыми inherited properties.

### W06-549. Species outputs

**Сделать и доказать:** Egg laying, milk, wool regrowth и другие реальные consumers имеют свои условия.

### W06-550. Persistence breeding

**Сделать и доказать:** Age/love/cooldown и reload не дают повторного offspring или потерю tame/bucket flags.

**Закрытие группы:** отчётить каждый из пяти результатов отдельно; partial/blocked не превращать в completed. Реально готовые ранее parts подтвердить, не переписывать.

## G111. Спавнеры, TrialSpawner и Vault

**Приоритет:** P1. **Карточки:** W06-551…W06-555.

**Связь:** дополнительное направление пакета 06.

**Сквозной объём направления:**

Эталон: BaseSpawner, SpawnerBlockEntity, TrialSpawner, VaultBlockEntity.
Работающие spawn/wave/reward consumers с persisted progress.

**Проверенные Java-точки входа:** `world/level/BaseSpawner.java`; `world/level/block/entity/SpawnerBlockEntity.java`; `world/level/block/entity/trialspawner/TrialSpawner.java`; `world/level/block/entity/vault/VaultBlockEntity.java`

**Базовые зависимости:** нет заранее подтверждённой межгрупповой зависимости; проверить actual consumers. Нумерация не разрешает fake foundation.

### W06-551. Обычный spawner

**Сделать и доказать:** Spawn delay, nearby conditions и entity data создают настоящих mobs.

### W06-552. Spawner configuration

**Сделать и доказать:** NBT/command updates валидно меняют spawn parameters без fake entity factory.

### W06-553. Trial lifecycle

**Сделать и доказать:** TrialSpawner выполняет реальные activation/wave/cooldown transitions и считает игроков/мобов.

### W06-554. Vault rewards

**Сделать и доказать:** Vault state, eligible player и loot consumer не выдают награду повторно.

### W06-555. Reload испытания

**Сделать и доказать:** Saved progress, entity references и rewards переживают restart без повторной волны или дюпа.

**Закрытие группы:** отчётить каждый из пяти результатов отдельно; partial/blocked не превращать в completed. Реально готовые ранее parts подтвердить, не переписывать.

## G112. Рейды

**Приоритет:** P1. **Карточки:** W06-556…W06-560.

**Связь:** дополнительное направление пакета 06.

**Сквозной объём направления:**

Эталон: Raid, Raids, Raider.
Manager, waves, реальные raiders, исход raid и сохранение.

**Проверенные Java-точки входа:** `world/entity/raid/Raid.java`; `world/entity/raid/Raids.java`; `world/entity/raid/Raider.java`

**Базовые зависимости:** нет заранее подтверждённой межгрупповой зависимости; проверить actual consumers. Нумерация не разрешает fake foundation.

### W06-556. Raid manager

**Сделать и доказать:** Настоящее raid state связано с village/игроком и сохраняет жизненный цикл.

### W06-557. Wave spawning

**Сделать и доказать:** Состав и места волн берутся из target data/rules, а не произвольного списка mobs.

### W06-558. Raider behavior

**Сделать и доказать:** Поддерживаемые illagers/ravager имеют реальные movement/attack/target consumers.

### W06-559. Победа и поражение

**Сделать и доказать:** Bossbar, hero/other rewards и termination запускаются по фактическому исходу.

### W06-560. Restart raid

**Сделать и доказать:** Save/load, logout и отсутствие жителей не оставляют вечный raid или двойные награды.

**Закрытие группы:** отчётить каждый из пяти результатов отдельно; partial/blocked не превращать в completed. Реально готовые ранее parts подтвердить, не переписывать.

## G113. Piglin и Hoglin

**Приоритет:** P1. **Карточки:** W06-561…W06-565.

**Связь:** дополнительное направление пакета 06.

**Сквозной объём направления:**

Эталон: Piglin, PiglinAi, Hoglin.
Targets/barter/combat/conversion и сохранение настоящих Nether mobs.

**Проверенные Java-точки входа:** `world/entity/monster/piglin/Piglin.java`; `world/entity/monster/piglin/PiglinAi.java`; `world/entity/monster/hoglin/Hoglin.java`

**Базовые зависимости:** нет заранее подтверждённой межгрупповой зависимости; проверить actual consumers. Нумерация не разрешает fake foundation.

### W06-561. Piglin targets

**Сделать и доказать:** Brain/anger/проверка gold equipment реально меняют отношение к игроку.

### W06-562. Bartering

**Сделать и доказать:** Gold interaction даёт настоящую loot-table выдачу с правильным временем и расходом.

### W06-563. Hoglin behavior

**Сделать и доказать:** Movement/attack/breeding и fear conditions используют реальные species capabilities.

### W06-564. Conversions

**Сделать и доказать:** World/environment conversion и immunity flags сохраняются в правильных timers.

### W06-565. Persistence Nether mobs

**Сделать и доказать:** Owner/anger/inventory/variant и reload не создают бесплатный barter или бесконечный conversion.

**Закрытие группы:** отчётить каждый из пяти результатов отдельно; partial/blocked не превращать в completed. Реально готовые ранее parts подтвердить, не переписывать.

## G114. Пчёлы и ульи

**Приоритет:** P1. **Карточки:** W06-566…W06-570.

**Связь:** дополнительное направление пакета 06.

**Сквозной объём направления:**

Эталон: Bee, BeehiveBlockEntity.
Navigation/pollination/occupants/honey/anger и lifecycle.

**Проверенные Java-точки входа:** `world/entity/animal/bee/Bee.java`; `world/level/block/entity/BeehiveBlockEntity.java`

**Базовые зависимости:** нет заранее подтверждённой межгрупповой зависимости; проверить actual consumers. Нумерация не разрешает fake foundation.

### W06-566. Bee navigation

**Сделать и доказать:** Bee движется к подходящим цветам/улью и реагирует на препятствия.

### W06-567. Pollination

**Сделать и доказать:** Настоящий поиск/посещение цветов влияет на pollen state и допустимые crop consumers.

### W06-568. Hive occupancy

**Сделать и доказать:** Вход/выход хранит живых occupants и их данные, не только счётчик.

### W06-569. Honey и anger

**Сделать и доказать:** Сбор мёда/дым/разрушение/атака запускают правильные hive и bee реакции.

### W06-570. Reload улья

**Сделать и доказать:** Occupants/timers/honey level сохраняются; reload не выпускает duplicate bee или второй honey drop.

**Закрытие группы:** отчётить каждый из пяти результатов отдельно; partial/blocked не превращать в completed. Реально готовые ранее parts подтвердить, не переписывать.

## G115. Wither и Warden

**Приоритет:** P1. **Карточки:** W06-571…W06-575.

**Связь:** дополнительное направление пакета 06.

**Сквозной объём направления:**

Эталон: WitherBoss, Warden.
Boss spawning/phases/sensors/attacks и честный persistent lifecycle.

**Проверенные Java-точки входа:** `world/entity/boss/wither/WitherBoss.java`; `world/entity/monster/warden/Warden.java`

**Базовые зависимости:** нет заранее подтверждённой межгрупповой зависимости; проверить actual consumers. Нумерация не разрешает fake foundation.

### W06-571. Wither spawning

**Сделать и доказать:** Pattern consumer создаёт реального boss с начальной invulnerability и charge state.

### W06-572. Wither gameplay

**Сделать и доказать:** Movement/targets/projectiles/explosion и bossbar связаны с настоящими фазами.

### W06-573. Warden sensors

**Сделать и доказать:** Vibration/smell/anger используют реальный event path и source memory.

### W06-574. Warden attacks

**Сделать и доказать:** Melee/sonic/digging emergence выполняются настоящими timed consumers, не generic teleport/hurt.

### W06-575. Boss lifecycle

**Сделать и доказать:** Death/reload/player absence не оставляют вторую bossbar, повторный loot или вечную invulnerability.

**Закрытие группы:** отчётить каждый из пяти результатов отдельно; partial/blocked не превращать в completed. Реально готовые ранее parts подтвердить, не переписывать.

## G116. Подводные блоки

**Приоритет:** P1. **Карточки:** W06-576…W06-580.

**Связь:** дополнительное направление пакета 06.

**Сквозной объём направления:**

Эталон: BubbleColumnBlock, ConduitBlockEntity, SpongeBlock.
Колонны, sponge и conduit взаимодействуют с реальными fluids/entities.

**Проверенные Java-точки входа:** `world/level/block/BubbleColumnBlock.java`; `world/level/block/entity/ConduitBlockEntity.java`; `world/level/block/SpongeBlock.java`

**Базовые зависимости:** нет заранее подтверждённой межгрупповой зависимости; проверить actual consumers. Нумерация не разрешает fake foundation.

### W06-576. Bubble column formation

**Сделать и доказать:** Water/source/support blocks определяют появление и направление настоящей колонны.

### W06-577. Entity effects колонки

**Сделать и доказать:** Игрок/boat/прочие entities получают нужные impulses и air/fall effects.

### W06-578. Sponge absorption

**Сделать и доказать:** Удаляется только допустимый объём воды с ванильным обходом и wet-state переходом.

### W06-579. Conduit structure

**Сделать и доказать:** Проверка воды/рамки меняет реальный active/power state.

### W06-580. Подводный lifecycle

**Сделать и доказать:** Block updates и reload не оставляют скрытые column/conduit effects после разрушения конструкции.

**Закрытие группы:** отчётить каждый из пяти результатов отдельно; partial/blocked не превращать в completed. Реально готовые ранее parts подтвердить, не переписывать.

## G117. Погода и молния

**Приоритет:** P1. **Карточки:** W06-581…W06-585.

**Связь:** дополнительное направление пакета 06.

**Сквозной объём направления:**

Эталон: ServerLevel, LightningBolt, SnowLayerBlock.
Timers, lightning, snow/ice, sleep и client sync.

**Проверенные Java-точки входа:** `server/level/ServerLevel.java`; `world/entity/LightningBolt.java`; `world/level/block/SnowLayerBlock.java`

**Базовые зависимости:** нет заранее подтверждённой межгрупповой зависимости; проверить actual consumers. Нумерация не разрешает fake foundation.

### W06-581. Weather timers

**Сделать и доказать:** Rain/thunder transitions принадлежат правильному миру/домену и сохраняют timers.

### W06-582. Lightning consumer

**Сделать и доказать:** Молния реально влияет на допустимые blocks/entities с attribution и gamerules.

### W06-583. Ice и snow

**Сделать и доказать:** Weather/random tick применяют environment/biome условия образования и таяния.

### W06-584. Sleep interaction

**Сделать и доказать:** Пропуск ночи/погоды соблюдает gamerules и не сбрасывает чужой domain time.

### W06-585. Client sync

**Сделать и доказать:** Join/rejoin/weather change отправляют согласованное состояние без постоянного локального дождя после выключения.

**Закрытие группы:** отчётить каждый из пяти результатов отдельно; partial/blocked не превращать в completed. Реально готовые ранее parts подтвердить, не переписывать.

## G118. Сетевой lifecycle игрока

**Приоритет:** P1. **Карточки:** W06-586…W06-590.

**Связь:** дополнительное направление пакета 06.

**Сквозной объём направления:**

Эталон: ServerGamePacketListenerImpl, ServerPlayer, ServerChunkCache.
Authoritative movement/teleport/chunks/inventory и callbacks старого соединения.

**Проверенные Java-точки входа:** `server/network/ServerGamePacketListenerImpl.java`; `server/level/ServerPlayer.java`; `server/level/ServerChunkCache.java`

**Базовые зависимости:** нет заранее подтверждённой межгрупповой зависимости; проверить actual consumers. Нумерация не разрешает fake foundation.

### W06-586. Movement validation

**Сделать и доказать:** Serverbound position/rotation проверяются до изменения авторитетного player state.

### W06-587. Teleport acknowledgement

**Сделать и доказать:** Pending teleport и ack не позволяют старым movement packets вернуть игрока назад.

### W06-588. Chunk synchronization

**Сделать и доказать:** Load/unload/block updates/entity visibility соответствуют фактическим server tracking sets.

### W06-589. Inventory packet authority

**Сделать и доказать:** Old container/state IDs и клиентские changed-slots не превращаются в новый server inventory.

### W06-590. Connection lifecycle

**Сделать и доказать:** Keepalive/disconnect/rejoin и delayed packets не исполняют callbacks старой player incarnation.

**Закрытие группы:** отчётить каждый из пяти результатов отдельно; partial/blocked не превращать в completed. Реально готовые ранее parts подтвердить, не переписывать.

## G119. Правила измерений

**Приоритет:** P1. **Карточки:** W06-591…W06-595.

**Связь:** дополнительное направление пакета 06.

**Сквозной объём направления:**

Эталон: DimensionType, EnvironmentAttributes, PortalForcer.
Environment attributes, scaling, respawn и синхронный переход между worlds.

**Проверенные Java-точки входа:** `world/level/dimension/DimensionType.java`; `world/attribute/EnvironmentAttributes.java`; `world/level/portal/PortalForcer.java`

**Базовые зависимости:** нет заранее подтверждённой межгрупповой зависимости; проверить actual consumers. Нумерация не разрешает fake foundation.

### W06-591. Dimension attributes

**Сделать и доказать:** EnvironmentAttributes/DimensionType реально определяют высоту, свет, fluid и sky behavior.

### W06-592. Travel scaling

**Сделать и доказать:** Portal routes применяют правильные coordinate scale и border constraints.

### W06-593. Respawn environment

**Сделать и доказать:** Bed/anchor/default respawn учитывают измерение и configured domain targets.

### W06-594. World transitions

**Сделать и доказать:** Time/weather/spawn info и entity state синхронизируются после перехода.

### W06-595. Persistent dimensions

**Сделать и доказать:** Reload и несколько domains не смешивают gamerules, seed и уже согласованный общий named RNG.

**Закрытие группы:** отчётить каждый из пяти результатов отдельно; partial/blocked не превращать в completed. Реально готовые ранее parts подтвердить, не переписывать.

## G120. Beacon и Conduit

**Приоритет:** P1. **Карточки:** W06-596…W06-600.

**Связь:** дополнительное направление пакета 06.

**Сквозной объём направления:**

Эталон: BeaconBlockEntity, ConduitBlockEntity.
Структура, menu/payment, область effects и teardown consumers.

**Проверенные Java-точки входа:** `world/level/block/entity/BeaconBlockEntity.java`; `world/level/block/entity/ConduitBlockEntity.java`

**Базовые зависимости:** нет заранее подтверждённой межгрупповой зависимости; проверить actual consumers. Нумерация не разрешает fake foundation.

### W06-596. Beacon pyramid

**Сделать и доказать:** Настоящие block scans и base levels определяют доступные beacon powers.

### W06-597. Beacon menu

**Сделать и доказать:** Payment item и selection валидируются сервером, не принимая произвольный client effect ID.

### W06-598. Area effects beacon

**Сделать и доказать:** Допустимые игроки получают выбранные effects с правильной дальностью и обновлением.

### W06-599. Conduit effects

**Сделать и доказать:** Active conduit даёт предусмотренные underwater effects/attacks через реальные targets и timers.

### W06-600. Structure teardown

**Сделать и доказать:** Break/obstruction/unload/reload снимают или восстанавливают effects и menu state без платёжного дюпа.

**Закрытие группы:** отчётить каждый из пяти результатов отдельно; partial/blocked не превращать в completed. Реально готовые ранее parts подтвердить, не переписывать.
