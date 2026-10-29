# Происхождение VectaMC

VectaMC основан на [SteelMC](https://github.com/Steel-Foundation/SteelMC).

| Поле | Значение |
| --- | --- |
| Исходный репозиторий | `https://github.com/Steel-Foundation/SteelMC.git` |
| Исходная ветка | `master` |
| Базовый коммит | `65cd7ab416bb2bbe04d4aca3b6d04b1d939d5a8d` |
| Версия workspace при импорте | `0.16.1+mc26.3` |
| Дата импорта | 2026-10-28 |
| Лицензия | AGPL-3.0-or-later |

История Git, авторство исходных файлов и [LICENSE](LICENSE) сохранены.
Исходный README сохранён в [UPSTREAM.md](UPSTREAM.md).
После импорта внутренние crates переименованы в `vecta-*`, исполняемый файл —
в `vecta`, пользовательское название — VectaMC. Названия SteelMC в этом документе
и UPSTREAM.md указывают на происхождение кода.

## Перенесённые исправления SteelMC · 2026-10-30

После развилки отдельно адаптированы следующие upstream-коммиты под `vecta-*`
и существующие Vecta menu/item contracts; это не полный merge ветки SteelMC.

| Upstream commit | Изменение |
| --- | --- |
| [3e75305c4](https://github.com/Steel-Foundation/SteelMC/commit/3e75305c478e0ac2a0744a47876be56a61a9aaa2) | Подбор предметов в рассчитанной позиции игрока до восстановления позиции тика |
| [feb5a7a10](https://github.com/Steel-Foundation/SteelMC/commit/feb5a7a10e6d716922580b976ed9dda6608a331f) | Movement emissions: убрать звуки шагов у предметов и других затронутых сущностей |
| [1770402c2](https://github.com/Steel-Foundation/SteelMC/commit/1770402c217484f1e92038313ae4412a5e1f4a3b) | Звуки экипировки через меню, creative slot packet и использование предмета |

Авторы этих исправлений: AshuraVie / AshuraV1 и CuzImClicks, как записано
в исходных коммитах. Vanilla-эталон остаётся 26.3; данные и generated Rust
этот перенос не меняет. Подробности проверок — в
[отчёте переноса](planning/reviews/upstream-2026-10-30.md).

## Локальный ванильный эталон

`update-minecraft-src.sh` вызывает `scripts/prepare_vanilla.py`. На Windows
можно запускать Python-скрипт напрямую. Версия берётся из `[workspace.package]`
в `Cargo.toml`; использование неподходящей версии останавливает подготовку.

Скрипт получает сервер из официального manifest Mojang, проверяет SHA-1 и
размер, затем декомпилирует его Vineflower 1.12.0. Загрузка Vineflower также
проверяется по SHA-256 из Maven Central. Полученные Java-файлы и JSON с
версией, URL и контрольными суммами находятся в `minecraft-src/`.

`minecraft-src/` и `.vecta-cache/` игнорируются Git. Бинарники и декомпилированный
код Minecraft не публикуются в репозитории VectaMC. Эталон используется
локально для изучения поведения и проверки реализации на Rust.
