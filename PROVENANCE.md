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
