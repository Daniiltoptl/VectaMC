# Агент B: мир, блоки, редстоун, команды

> В Arena локальные пути Windows, папки сборки на диске D и локальная сборка не применяются: см. `planning/agents/ARENA.md`.


Сначала прочитай `C:\Users\My comp\Desktop\Vecta\planning/agents/COMMON.md`
(только чтение), затем `AGENTS.md` и `planning/night-06b/` (BRIEF, CHECKLIST,
progress) в своей папке. Папка `C:\Users\My comp\Desktop\Vecta-b`, ветка
`codex/night-vanilla-03-b` (tip `5e369ddc2`, перенос SteelMC уже влит; в нём
рефактор жидкостей `vecta-core/src/fluid/` — изучи перед G105). Журнал и отчёт: `planning/night-06b/report.md`.
Этот файл не коммить.

Сборочные входы `vecta-utils/build_assets/` (gitignored) уже скопированы,
Java-исходники читай из папки A.

## Срез 0: незакоммиченная работа (первым)

Прошлый исполнитель оставил незакоммиченный срез «костная мука на level random
+ кораллы» (`bonemeal.rs`, `world/random.rs`, `world/mod.rs`). В нём ошибка:
`LevelRandomRng` оборачивает level random в трейты `rand`, а `random_range` из
`rand` использует другой алгоритм, чем Java `nextInt(bound)`. Поток и значения
не совпадут с ванилью. Переделай: убери `LevelRandomRng`, вызывай ванильный
генератор напрямую (`next_i32_bounded` = Java `nextInt(bound)`), сверь с
`BoneMealItem.growWaterPlant/growCrop` и `getRandomElementOf`. Если
`Bonemealable` принимает `&mut dyn rand::Rng`, это фундамент. Идея «брать
блокировку на каждый отдельный вызов» верная (между вызовами
`perform_bonemeal` пишет в мир, держать `with_level` нельзя), но методы должны
быть ванильные: хэндл над level random с `next_i32_bounded`/`next_f32`/… по
Java, по одной блокировке на вызов. Перевод `Bonemealable` на него —
изменение общего интерфейса: сделай минимально, прогони все bonemealable-блоки
и перечисли в отчёте. Если не уверен в форме интерфейса — блокер с вариантами,
и переходи к редстоуну. Тесты, коммит, push.

## Очередь

Подробности — `planning/night-06b/BRIEF.md` и `NIGHT_TASK_06.md`.

1. Редстоун: G101 пыль (power graph, порядок обновлений), G102 повторители,
   компараторы, факелы, G103 поршни (push limit, slime/honey,
   `MovingPistonBlock`), G104 рельсы.
2. Жидкости и огонь: G105 (растекание, источники, lava+water, вёдра,
   waterlogging), G106 огонь, костры, свечи, G116 подводные блоки, G117 погода
   и молния.
3. Рост растений: G107 саженцы и TreeGrower на реальных configured features,
   костная мука для всех блоков, тростник, кактус, бамбук, лозы, пещерные лозы,
   dripleaf, ягоды, нарост, хорус, тыквы/арбузы, какао, грибы, келп, аметист,
   сталактиты, лёд/снег по свету, порошковый снег.
4. Интерактивные блоки: котлы, проигрыватель, нотный блок, колокол, рычаги,
   кнопки, плиты, растяжка, наблюдатель, датчик дневного света, мишень,
   громоотвод, sculk sensor, окисление меди и воск, медные лампы, строительные
   леса, сундуки (двойные, ловушка, эндер), бочки, шалкер-боксы, резные полки.
5. Измерения: G095 адский портал (рамка, поиск через POI, масштаб 8), портал
   Энда, end gateway, граница мира, void, G119 правила `dimension_type`.
6. Команды: `fill`, `fillbiome`, `clone`, `setblock`, `place`, `locate`,
   `spreadplayers`, `forceload`, `worldborder`, `weather`, `time`,
   `difficulty`, `gamerule`, `setworldspawn`, `spawnpoint`, `particle`,
   `playsound`, `stopsound`, `tick`, `scoreboard`, `team`, `bossbar`, `title`,
   `tellraw`, `schedule`, `execute`. Дерево аргументов, права, translation keys.
