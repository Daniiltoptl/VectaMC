# Работа в Arena (облачная песочница)

Этот файл главнее всех остальных брифов там, где речь о среде. Очередь задач,
правила кода и чек-лист — в `AGENT_A.md` / `AGENT_B.md` / `AGENT_C.md`,
`COMMON.md`, `CHECKLIST.md` в этой же папке и в `AGENTS.md`.

## Что есть и чего нет

- Есть: git, Python, сеть к github.com и api.github.com, GitHub Actions по push.
- Нет: Rust/cargo, Java-исходников Minecraft (`minecraft-src/`), SteelExtractor,
  локальных папок `C:\Users\...` и `D:\CodexBuilds\...`. Всё, что в брифах
  говорит о них, в Arena не применяется. Не пытайся их установить или скачать.
- Писать можно только в свою ветку `arena/<id>`, которую выдала Arena.

## Начало

1. `git fetch origin`.
2. Влей в свою ветку исходную ветку своего агента (fast-forward или обычный
   merge, без rebase и force-push):
   - агент A: `origin/codex/night-vanilla-03`, затем `origin/wip/agent-a-2026-11-11`
     (незаконченная работа прошлого исполнителя, не компилировалась);
   - агент B: `origin/codex/night-vanilla-03-b`;
   - агент C: `origin/codex/night-vanilla-03-c`, затем `origin/wip/agent-c-2026-11-11`
     (незаконченный ИИ мобов, не компилировался).
3. Прочитай `AGENTS.md`, `planning/agents/COMMON.md`, `planning/agents/CHECKLIST.md`,
   свой `planning/agents/AGENT_*.md`, ревью `planning/reviews/`.
4. Первым срезом доведи wip до зелёного CI (A и C), B — сразу очередь.

## Сборка и тесты: только через CI

Каждый push в твою ветку запускает CI: `cargo check`, `cargo test --workspace`
(ubuntu и windows), `cargo fmt --check`, `cargo clippy -D warnings`. Логи
скачать нельзя, но при падении CI публикует строки ошибок и хвост лога как
аннотации, они читаются через API:

```bash
R=Daniiltoptl/VectaMC; B=$(git branch --show-current)
RUN=$(curl -s "https://api.github.com/repos/$R/actions/runs?branch=$B&per_page=1" \
  | python3 -c "import json,sys;r=json.load(sys.stdin)['workflow_runs'][0];print(r['id'],r['status'],r['conclusion'],r['head_sha'][:9])")
echo "$RUN"   # id status conclusion sha — жди status=completed
ID=${RUN%% *}
curl -s "https://api.github.com/repos/$R/actions/runs/$ID/jobs" \
  | python3 -c "import json,sys;[print(j['id'],j['name'],j['conclusion']) for j in json.load(sys.stdin)['jobs']]"
# для каждого упавшего job:
curl -s "https://api.github.com/repos/$R/check-runs/<JOB_ID>/annotations" \
  | python3 -c "import json,sys;[print(a['title'],'\n',a['message'],'\n') for a in json.load(sys.stdin)]"
```

Если у тебя есть токен в `GH_TOKEN`/`GITHUB_TOKEN`, добавляй
`-H "Authorization: Bearer $GH_TOKEN"` (без него лимит 60 запросов в час).
CI идёт 15–30 минут: пока ждёшь, читай код и готовь следующий срез, не сиди
без дела и не пушь каждую мелочь — собирай срез целиком, потом push.

Срез считается готовым только при зелёном CI на его коммите. Красный CI —
чини, прежде чем идти дальше.

## Без Java-исходников (решение руководителя)

Руководитель проекта разрешил работать без локального `minecraft-src`.
Ванильное поведение бери из: extracted данных в репозитории
(`*/build_assets/`, `*/test_assets/`), уже реализованного и проверенного кода
Vecta, ревью в `planning/reviews/`, своих знаний о Java-классе (называй класс
и метод). Каждое место, где ты опирался на знание Java без исходника, отметь в
отчёте списком «Сверить с Java: Класс.метод — что предполагалось». Claude
сверит это локально с настоящими исходниками на ревью.

Не выдумывай registry/protocol ID, числа из таблиц, теги и веса: если их нет
в extracted данных — это блокер («нужен extract: ...»), бери следующий пункт.

## Блокеры

Если среда что-то не даёт сделать (нет доступа, нет данных, CI падает не по
твоей вине) — запиши в отчёт точный текст ошибки и команду, и иди дальше.
Пользователь передаст это Claude, тот починит на своей стороне.

## Отчёт

`planning/agents/report-<A|B|C>-<id>.md` в своей ветке + ответ обычным текстом:
что сделано для игрока, коммиты, итог последнего CI (зелёный/красный по
job), блокеры, список «Сверить с Java».
