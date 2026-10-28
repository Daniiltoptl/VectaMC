# Ночной сеанс 01 — прогресс

## База сеанса

| Параметр | Значение |
| --- | --- |
| Ветка | `codex/night-vanilla-01` |
| Исходный коммит | `180ee147ed50cac0f13ad2dd53d045e0efa54466` |
| Родительская ветка | `codex/gemini-v05-09a` |
| Версия workspace | `0.16.1+mc26.3` |
| Эталон | Minecraft Java 26.3, `minecraft-src/.vecta-reference.json`, 5037 `.java` |
| Rust | `rustc 1.101.0-nightly (822182db4 2026-10-13)` |
| Cargo | `cargo 1.101.0-nightly (29c5daa1a 2026-10-07)` |
| Java | Temurin OpenJDK 25.0.4 |
| ОС | Windows |
| Каталог сборки | `D:\CodexBuilds\VectaMC`, `CARGO_INCREMENTAL=0` |
| Свободно на момент старта | C: 23 ГБ, D: 6.2 ГБ |

Рамки Gemini (`b22e55102`) и исправления Codex (`0fbbc5978`) входят в базу;
они не переписываются. Открытые части рамок: карты (N18/V05.07), entity
pick-block (N19), командный доступ к слоту (V12).

## Журнал этапов

Записи добавляются по мере закрытия задач: ID, коммит, что сделано,
команды проверок и результат.

### N00 · Воспроизводимая база — проверено

Baseline выполнен один раз на исходном коммите `180ee147e`:

```sh
cargo test --locked --workspace -j 1
```

Результат: **3748 passed, 0 failed, 18 ignored**, exit 0, 4m49s. Совпадает
с числом из `planning/reviews/V05.09a.md`, поэтому старые ошибки можно
отличать от регрессий.

Среда сборки: `CARGO_TARGET_DIR=D:\CodexBuilds\VectaMC`, `CARGO_INCREMENTAL=0`,
`TMP`/`TEMP=D:\CodexBuilds\tmp` (диск C: ранее переполнялся, temp линкера
на нём же). Логи — в `D:\CodexBuilds\logs\`.

Предел проверки: Linux и настоящий клиент 26.3 недоступны; EULA не принималась.

### N02 · Мешки всех вариантов — реализовано

Коммиты: `ca764fa5d` (реализация), `ac6d04fb5` (проверки).

- `try_item_click_behavior_override` в `MenuBehavior` повторяет
  `AbstractContainerMenu.tryItemClickBehaviourOverride`: сначала поведение
  предмета на курсоре, затем поведение предмета в слоте. Обработанный клик
  сразу рассылается клиенту (`broadcastChangesOnContainerMenu`).
- `BundleItem` реализует оба направления клика, `use`, анимацию `Bundle`,
  200 тиков и выбрасывание содержимого с ванильными звуками и статистикой
  `ITEM_USED`. Используется общий `as_bundle` hook, отдельные копии для
  вариантов мешка не создавались.
- `BundleContents` хранит runtime-индекс выбора (кодек, NBT и `HashComponent`
  его не пишут), получил `as_mutable`/`BundleContentsMut` с ванильными
  дробными весами (`Fraction.ONE.subtract(weight)`), `try_insert`,
  `max_amount_for`, `remove_one` и `toggle_selected_item`.
- Новый пакет `ServerboundSelectBundleItemPacket` (`S_BUNDLE_ITEM_SELECTED`,
  id 3) декодируется с проверкой `index >= -1` и применяется к открытому меню
  через `Menu::set_selected_bundle_item_index`.
- Проверки: `cargo test -p vecta-core --lib` — 2883 passed, 1 ignored;
  `cargo test -p vecta-protocol --lib` — пакет и валидация индекса;
  `cargo test -p vecta-registry --lib bundle` — 5 passed;
  `cargo clippy -p vecta-core -p vecta-registry -p vecta-protocol
  --all-targets --all-features -- -D warnings` — чисто; `cargo fmt --all`.

Предел проверки: настоящий клиент 26.3 недоступен, поэтому синхронизация
выбора проверена только на уровне пакета и меню, а не визуально.

### N01 · Инвентарь — частично

Коммит: `bfbdd45e1`.

- Сверены с `AbstractContainerMenu.doClick` и
  `ServerGamePacketListenerImpl.handleContainerClick`: порядок обработки
  пакета клика, `state_id`, `suppressRemoteUpdates`/`broadcastFullState`,
  `do_pickup`, `do_quick_move`, `do_swap`, `do_throw`, `do_clone`,
  `do_pickup_all`, quick-craft. Расхождений в порядке операций не найдено;
  `Slot.safeClone` в 26.3 не проверяет `mayPickup`, поэтому `do_clone`
  соответствует эталону.
- Исправлен дефект: `QUICK_MOVE` с `slot_num == -999` отвергался, хотя
  vanilla выбрасывает предмет с курсора (ветка `-999` общая для `PICKUP`
  и `QUICK_MOVE`). Тест в `inventory/click.rs`.
- Не сделано: TODO `ResultSlot::on_take` (учёт crafted count и `ITEM_CRAFTED`),
  сверка закрытия меню при смерти/disconnect, уничтожения контейнера
  и сценариев с несколькими игроками.


