# Изменения VectaMC

## 2026-10-28 · Начало разработки

- Импортирован SteelMC `65cd7ab416bb2bbe04d4aca3b6d04b1d939d5a8d`
  с историей и лицензией; цель — Minecraft Java 26.3.
- Оформлены README, план этапов и сведения о происхождении проекта.
- CI адаптирован к VectaMC: сборка/тесты Linux и Windows, Rustfmt, Clippy,
  проверки Python-инструментов. Удалены workflows публикации и ботов SteelMC.
- Подготовка ванильного эталона заменена на Python + Vineflower без Gradle:
  проверка загрузок, точная версия из Cargo.toml, локальный provenance-файл.
- Базовый `cargo check -p steel-core --locked` прошёл на Windows.

Это начальный этап разработки, а не релиз с полной ванильной совместимостью.
