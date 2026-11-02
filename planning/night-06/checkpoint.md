# Checkpoint пакета 06

Обновление после сеанса 2 (G006…G013), 2026-11-02.
Продолжать actual tip `codex/night-vanilla-03`, не откатываться к историческим SHA.

| Поле | Состояние |
| --- | --- |
| Игровой import SteelMC | `3a1fd4815` |
| Docs предыдущего пакета | `7fb7c8bf0` |
| Minecraft target | Cargo.toml `0.16.1+mc26.3` |
| Local source | 26.3, 5037 Java, server SHA1 `33680f5f2ac32864d6d7cf5e56a705fdb3e05f4c` |
| Вход сеанса 2 | `cfc95d0a3`, workspace 4198 passed / 0 failed / 18 ignored |
| Код сеанса 2 | `58bfb86db`, `b28f141c0`, `d14a4b01b`, `65d3817d1`, `fb9ec81ed`, `e485e254a`, `1f277a4b7` |
| Свежий workspace | **4240 passed / 0 failed / 18 ignored**, exit 0 |
| Свежие узкие проверки | dispenser 79; bucket 26; equipment 11; flint 8; bonemeal 4; bottle 7; shears 4; empty_bucket 8 |
| Clippy/fmt | exit 0 (`1f277a4b7` устранил три находки в новом коде) |
| Новый пакет | 120 направлений / 600 карточек |
| Закрытые группы | G001–G003 (ревью), G006, G007, G009 (body-slot partial), G010, G011, G012, G013 |
| Следующий consumer | **G021 / W06-101…105: implicit components round-trip** (см. B-06-04: сначала hook `apply_components`) |

[Отчёт сеанса 2](report-next-02.md), [полное ТЗ](../../NIGHT_TASK_06.md),
[маршрут](route.md), [Java index](reference-index.json),
[blockers](blockers.md), [progress](progress.md).

Не перезапускать прежний baseline без причины. Сначала actual branch/status и
выбранный consumer. Группы 001–100 раскрывают старые W05-001…100; группы 101–120
добавляют новые области. Нумерация задаёт объём поведения, не требует 600
commits и не доказывает 600 missing features. Если путь уже готов, доказать
значимый сценарий и сохранить код.

Крупные подтверждённые ограничения: большинство special dispenser consumers,
arrows/оружие, maps/recipe book, mobs/AI и full survival остаются частичными.
TNT engine, containers/menus, shared RNG/world order и перечисленные в reviews
fixes сохранять. Runtime scoreboard/team read access требует архитектурного
решения после проверки по AGENTS. EULA/client/Linux и собственный Mojang server
comparison не проверялись.

Открытые partial этого сеанса: W06-042 (body slot, нет волка/лошади),
W06-046 (fire charge → G047), W06-052 (`TreeGrower`, B-06-01),
W06-057 (bee release, B-06-03), W06-062 (соты, B-06-03).
G021 не начата: B-06-04 фиксирует двойную запись имени/lock и отсутствие
`applyComponents` при reload; это отдельный срез по всем контейнерам.

Foreign dirty file: `planning/night-01/report.md`; не трогать.
SHA256 `06680CBC971245B70C990D95C906F0F1E3685EFFDE61C13D562664B4B6B05222`.

Foreign dirty `WORKBUDDY_START_06.md` также не трогать и не stage.
SHA256 `48430DC1D56AF1E679E0A81411518002B8C406B3F27648AD1A197DF174AFAD5A`.

G004 зависит от G005, обе не начаты; отсутствие consumer само по себе не внешний
blocker. Fire charge из dispenser требует SmallFireball/G047, прямое зажигание
его не заменяет. 6/13 projectile item registrations (snowball, три яйца,
experience bottle, splash potion; fire charge ждёт G047, wind charge ждёт G048)
не означают готовность всех projectile механик. Record-codec/factory tests не
выдавать за живой restart/unload или сетевых клиентов.

Продолжать существующие progress.md/blockers.md. Исторический `report.md` не
переписывать. Не делать сотни placeholder отчётов.
