# Night 06 — blockers

Проверенные блокеры. Формат: что проверено → чего не хватает → риск догадки →
точный path/output/decision → зависимые карточки.

## B-06-01 · G011/W06-052 — у саженцев нет bonemeal-роста

**Проверено:** `vecta-core/src/behavior/blocks/vegetation/sapling_block.rs`
реализует только `BlockBehavior` (`can_survive`, `get_state_for_placement`) и не
имеет `Bonemealable`, поэтому `BoneMealItem.growCrop` возвращает `false` для
саженца. Машинерия фичи уже есть и работает:
`MangrovePropaguleBlock::grow_tree` вызывает
`FeatureDecorationRunner::place_tree_feature` с
`vanilla_configured_features`. Все нужные configured features присутствуют в
`vecta-registry/src/generated/vanilla_configured_features.rs`: `OAK`,
`FANCY_OAK`, `OAK_BEES_005`, `FANCY_OAK_BEES_005`, `SPRUCE`, `MEGA_SPRUCE`,
`MEGA_PINE`, `BIRCH`, `BIRCH_BEES_005`, `JUNGLE_TREE_NO_VINE`,
`MEGA_JUNGLE_TREE`, `ACACIA`, `DARK_OAK`, `CHERRY`, `CHERRY_BEES_005`,
`PALE_OAK_BONEMEAL`, `AZALEA_TREE`.

**Чего не хватает:** аналога vanilla `TreeGrower`
(`minecraft-src/minecraft/src/net/minecraft/world/level/block/grower/TreeGrower.java`):
взвешенные списки `trees`/`megaTrees`/`flowerTrees` на саженец, выбор
`shortestTreeType`, проверка цветов вокруг (`hasFlowers`), поиск 2×2 саженцев
для mega-деревьев, удаление/восстановление саженцев при неудачном размещении.
Плюс `SaplingBlock: Bonemealable` с ванильными `isBonemealSuccess`
(`random < 0.45`) и `performBonemeal` (STAGE 0→1, затем рост).

**Риск догадки:** частичная реализация только 1×1-деревьев даст неверный набор
деревьев (у dark oak/pale oak `trees` пуст и рост идёт только через mega), а
также потеряет FANCY_OAK и bees-варианты. Это shared worldgen foundation, на
которую опирается `SaplingBlock`, поэтому частичная версия не заводилась.

**Решение:** реализовать `TreeGrower`-эквивалент одним срезом вместе с
`SaplingBlock: Bonemealable` и тестом на настоящий `OAK_LOG` из фичи.
До этого W06-052 — partial. Зависимые карточки: W06-052.

## B-06-02 · G010/W06-046 — fire charge из раздатчика

**Проверено:** vanilla регистрирует `Items.FIRE_CHARGE` через
`DispenserBlock.registerProjectileBehavior`, то есть запускает `SmallFireball`
(`DispenseItemBehavior.bootStrap`). В Vecta нет `SmallFireball` entity, поэтому
`FIRE_CHARGE` намеренно не зарегистрирован: иначе пришлось бы подменить полёт
прямым поджиганием блока.

**Чего не хватает:** `SmallFireball` entity + регистрация (G047).

**Решение:** оставить `FIRE_CHARGE` вне `DISPENSE_BEHAVIORS` (тест
`a_fire_charge_stays_unregistered_until_the_small_fireball_exists` это
фиксирует) и реализовать вместе с G047. Зависимые карточки: W06-046 (часть).

## B-06-03 · G013/W06-062 — соты из улья

**Проверено:** `BeehiveBlock` (`vecta-core/src/behavior/blocks/container/beehive_block.rs`)
имеет свойство `LEVEL_HONEY`, но не имеет `dropHoneycomb`. В Vecta нет
`Bee` entity, поэтому `releaseBeesAndResetHoneyLevel` не может выпустить
пчёл, и нет пути «лут блока с инструментом» для трёх сот.

**Чего не хватает:** `BeehiveBlock.dropHoneycomb` (лут `blocks/beehive` с
shears) и `Bee` entity для `BeeReleaseStatus::BEE_RELEASED`.

**Решение:** `ShearsDispenseItemBehavior` реализует только entity-ветку
(настоящий `Shearable`), улей пока не стрижётся. Зависимые карточки: W06-062.
