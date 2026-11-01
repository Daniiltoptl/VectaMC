# Night 06 — progress

Ветка `codex/night-vanilla-03`. Стартовый HEAD `3a79ec57f`
(`docs: expand vanilla implementation specification to 600 work items`).
Эталон: локальная Java 26.3, `minecraft-src/.vecta-reference.json`, 5037 файлов,
workspace `0.16.1+mc26.3`.

Не трогается: `planning/night-01/report.md` (чужое изменение) и
`WORKBUDDY_START_06.md` (чужая незакоммиченная правка порядка строк).

## G001 / W06-001…005 — пузырёк опыта из раздатчика

Старый ID: W05-001, продолжение W04-032. Статус: **реализовано и проверено**
на серверном уровне; настоящий клиент не запускался.

### Что было

`ThrownExperienceBottleEntity` и `ExperienceBottleItem` существовали и работали
(полёт, гравитация 0.07, события 2002/1053, опыт `3 + nextInt(5) + nextInt(5)`,
discard, NBT). Раздатчик знал только снежок: в
`register_projectile_dispense_behaviors` была одна регистрация, поэтому импульс
по раздатчику с пузырьком уходил в default-dispatch и выбрасывал предмет как
item entity.

### Что изменилось для игрока

Импульс раздатчика с пузырьком опыта теперь запускает настоящий
`ThrownExperienceBottle` с ванильной конфигурацией, расходует один предмет и
даёт те же события, что Java. Попадание разбивает пузырёк и выдаёт опыт через
существующий consumer.

### Ванильные методы

- `world/item/ExperienceBottleItem.java`: `createDispenseConfig` (uncertainty
  `DEFAULT.uncertainty * 0.5` = 3.0, power `DEFAULT.power * 1.25` = 1.375,
  position function по умолчанию).
- `core/dispenser/ProjectileDispenseBehavior.execute` (позиция → shoot → shrink 1)
  и `playSound` (событие 1002; анимация 2000 наследуется).
- `world/entity/projectile/throwableitemprojectile/ThrownExperienceBottle.onHit`
  (события 2002/1053, опыт, discard).

### Изменения Vecta

- `behavior/blocks/redstone/dispenser_block.rs`: регистрация
  `vanilla_items::EXPERIENCE_BOTTLE` в `register_projectile_dispense_behaviors`
  с `DispenseConfig { uncertainty: DEFAULT_UNCERTAINTY * 0.5, power: DEFAULT_POWER * 1.25, ..DEFAULT }`.
- Тестовый helper `snowballs` обобщён до `projectiles_of_type(fixture, pos, entity_type)`
  и переиспользован снежком, экипировкой и пузырьком.
- `tests/experience_bottle.rs` (новый): три сценария раздатчика.
- `thrown_experience_bottle/tests.rs`: сценарий «загруженный пузырёк всё ещё
  выдаёт опыт и удаляется при попадании».

### Карточки

| ID | Результат |
| --- | --- |
| W06-001 | запуск: позиция `center + facing*0.7 + (0,0.1,0)`, power 1.375, uncertainty 3.0 — тест `dispensed_experience_bottle_launches_one_projectile_from_the_java_position` |
| W06-002 | компоненты: стак с `CUSTOM_NAME` переносится на снаряд с count 1, исходный теряет ровно 1 — тест `..._keeps_the_dispensed_stack_components` |
| W06-003 | попадание и опыт через существующий `ExperienceOrbEntity::award_with_direction` — уже покрыто `block_impact_awards_experience_toward_each_hit_face_and_discards_bottle` и `entity_impact_...` |
| W06-004 | события 1002 + 2000 и расход при rejected insertion без возврата предмета — тест `rejected_experience_bottle_insertion_still_consumes_and_emits_events` |
| W06-005 | save/load сохраняет состояние (существующий `saved_bottle_preserves_shooter_and_rendered_item_components`), попадание после load выдаёт опыт и удаляет снаряд — тест `a_loaded_bottle_awards_experience_and_is_discarded_on_hit` |

### Проверки

```
cargo check -p vecta-core --lib --tests -j 1                     → exit 0
cargo test -p vecta-core --lib dispenser -j 1                    → 34 passed, 0 failed
cargo test -p vecta-core --lib experience_bottle -j 1            → 8 passed, 0 failed
cargo fmt --all --check                                          → exit 0
```

Логи: `D:\CodexBuilds\logs\w06-g001-*.log`. Прежний фильтр раздатчика был
31 passed (ревью W04-019-034); 31 + 3 новых = 34.

### Границы

- Клиент 26.3 не запускался: стартовый звук/частицы и полёт проверены на уровне
  событий и состояния, не глазами.
- Опыт выдаётся через `award_with_direction`; раскладка орбов по попаданию
  проверена существующими тестами, но не сверялась с живым Mojang-сервером.
