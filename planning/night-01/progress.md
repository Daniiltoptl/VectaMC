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

