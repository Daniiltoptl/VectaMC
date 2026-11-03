# Checkpoint пакета 06

Обновление сеанса7, 2026-11-03. Проверки/код текущего среза — в report-next-07.
Продолжать actual tip `codex/night-vanilla-03`, не откатываться к историческим SHA.

| Поле | Состояние |
| --- | --- |
| Игровой import SteelMC | `3a1fd4815` |
| Docs предыдущего пакета | `7fb7c8bf0` |
| Minecraft target | Cargo.toml `0.16.1+mc26.3` |
| Local source | 26.3, 5037 Java, server SHA1 `33680f5f2ac32864d6d7cf5e56a705fdb3e05f4c` |
| Вход сеанса 2 | `cfc95d0a3`, workspace 4198 passed / 0 failed / 18 ignored |
| Код сеанса 2 | `58bfb86db`, `b28f141c0`, `d14a4b01b`, `65d3817d1`, `fb9ec81ed`, `e485e254a`, `1f277a4b7` |
| Входящий workspace исполнителя | **4240 passed / 0 failed / 18 ignored**, по его отчёту |
| Ревью сеанса 2, узкие проверки | dispenser 83 passed; normal_nbt_reload 1 passed |
| Ревью сеанса 2, workspace | **4245 passed / 0 failed / 18 ignored**, exit 0 |
| Сеанс 3 (G021) | `69bcdde45`; workspace **4252 passed / 0 failed / 18 ignored**, fmt/Clippy exit 0 |
| Сеанс 4 (G022) | `/data` для block/storage, foundation путей NBT; workspace **4273/0/18**, fmt/Clippy exit 0 |
| Ревью G022 | `cd11aa81a`; workspace **4279/0/18**, utils 228, paths 31 |
| Сеанс 5 | placement context, настоящий region I/O, `/data entity`; workspace **4285/0/18** |
| Сеанс 6 | G024 living load/save + disk, Team/UUID, break callback; workspace **4293/0/18** |
| Ревью сеанса6 | `a91820a33`: numeric/partial codecs, packed attributes, raw effects, sleeping, UUID/combat refs; workspace **4299/0/18**, Clippy/fmt exit0, Python4, audit3105/49/230 |
| Ревью сеанса5 | `1a8bb2457`: player guard, single target, numeric NBT, sync/cooldown; workspace **4288/0/18**, entity7, Clippy/fmt exit0 (Cargo warning см. review) |
| Ревью G022, Codex | `cd11aa81a`; workspace **4279/0/18**, fmt/Clippy exit0; Cargo manifest warning см. review |
| Статус G021 | gate, placement read context и реальный region I/O реализованы; live restart/ticket lifecycle не проверены |
| Первый срез ревью G021 | workspace **4252/0/18**, block_item 1 passed, item_block_entity_data 2 passed; Clippy/fmt exit 0 |
| Authorization slice | workspace **4254/0/18**, gate tests 2 passed, ordinary/type-mismatch 1 passed; Clippy/fmt exit 0 |
| Код authorization slice | `673c7a558`, produced metadata + generator + реальные gates/tests |
| Код первого среза ревью G021 | `8b581a658`; B-06-05 затем снят, gate идёт отдельным срезом |
| Clippy/fmt Codex | exit 0 |
| Исправления Codex | `df47b146e`; документы следуют отдельно, продолжать actual tip |
| Новый пакет | 120 направлений / 600 карточек |
| Реализованные пути | G001–G003, G006/G007 и consumers G009…G013; все partial см. blockers/review |
| Сеанс7, первый срез | `2a0ef1daf`: typed ContextMap,16keys/31sets, все текущие callers; workspace **4304/0/18**, Clippy/fmt exit0, Python4, audit3105/49/230 |
| Следующий consumer | **Одобренный общий domain read context** с Arc scoreboard/storage, затем providers/compute и направления 3–12 |

[Отчёт сеанса 2](report-next-02.md), [полное ТЗ](../../NIGHT_TASK_06.md),
[маршрут](route.md), [Java index](reference-index.json),
[blockers](blockers.md), [progress](progress.md).

[Codex review сеанса 2](../reviews/night-06-g006-g013-codex.md),
[новый запрос](../../WORKBUDDY_CONTINUE_06_03.md).

[Ревью G021](../reviews/night-06-g021-codex.md),
[запрос после G021](../../WORKBUDDY_CONTINUE_06_04.md).

[Ревью G022](../reviews/night-06-g022-codex.md),
[актуальный большой запрос](../../WORKBUDDY_CONTINUE_06_05.md).
B-06-05 снят: produced block_entities output уже получен, 26.3 / 49 типов.
Не повторять диагностические попытки и не подменять данные ручным списком.

Не перезапускать прежний baseline без причины. Сначала actual branch/status и
выбранный consumer. Группы 001–100 раскрывают старые W05-001…100; группы 101–120
добавляют новые области. Нумерация задаёт объём поведения, не требует 600
commits и не доказывает 600 missing features. Если путь уже готов, доказать
значимый сценарий и сохранить код.

Крупные подтверждённые ограничения: большинство special dispenser consumers,
arrows/оружие, maps/recipe book, mobs/AI и full survival остаются частичными.
TNT engine, containers/menus, shared RNG/world order и перечисленные в reviews
fixes сохранять. Существующие read contexts и scoreboard/team callers сначала
проверять; задавать вопрос только о реально неясном контракте. EULA/client/Linux и собственный Mojang server
comparison не проверялись.

Открытые partial этого сеанса: W06-042 (body slot, нет волка/лошади),
W06-046 (fire charge → G047), W06-052 (`TreeGrower`, B-06-01),
W06-057 (bee release, B-06-03), W06-062 (соты, B-06-03).
G021 дополнена placement book context и настоящим RegionManager round-trip:
явные компоненты сохраняются и читаются. Live restart/unload/tickets не проверены.
B-06-04 снят: Java тоже хранит корневые имя/lock и не применяет item components
при обычном reload; формат контейнеров не менялся. Гейт `onlyOpCanSetNbt`
реализован на produced metadata и существующем OP contract. Реальный chunk save
через `ChunkMap` остаётся открытым acceptance-сценарием.
G022 реализована как builtin `/data` для block/storage (`get`/`merge`/`remove`/
`modify` × `from`/`string`/`value`). Entity provider и supported living load
реализованы; Brain/locator-bar icon остаются partial, compute отсутствует (B-06-07).
B-06-09 снят: placement read context реализован. Team API уже есть в scoreboard.
Актуализация сеанса6: общий supported living loader и disk путь реализованы,
ревью исправило codecs/partial results, operation indices атрибутов, raw effect
load, sleep и combat refs. Полной G024 ещё нет: Brain/locator-bar icon не готовы.
B-06-08 оказался ложной диагностикой: comparator input для контейнера уже есть,
добавлен сквозной regression; см. ревью G022 и его результаты проверок.
`Player::server()` уже позволяет провести read context без World→Server pointer.
Дополнительно открыты water
evaporation, underwater coral selection и SulfurCube ignition; см. blockers.

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

Актуальное [ревью сеанса5](../reviews/night-06-session05-codex.md) и
[большой запрос06](../../WORKBUDDY_CONTINUE_06_06.md) уточняют приоритеты файла05.

Актуальные [ревью сеанса6](../reviews/night-06-session06-codex.md) и
[большой запрос07](../../WORKBUDDY_CONTINUE_06_07.md) имеют приоритет над старыми
диагнозами отсутствующего living loader и запрета extractor для providers.
TYPE/configured provider registries реальны; данные получать штатным extractor.
Единая context-map архитектура уже согласована; реализация ещё впереди.
