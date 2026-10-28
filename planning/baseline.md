# Исходные результаты проверок (V00)

Снимок состояния перед началом работ по ванильному паритету.
Коммит: `a2db838b68ac4e21d9626aa1f8b146126911f2b3`.
Версия workspace: `0.16.1+mc26.3` (`Cargo.toml`).

## Среда

| Параметр | Значение |
| --- | --- |
| ОС | Windows (`win32`) |
| Rust | `rustc 1.101.0-nightly (822182db4 2026-10-13)` |
| Cargo | `cargo 1.101.0-nightly (29c5daa1a 2026-10-07)` |
| Toolchain | `nightly-2026-10-14` из `rust-toolchain.toml` |
| Эталон | `minecraft-src/` — 5037 `.java`, совпадает с `minecraft-src/.vecta-reference.json` |
| Python | 3.13 |

Linux в этой среде недоступен; проверки Linux не выполнялись и не заявляются.

## Команды и результаты

```sh
cargo check --locked --workspace
# Finished `dev` profile in 1m 53s — без ошибок и предупреждений

cargo test --locked --workspace
# 31 набор, 3725 passed, 0 failed, 18 ignored

cargo fmt --all --check
# exit 0

cargo clippy --locked --all-targets --all-features -- -D warnings
# exit 0; единственное сообщение — manifest warning `unused dependency mimalloc`
# в vecta/Cargo.toml, существовавшее до этих изменений

python -m unittest discover -s scripts -p "test_*.py" -v
# Ran 4 tests — OK
```

## Что это доказывает и что нет

Эти результаты подтверждают только воспроизводимость сборки, тестов и линтеров
на указанном коммите. Они не подтверждают ванильное поведение: наличие
регистрации, зелёная компиляция и unit-тесты не являются доказательством
полноты механики. Настоящий клиент 26.3 в этой среде не запускался.
