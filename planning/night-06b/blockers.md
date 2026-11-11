# Агент B — блокеры и известные отклонения (ночь 06, пакет 06)

Формат: срез → что мешает → что сделано вместо → что нужно, чтобы закрыть.

## Срез 1 (bonemeal на level random + кораллы)

### 1. `BlockBehavior::update_shape` не получает `RandomSource`

- Java: `Block.updateShape` (`Block.java:201`) вызывает
  `state.updateShape(level, ticks, pos, direction, neighbourPos, neighbourState, level.getRandom())`,
  поэтому `GrowingPlantBodyBlock.updateShape` передаёт рандом в
  `headBlock.getStateForPlacement(random)` (возраст головы).
- Vecta: трейт `BlockBehavior::update_shape` (`vecta-core/src/behavior/block/mod.rs`)
  рандома не имеет; в workspace 366 упоминаний `update_shape`, включая десятки
  переопределений и тестов. Добавление параметра — отдельный механический
  рефакторинг общего трейта, в срез 1 не входит (BRIEF просит менять общие
  трейты минимально).
- Сделано вместо: `GrowingPlantBodyBlock::update_shape` создаёт
  `RandomSource::Legacy(LegacyRandom::from_seed(rand::random()))` и берёт
  `nextInt(6)` — диапазон и наблюдаемое поведение как в ванили, но поток не
  мировой (это допустимая живая случайность по AGENTS.md/CHECKLIST).
- Чтобы закрыть: добавить `random: &mut dyn RandomAccess` в
  `BlockBehavior::update_shape` и во все вызовы, прокидывая
  `LevelRandomSource::new(world.random())` там, где есть `&Arc<World>`.

### 2. `sapling_block.rs` без bonemeal/random tick

- `vecta-core/src/behavior/blocks/vegetation/sapling_block.rs:12` —
  `// TODO: Implement random ticking, bonemeal growth, and tree grower behavior.`
- Это не блокер данных: пункт «саженцы и TreeGrower из реальных configured
  features» стоит в моей области (BRIEF, раздел 2) и будет отдельным срезом;
  записано, чтобы не потерять.
