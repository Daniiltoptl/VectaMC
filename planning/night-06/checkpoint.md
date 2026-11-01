# Checkpoint пакета 06

Это расширение документации; игровой код в сеансе подготовки пакета 06 не менялся.
Продолжать actual tip `codex/night-vanilla-03`, не откатываться к SHA ниже.

| Поле | Проверенная исходная точка |
| --- | --- |
| Игровой import SteelMC | `3a1fd4815` |
| Docs предыдущего пакета | `7fb7c8bf0` |
| Minecraft target | Cargo.toml `0.16.1+mc26.3` |
| Local source | 26.3, 5037 Java, server SHA1 `33680f5f2ac32864d6d7cf5e56a705fdb3e05f4c` |
| Последний игровой workspace | 4185 passed / 0 failed / 18 ignored |
| Последний игровой Clippy/fmt | exit 0, подробности в upstream review |
| Новый пакет | 120 направлений / 600 карточек |
| Начать | G001, W06-001…005: experience bottle dispenser |

[Стартовый запрос](../../WORKBUDDY_START_06.md), [полное ТЗ](../../NIGHT_TASK_06.md),
[маршрут](route.md), [Java index](reference-index.json).

Не перезапускать прежний baseline без причины. Сначала actual branch/status и выбранный consumer.
Группы 001–100 раскрывают старые W05-001…100; группы 101–120 добавляют новые области.
Нумерация задаёт объём поведения, не требует 600 commits и не доказывает 600 missing features.
Если путь уже готов, доказать значимый сценарий и сохранить код.

Крупные подтверждённые ограничения: большинство special dispenser consumers,
arrows/оружие, maps/recipe book, mobs/AI и full survival остаются частичными.
TNT engine, containers/menus, shared RNG/world order и перечисленные в reviews fixes сохранять.
Runtime scoreboard/team read access требует архитектурного решения после проверки по AGENTS.
EULA/client/Linux и собственный Mojang server comparison новым пакетом не проверялись.

Foreign dirty file: `planning/night-01/report.md`; не трогать.
SHA256 `06680CBC971245B70C990D95C906F0F1E3685EFFDE61C13D562664B4B6B05222`.

Для дальнейшей работы создавать progress.md/blockers.md/report.md по фактическим результатам,
не считать отсутствие пустого журнала blocker и не делать сотни placeholder отчётов.
