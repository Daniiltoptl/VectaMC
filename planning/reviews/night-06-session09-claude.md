# Ревью сеанса 9: компостер и WorldlyContainerHolder

Ревьюер: Claude. Дата: 2026-11-07. Проверены `e332000db` и `8a3ad5e70` ветки
`codex/night-vanilla-03` против `ComposterBlock.java`, `SimpleContainer.java`,
`HopperBlockEntity.java` и `ItemEntity.java` из 26.3.

## Итог

Срез хороший: `useItemOn`/`useWithoutItem`/`addLayer`/`empty`/`extractProduce`/
`tick`/`onPlace`, контекст `BLOCK_INTERACT`, configured providers вместо таблицы,
порядок `getBlockContainer` (holder раньше блок-энтити) и три виртуальных
контейнера совпадают с Java. Найдено два расхождения, оба исправлены.

## Найденные расхождения и исправления

### 1. Скорость выпавшей костной муки расходовала level random

`new ItemEntity(level, x, y, z, stack)` берёт начальную скорость из
`this.random` самой сущности, а не из `level.getRandom()`. Из level random идёт
только `offsetRandomXZ`. Vecta брала и скорость из level random, то есть делала
два лишних вызова и сдвигала мировой поток для всего, что выполняется после.
Теперь скорость берётся из unseeded runtime RNG, как у сущности в Java.

### 2. Виртуальные контейнеры не повторяли `SimpleContainer`

В Java `SimpleContainer.setItem` и `removeItem` сами вызывают `setChanged`.
Трейт `Container` в Vecta так не делает, а `ContainerLockGuard::set_item`
уведомляет только владельца. Наблюдаемое следствие: воронка под готовым
компостером, у которой пять неполных слотов без места для муки. В Java
`removeItem` уже опустошил компостер, мука потеряна. В Vecta компостер оставался
полным. `InputContainer::set_item`, `OutputContainer::remove_item` и
`OutputContainer::set_item` теперь повторяют `SimpleContainer`. Тест
`a_failed_pull_still_empties_the_composter`.

Заодно число событий `BLOCK_CHANGE` при заборе стало как в Java: по два.

## Проверено и совпадает

- Шанс через `compostable/*` с `MatchBlock` на уровне 0; при отсутствии
  провайдера 0 слоёв.
- `consume(1, player)` и `ITEM_USED` до расхода, level event 1500 с флагом
  успеха, `TRY_WITH_EMPTY_HAND` для не-компостируемого предмета.
- `READY` по тику с `cycle`, звуки `COMPOSTER_READY`/`EMPTY`, `onPlace` на уровне 7.
- `EmptyContainer` на уровне 7: выбрасыватель сохраняет предмет, воронка
  считает его полным.
- Блокировка RNG: в путях воронки и выбрасывателя `with_level` не удерживается
  во время `set_changed`.

## Не исправлено

- `Compostable`, как и `CookingFuel`, хранит `Identifier` без inline-формы
  `ResolvableInt`.
- Форма блока (`getShape` по уровню, `getInteractionShape`,
  `getCollisionShape`) в этом срезе не сверялась.
- Клиент 26.3 вживую не проверялся.

## Проверки

- Узкий прогон `composter`/`hopper`/`dropper`: 63 passed.
- `cargo test --locked --workspace -j 1`: 4342 passed, 0 failed, 18 ignored.
- `cargo clippy --locked --workspace --all-targets --all-features -j 1 -- -D warnings`: exit 0.
- `cargo fmt --all --check`: exit 0.
