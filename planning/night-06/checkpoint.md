# Checkpoint пакета 06

Обновление после G001–G003 и ревью Codex, 2026-11-02.
Продолжать actual tip `codex/night-vanilla-03`, не откатываться к историческим SHA.

| Поле | Состояние |
| --- | --- |
| Игровой import SteelMC | `3a1fd4815` |
| Docs предыдущего пакета | `7fb7c8bf0` |
| Minecraft target | Cargo.toml `0.16.1+mc26.3` |
| Local source | 26.3, 5037 Java, server SHA1 `33680f5f2ac32864d6d7cf5e56a705fdb3e05f4c` |
| Историческая база до G001 | 4185 passed / 0 failed / 18 ignored |
| Входящий tip G001–G003 | `78da455e5`; агент сообщил 4197 passed / 0 failed / 18 ignored |
| Код ревью Codex | `1fc3e504a` (tests и комментарий); продолжать actual tip, включая docs |
| Свежий workspace Codex | **4198 passed / 0 failed / 18 ignored**, exit 0 |
| Свежие узкие проверки | dispenser_block: 27 passed; splash_potion: 5 passed |
| Clippy/fmt Codex | exit 0; затронутый round-trip после test-only lint fix — 1 passed |
| Новый пакет | 120 направлений / 600 карточек |
| Закрытый slice | G001–G003: bottle, egg/blue/brown, splash dispenser registrations |
| Следующий consumer | **G006, W06-026…030: наполненные вёдра** |

[Новый запрос](../../WORKBUDDY_CONTINUE_06_02.md), [полное ТЗ](../../NIGHT_TASK_06.md),
[маршрут](route.md), [Java index](reference-index.json),
[ревью G001–G003](../reviews/night-06-g001-g003-codex.md).

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

Foreign dirty `WORKBUDDY_START_06.md` также не трогать и не stage.
SHA256 `48430DC1D56AF1E679E0A81411518002B8C406B3F27648AD1A197DF174AFAD5A`.

G004 зависит от G005, обе не начаты; отсутствие consumer само по себе не внешний blocker.
Fire charge из dispenser требует SmallFireball/G047, прямое зажигание его не заменяет.
6/13 projectile item registrations не означают готовность всех projectile механик.
Record-codec/factory tests не выдавать за живой restart/unload или сетевых клиентов.

Продолжать существующие progress.md/blockers.md. Следующий итог — report-next-02.md;
исторический report.md не переписывать. Не делать сотни placeholder отчётов.
