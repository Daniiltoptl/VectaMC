# Агент C: мобы и сущности

> В Arena локальные пути Windows, папки сборки на диске D и локальная сборка не применяются: см. `planning/agents/ARENA.md`.


Сначала прочитай `C:\Users\My comp\Desktop\Vecta\planning/agents/COMMON.md`
(только чтение) и `AGENTS.md`. Папка `C:\Users\My comp\Desktop\Vecta-c`,
ветка `codex/night-vanilla-03-c` (tip `b7706815b`, перенос SteelMC уже влит,
ветка на GitHub есть). Журнал —
создай `planning/night-06c/` (progress, checkpoint, blockers), отчёт
`planning/night-06c/report.md`. Этот файл не коммить.

Сборочные входы `vecta-utils/build_assets/` (gitignored) уже скопированы.
Java-исходники читай из папки A. Ревью прошлых сеансов —
`planning/reviews/night-06-session*-claude.md`.

## Что уже пришло из SteelMC

Кальмар и его цели (`squid_flee_goal`, `squid_random_movement_goal`), спавнер
(`block_entity/entities/spawner/`, `SpawnPlacements`), вагонетка со спавнером,
отталкивание сущностей и spatial queries (`entity/pushing`, `entity/selector`).
Не пиши их заново: изучи и строй на них. G111 (спавнер) считай закрытым,
кроме TrialSpawner и Vault.

## Главное правило блока

Мобы — самый большой и самый фундаментальный блок. Сначала фундамент, потом
виды. Никаких «моб стоит» или упрощённых ИИ: только настоящие `Goal`,
`GoalSelector`, `Brain`/`Behavior`/`Sensor`/`Memory` по Java. Перед кодом
изучи, что уже есть в `vecta-core/src/entity/`, и опиши в `progress.md`
текущее состояние и план фундамента. Если архитектура ИИ неясна (как хранить
Brain, как тикать, как сериализовать memories), запиши вопрос в `blockers.md`
с вариантами и делай независимые пункты (сущности, протокол).

Damage pipeline, голод, эффекты и смерть игрока — зона агента A. Если мобу
нужна атака/урон, используй существующий путь; чего не хватает — блокер «нужно
от агента A».

## Очередь

1. Фундамент ИИ: `Mob`, `PathfinderMob`, `GoalSelector` (приоритеты, флаги,
   running set), `MoveControl`, `LookControl`, `JumpControl`, `BodyRotation`,
   `Sensing`. Затем G109 навигация: `PathNavigation`, `PathFinder`,
   `WalkNodeEvaluator`, `SwimNodeEvaluator`, `FlyNodeEvaluator`, кэш пути,
   `PathType` по блокам. Затем `Brain` (memories, sensors, activities,
   behaviors) для мобов, которые на нём в Java.
2. Цели: `TargetGoal`, `NearestAttackableTargetGoal`, `HurtByTargetGoal`,
   `MeleeAttackGoal`, `RangedBowAttackGoal`, `RandomStrollGoal`,
   `LookAtPlayerGoal`, `FloatGoal`, `PanicGoal`, `TemptGoal`, `BreedGoal`,
   `FollowParentGoal`, `AvoidEntityGoal`.
3. G069 natural spawning (spawn cap по категориям, свет, биомы и
   `spawn_costs`, packs, `SpawnPlacements`), G070 despawn и persistence.
4. Враждебные: G071 зомби (и варианты, осада двери, превращения), G072
   скелет (лук через путь агента A), G073 крипер (взрыв через существующий
   explosion), G074 паук, G075 эндермен.
5. Мирные: G110 разведение и outputs (молоко, шерсть, яйца), G079 водные,
   G080 приручаемые (wolf, cat, horse, parrot), G077 големы.
6. G076 жители: профессии и POI, workstations, торговля (офферы из данных,
   restock, цены, репутация/gossip), сон, паника. Затем wandering trader.
7. G113 Piglin/Hoglin, G114 пчёлы и ульи, G112 рейды, G111 спавнеры,
   TrialSpawner и Vault, G115 Wither и Warden, G078 дракон и End fight.
8. Прочие сущности: ItemEntity (слияние, деспаун, pickup delay),
   ExperienceOrb (слияние), FallingBlock, PrimedTnt и TNT-вагонетка, молния,
   рамки, LeashKnot, display-сущности, Interaction, Marker; физика
   сущностей (трение, лёд, слизь, мёд, паутина, flow push, пузырьковые колонны).
9. Покрытие протокола: `planning/audit/protocol-coverage.md` — таблица всех
   пакетов 26.3 из extracted данных: реализован / есть обработчик / нет.
