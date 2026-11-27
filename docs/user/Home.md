# EPlatform

Клиентская платформа форм в духе **Qt Widgets**: `.ui` из Designer, логика на **Lua**, сессии на **Phoenix/Elixir**, отрисовка в браузере (**Rust / wasm / wgpu**).

Это **не полный Qt** и не биндинги к libQt. Это своя реализация нужного подмножества семантики (layout, виджеты, Fusion-like стиль, QSS-подмножество, MDI/shell).

## С чего начать

1. [[Quick Start]] — поднять сервер, открыть форму в браузере
2. [[Supported Widgets]] — что можно класть в `.ui` *(автоген)*
3. [[Lua Form API]] / [[Lua App API]] / [[Host Lua API]] — API *(автоген)*
4. [[Architecture]] — слои системы
5. [[Rendering Pipeline]] / [[Paint Commands]] — отрисовка *(enum’ы автоген)*

Справочники виджетов, Lua и paint-команд **пересобираются** из кода (`scripts/gen_user_docs.py`); обзорные страницы правятся вручную в `docs/user/`.

## Для кого эта Wiki

| Роль | Что читать |
|------|------------|
| Автор форм / Lua | Quick Start, Widgets, Form/App API |
| Интегратор / архитектор | Architecture, Rendering Pipeline |
| Контрибьютор rqt | инженерные `docs/` в репо + parity guide |

Исходники этих страниц: [`docs/user/`](https://github.com/VitaSound/EPlatform/tree/main/docs/user) в основном репозитории. Wiki обновляется скриптом `scripts/sync_wiki.sh`.
