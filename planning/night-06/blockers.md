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

## B-06-04 · G021/W06-101…105 — имя и lock контейнера пишутся дважды

**Проверено:** `vecta-core/src/block_entity/container_identity.rs` пишет
`CustomName` и `lock` обычными NBT-ключами (`save`/`load`), и в то же время
`collect_implicit_components` кладёт те же значения в `components`. Контейнеры
вызывают `self.identity.save(nbt)` из `save_additional`
(`chest.rs:353`, `barrel.rs:252`, `dispenser.rs:189`, `beacon.rs:366`, …).

В 26.3 `BaseContainerBlockEntity` не пишет эти ключи в `saveAdditional`: они
живут только в `components` (`collectImplicitComponents`), а
`removeComponentsFromTag` вычищает legacy-ключи у старых миров.

**Чего не хватает:** шага `applyComponents` после загрузки карты компонентов.
`BlockEntity::load_with_components` (`block_entity/mod.rs`) вызывает
`load_additional_with_context`, затем `base().load_components(nbt)`, но не
применяет загруженные компоненты к состоянию сущности. Поэтому
`applyImplicitComponents` при reload не вызывается, и имя/lock восстанавливаются
только по legacy-ключам.

**Риск догадки:** это shared persistence foundation и формат сохранения сразу
для всех контейнеров (chest, barrel, dispenser, dropper, hopper, furnace,
shulker, beacon, …). Полу-сделанный переход оставил бы часть сущностей на
legacy-ключах, часть на компонентах.

**Решение:** отдельным срезом — hook `apply_components(&DataComponentMap)` в
`BlockEntity`, вызов его из `load_with_components`, `identity` читает имя/lock
из карты, `save_additional` перестаёт писать legacy-ключи, а `load` вызывает
`remove_components_from_tag`. Затем W06-101…105 можно закрывать тестами
(включая unload/load настоящего сохранённого BE). Зависимые карточки:
W06-101…105 (G021 не начата).
