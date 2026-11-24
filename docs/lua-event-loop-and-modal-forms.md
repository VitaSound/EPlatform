# Lua event loop и модальные формы: `tcs_back@ref_qt` vs `EPlatform`

> Статус: исследовательская записка для дальнейших раздумий, не план миграции.
> Дата: 2026-11-24.
> Каталога `tcs_qui` нет; сравниваю с `../tcs_qtgui` (Qt-клиент).
> Основной «старый» код — ветка `ref_qt` в `../tcs_back`, а не `dev`/`master`.

## 0. Источники

- `../tcs_back@ref_qt:lib/ex_platform/lua/manager.ex` — event loop, очередь, `busy`, `open_opts`.
- `../tcs_back@ref_qt:lib/ex_platform/lua/api.ex` — `open_modal_lua`, `close_modal`, `invoke_qt`, `get_prop`, `mfa`, `list_*`.
- `../tcs_back@ref_qt:priv/lua_modules/form.lua` — `open_modal_lua`, `finish_modal`, `exec_qt`.
- `../tcs_back@ref_qt:priv/lua_modules/application.lua` — `open_lua`, `open_modal_lua`, `exec_qt`.
- `../tcs_qtgui/tcscore.cpp`, `mainwindow.cpp`, `invoke_mainwindow.cpp` — Qt-клиент, `invoke_qt`, модальность.
- `EPlatform/server/lib/e_platform/dialog_session.ex` — GenServer на форму.
- `EPlatform/server/lib/e_platform/dialog_manager.ex` — запуск/остановка сессий.
- `EPlatform/server/lib/e_platform/lua/api.ex` — текущая Lua-поверхность.
- `EPlatform/server/lib/e_platform_web/channels/platform_channel.ex` — транспорт и маршрутизация событий.

## 1. Краткий вывод

1. В `tcs_back@ref_qt` действительно есть отдельный, довольно проработанный event loop:
   очередь событий на форму, флаг `busy`, `open_opts`, модальные формы, `invoke_qt`, `mfa`, доступ к БД.
2. В `EPlatform` перенесён только базовый каркас: `start_dialog`, `set_property`, `get_property`, `close_form`, `close_application`.
3. Старый «неблокирующий» трюк — это **не отсутствие блокировки**, а **изоляция блокировки по форме**:
   - событие одной формы обрабатывается в отдельном BEAM-процессе (GraphQL resolver);
   - очередь и `busy` хранятся в ETS и привязаны к `{application_id, form_id}`;
   - модальный вызов блокирует только родительскую форму, а дочерняя форма продолжает жить в своём процессе;
   - результат модалки возвращается **сырым `send`/`receive`**, а не через `GenServer.call`.
4. Текущий `DialogSession` на GenServer — хорошая основа, но при наивном добавлении модалок легко получить дедлок:
   - если результат модалки слать через `GenServer.cast`/`GenServer.call`, родительский GenServer уже стоит в `receive` и не обработает его;
   - если запускать дочернюю форму через `GenServer.call` к родителю/менеджеру, можно получить циклическое ожидание.
5. В старом коде есть и слабые места: утечки ETS/Agent, отсутствие остановки Lua-состояния при закрытии, хрупкая очистка `busy` при падении процесса, отсутствие обработки ошибок старта модалки. Их не стоит копировать.

## 2. Сравнительная таблица

| Аспект | `tcs_back@ref_qt` | `EPlatform` сейчас | Комментарий |
|---|---|---|---|
| Хранение Lua-состояния | `Agent` + `Registry` | `DialogSession` GenServer + `Registry` | У EPlatform лучше контроль жизненного цикла |
| Event loop | ETS-очередь + `busy` на форму, обработка в процессе GraphQL-резолвера | mailbox GenServer на форму | GenServer сам является и очередью, и локом |
| Сериализация событий | `:ets.insert_new(:lua_form_busy, ...)` | один GenServer на форму | В GenServer отдельный lock не нужен |
| Очередь событий | `:lua_form_event_queue` (`:queue`) | mailbox GenServer | Явной очереди нет, порядок даёт mailbox |
| Запуск формы | `start_dialog`/`start_file`, `init` синхронно | `DialogManager.start_dialog`, `init` в `handle_continue` | У EPlatform `init` асинхронный |
| `open_opts` | есть, передаются в `init` | нет | Нужно для модалок/динамических форм |
| Модальные формы | `open_modal_lua`, `close_modal`, `modal_registry` | нет | Главный недопорт |
| Вложенные модалки | работают через отдельные `req_ref` и процессы | не проверено | Требует аккуратного raw `send` |
| Блокировка родителя | да, через `busy` + `receive` | нет модалок | В GenServer будет блокировка процесса |
| Результат модалки | `send(pid, {:modal_result, ref, result})` | нет | Нельзя делать `cast`/`call` |
| `invoke_qt` | есть, generic bridge | нет | Нужен для `setProperty`, `getProperty`, динамического UI |
| `get_prop` | через `invoke_qt` + `get_prop_response` | базовый `getProperty` | Разные протоколы |
| `create_form` | `mdi`/`modal` | только `ui_xml`/`form_id` | Нужны поля `mdi`/`modal` |
| `create_empty_form` | есть | нет | Для динамических форм |
| `open_lua` | есть | нет | Открытие формы без модалки |
| `list_*` (БД) | есть, Ecto | нет | Нет БД-слоя |
| `mfa` bridge | есть | нет | Опасный, но полезный для прототипов |
| `sleep` | есть | нет | Может понадобиться |
| `get_prop` timeout | 5 с, ETS-запись не удаляется | 5 с, без ETS | У EPlatform чище |
| `close_form` | только broadcast, Agent не останавливается | broadcast + `send(self(), :stop_form)` | EPlatform лучше, но есть нюанс с чужой формой |
| `close_application` | только broadcast | broadcast + остановка сессий через `Task` | EPlatform лучше |
| Закрытие вкладки | не отслеживает все сессии | `PlatformChannel.terminate` останавливает `form_ids` канала | Внутренние модалки могут не попасть в `form_ids` |
| Ошибки `init` | падение/исключение | `dialogError` + остановка | EPlatform лучше |
| Транспорт | GraphQL mutation/subscription | Phoenix Channels | Не главное отличие для Lua-логики |
| Клиент | Qt/C++ (`QUiLoader`, `QMdiArea`) | Rust `rqt` + wgpu | Разные рендереры, но семантика форм похожа |

## 3. Старый event loop: как он устроен

### 3.1. Поток события

Упрощённо:

```text
Qt-клиент
  → GraphQL mutation sendEvent / formEvent
  → Absinthe resolver
  → ExPlatform.Lua.Manager.on_event/1
  → ETS queue + busy
  → Lua.call_function(:onEvent, ...)
  → Lua API (set_property / get_prop / open_modal_lua / ...)
  → Absinthe.Subscription.publish
  → Qt-клиент получает command
```

Ключевое: `on_event` вызывается **в процессе GraphQL-резолвера**, а не в одном общем GenServer. Поэтому блокировка одного события не блокирует другие формы, если у них разные процессы и разные ключи очереди.

### 3.2. Очередь и `busy`

В `Lua.Manager` есть две ETS-таблицы:

- `:lua_form_event_queue` — очередь событий по ключу `{application_id, form_id}`;
- `:lua_form_busy` — флаг «эта форма сейчас обрабатывается».

Псевдокод:

```elixir
def on_event(%{command: "formEvent", ...} = payload) do
  key = {application_id, form_id}
  enqueue(key, payload)

  if :ets.insert_new(:lua_form_busy, {key, true}) do
    try do
      drain(key)
    after
      :ets.delete(:lua_form_busy, key)
    end
  else
    :queued
  end
end
```

`drain/1` вынимает события из очереди и вызывает `Lua.call_function(:onEvent, ...)`.

Смысл:

- одно событие на форму выполняется в один момент времени;
- если форма уже занята, событие просто ждёт в ETS-очереди;
- разные формы не мешают друг другу.

Слабые места:

- `busy` — это просто запись в ETS, а не монитор процесса. Если процесс, взявший `busy`, будет убит без выполнения `after`, запись может остаться. ETS-таблица принадлежит первому процессу, вызвавшему `ensure_tables/0`; при его смерти таблица исчезнет, но это уже другая история.
- очередь не супервизируется: при падении процесса-владельца ETS события теряются.
- `Lua.Manager.call_function/3` (если его использовать напрямую) обходит очередь и `busy`, то есть может гоняться с `on_event`.

### 3.3. Модальные формы

Старый код не пытается сделать модалку «неблокирующей вообще». Он делает так, чтобы **блокировался только родитель**, а сервер и дочерняя форма продолжали работать.

`open_modal_lua` в `Lua.Api`:

```elixir
:ets.insert(:global_request_registry, {req_ref, self()})

publish_ui_event(app_id, %{
  command: "invoke_qt",
  form_id: parent_id,
  target_id: "",
  method: "setProperty",
  args: %{"1" => "enabled", "2" => false}
})

{^app_id, modal_form_id} =
  LuaManager.start_file(app_id, lua_file, %{
    "modal" => true,
    "mode" => "mdi",
    "modal_request_ref" => req_ref
  })

try do
  receive do
    {:modal_result, ^req_ref, result} -> result
  after
    300_000 -> ... -1
  end
after
  publish_ui_event(app_id, %{
    command: "invoke_qt",
    form_id: parent_id,
    target_id: "",
    method: "setProperty",
    args: %{"1" => "enabled", "2" => true}
  })
end
```

Что происходит:

1. Родительский процесс кладёт `req_ref` в глобальный ETS и запоминает свой `pid`.
2. Клиенту отправляется `invoke_qt` — выключить родительскую форму.
3. Запускается дочерняя Lua-форма; её `init` выполняется **в том же процессе**.
4. Родительский процесс блокируется в `receive` и ждёт `{:modal_result, req_ref, result}`.
5. Дочерняя форма живёт в клиенте; её `create_form` консьюмит `open_opts` и кладёт `modal_request_ref` в `modal_registry`, а её события обрабатываются уже другими процессами GraphQL-резолверов.
6. При закрытии дочерняя форма делает `close_modal`, который по `request_ref` достаёт `target_pid` из `:global_request_registry` и отправляет сырое сообщение родителю:

```elixir
send(target_pid, {:modal_result, req_ref, result})
```

7. Родительский `receive` просыпается, `after` включает родителя обратно.

То есть «неблокирование» достигнуто за счёт:

- отдельного процесса на каждое событие/форму;
- отдельной очереди и `busy` на форму;
- сырого `send`/`receive` для результата модалки;
- клиентской модальности (`Qt::ApplicationModal` в `tcs_qtgui`).

### 3.4. Вложенные модалки

Схема для A → B → C:

```text
A: GraphQL resolver PA
   busy(A) = true
   open_modal_lua(B)
   receive {:modal_result, ref_B, ...}

B: GraphQL resolver PB
   busy(B) = true
   open_modal_lua(C)
   receive {:modal_result, ref_C, ...}

C: GraphQL resolver PC
   busy(C) = true
   ... пользователь закрывает C
   close_modal(C)
   send(PB, {:modal_result, ref_C, ...})
   → PB просыпается, закрывает B
   send(PA, {:modal_result, ref_B, ...})
   → PA просыпается, закрывает A
```

Важно: у каждого уровня свой `req_ref`, свой `pid` и своя запись в `modal_registry`. Поэтому вложенность работает, пока живы промежуточные процессы.

Но есть нюансы:

- **Глубина = длина цепочки заблокированных процессов.** Если модалки вложены глубоко, это стопка процессов, каждый из которых ждёт.
- **Ошибка старта дочерней формы.** В `open_modal_lua` `start_file` стоит до `try do receive ... after ... end`. Если `start_file` упадёт, `after` не выполнится: родитель останется выключенным, а запись в `:global_request_registry` останется.
- **Таймаут 300 секунд.** Если пользователь не закрыл модалку, родитель ждёт 5 минут. После таймаута родитель включается, но дочерняя форма и её состояние не обязательно закрываются.
- **`get_prop` внутри `init` дочерней формы.** В `start_file` `init` выполняется до `Agent.start_link`/регистрации. Если `init` вызовет `get_prop`, сервер отправит клиенту `invoke_qt`, а обратный `get_prop_response` не найдёт форму в `Registry`/`Agent`. Это потенциальный дедлок/таймаут.
- **Закрытие родителя, пока открыта дочерняя.** Если родительский процесс убит, дочерняя форма может остаться; её `close_modal` будет слать результат в мёртвый `pid`.

### 3.5. Почему это не блокирует весь сервер

Потому что нет единого «мозга» на все формы. Каждая форма имеет свой ключ, свою очередь и свой процесс обработки. Блокируется только тот процесс, который сейчас выполняет Lua-сценарий родителя. Остальные формы, каналы и клиенты живут дальше.

Это же является ответом на вопрос «как решён вопрос с неблокированием»: **не решён глобально, а разнесён по формам**. Блокировка локализована.

### 3.6. Слабые места старого решения

1. **ETS-очередь без супервизора.** Падение процесса-владельца может потерять очередь.
2. **`busy` без монитора.** Очистка только через `after`; при жёстком убийстве процесса запись может остаться.
3. **Утечка Lua-состояния.** `close_form` только публикует команду; `Agent`/Lua-состояние не останавливается.
4. **Утечка `:global_request_registry`.** При таймауте `get_prop` запись не удаляется; при ошибке `open_modal_lua` — тоже.
5. **Хрупкий порядок старта.** `start_file` до `try/after`; `init` до регистрации.
6. **Нет обработки ошибок модалки.** Ошибка `init` дочерней формы не возвращает результат родителю.
7. **Нет полноценной очистки при закрытии приложения.** `close_application` только публикует команду.
8. **`close_form` не различает «свою» и «чужую» форму.** В Lua это глобальная функция, но по смыслу она всегда закрывает текущую форму.

## 4. Текущий `EPlatform`: что уже есть

### 4.1. GenServer на форму

```text
PlatformChannel
  → DialogManager.dispatch(application_id, form_id, payload)
  → GenServer.cast(DialogSession, {:dispatch, payload})
  → DialogSession.handle_cast
  → Lua.call_function(:onEvent, ...)
```

`DialogSession` — это и есть лок на форму: пока он выполняет `handle_cast`, следующие события этой формы ждут в mailbox. Это близко к старой очереди, но без ETS и без отдельного `busy`.

### 4.2. `getProperty` уже сделан через сырой `send`

`PlatformChannel` перехватывает `getProperty`/`getFormProperty` и вызывает:

```elixir
DialogManager.reply_property(application_id, form_id, ref, value)
```

А `reply_property` делает:

```elixir
send(pid, {:property_reply, ref, value})
```

Это важно: Lua-код внутри `DialogSession` ждёт `receive` с `ref`, и сырое сообщение доходит до него, даже если GenServer занят. Если бы здесь был `GenServer.cast`, оно бы встало в mailbox и не разбудило `receive`.

### 4.3. Чего нет

- модальных форм;
- `open_opts` и передачи опций в `init`;
- `open_lua` / `open_modal_lua` / `close_modal`;
- `invoke_qt` и клиентского моста;
- `create_empty_form`;
- `list_*` и БД;
- `mfa`;
- `sleep`;
- `get_prop` через `invoke_qt`;
- полноценной очистки при закрытии дочерних форм.

## 5. Риски при наивном портировании модалок в `DialogSession`

### 5.1. Главный дедлок: `cast`/`call` вместо `send`

Если родительский `DialogSession` стоит в `Lua.call_function` → `receive` и ждёт `{:modal_result, ref, result}`, то:

- `GenServer.cast(parent, {:modal_result, ...})` **не разбудит** `receive`; сообщение попадёт в mailbox и будет обработано только после возврата из `handle_cast`, которого не будет, пока `receive` не завершится. Дедлок.
- `GenServer.call(parent, ...)` из дочернего процесса тоже может привести к ожиданию родителя, который сам ждёт дочернего. Дедлок.

Правильно: `send(parent_pid, {:modal_result, ref, result})`, как в старом коде.

### 5.2. Запуск дочерней формы через `GenServer.call`

Если `open_modal_lua` вызывает `GenServer.call(DialogManager, ...)` или `GenServer.call(parent, ...)`, а менеджер/родитель в это время ждёт модалку, можно получить цикл. Запуск дочерней формы должен быть неблокирующим: `DynamicSupervisor.start_child` + возврат `{:ok, form_id}`.

### 5.3. `getProperty` и модалка

Если дочерняя форма вызывает `getProperty` во время `init`, а `init` выполняется в `handle_continue` дочернего GenServer, то дочерний GenServer уже зарегистрирован (в отличие от старого `Agent`), и сырой `send` с `:property_reply` дойдёт. Это плюс текущей схемы. Но если `init` запускается через `GenServer.call` из родителя, можно снова получить дедлок.

### 5.4. Закрытие родителя при открытой дочерней

Если родительский `DialogSession` останавливается, дочерний процесс может остаться. Нужно:

- либо мониторить родителя из дочерней сессии;
- либо при `terminate` родителя закрывать всех детей;
- либо хранить в `DialogManager` дерево `parent -> children`.

### 5.5. `PlatformChannel.terminate` и внутренние формы

Сейчас канал останавливает только `form_ids`, которые он сам запускал через `start_dialog`. Если модалки будут запускаться из Lua, их `form_id` не попадут в это множество. Нужно:

- добавлять все дочерние `form_id` в канал; или
- при `terminate` вызывать `DialogManager.stop_application(application_id)`, который смотрит в `Registry`.

### 5.6. `close_form` и «чужая» форма

Текущий `close_form` делает `send(self(), :stop_form)`. Если Lua вызовет `close_form` для другой формы, остановится текущая сессия. Для модалок это опасно. Нужно либо разделить `close_form` и `close_self`, либо передавать `form_id` и останавливать именно его через `DialogManager.stop`.

### 5.7. Слабый `handle_cast` для `getProperty`

В `DialogSession` есть ветка:

```elixir
handle_cast({:dispatch, %{"command" => "getProperty"} = payload}, state)
```

Она делает `send(self(), {:property_reply, ...})`. Но `PlatformChannel` уже перехватывает `getProperty` и вызывает `reply_property`, поэтому эта ветка, вероятно, мёртвая. Если её случайно задействовать, можно получить двойную отправку/путаницу. Лучше убрать или явно задокументировать.

## 6. Рекомендуемая схема для `EPlatform`, если портировать модалки

### 6.1. Данные сессии

Добавить в `DialogSession` состояние:

```elixir
%{
  application_id: ...,
  form_id: ...,
  parent: nil | %{pid: pid, ref: ref},
  modal: false,
  children: MapSet.new(),
  ...
}
```

Для дочерней формы:

- `parent_pid`;
- `modal_ref`;
- `modal: true`.

### 6.2. Запуск модалки

`DialogManager.open_modal(application_id, parent_form_id, lua_file, opts)`:

1. Генерирует `ref`.
2. Запускает дочернюю сессию через `DynamicSupervisor.start_child` с `parent_pid` и `ref`.
3. Возвращает `{:ok, form_id, ref}` **сразу**, не дожидаясь `init`.
4. Родительский Lua-код делает `receive {:modal_result, ^ref, result}` с таймаутом.

Важно: `start_child` не должен ждать `init` дочерней формы. Сейчас `init` идёт в `handle_continue`, это хорошо.

### 6.3. Закрытие модалки

Дочерняя сессия при `close_modal`:

```elixir
send(parent_pid, {:modal_result, ref, result})
broadcast(closeForm)
send(self(), :stop_form)
```

Родительский `receive` просыпается. Родительский `after`:

- снимает `modal`-флаг;
- отправляет клиенту команду включить родителя;
- очищает `children`.

### 6.4. Ошибки и таймауты

- Если `init` дочерней формы упал, дочерняя сессия должна отправить `{:modal_result, ref, {:error, reason}}` и остановиться.
- Если родительский `receive` вышел по таймауту, он должен:
  - отправить команду закрыть дочернюю форму;
  - остановить дочернюю сессию через `DialogManager.stop`;
  - очистить `children`.

### 6.5. Вложенные модалки

Ничего специального не нужно, если:

- у каждой дочерней формы свой `parent_pid` и `ref`;
- результат всегда идёт сырым `send`;
- нет `GenServer.call` между родителем и ребёнком;
- родитель не ждёт `init` ребёнка синхронно.

Цепочка A → B → C будет работать как стек процессов, как в старом коде.

### 6.6. Клиент

Нужно добавить в `createForm` команду поля:

- `modal: true/false`;
- `mdi: true/false`;
- возможно, `parent_form_id`.

Клиент `rqt`/`app` должен:

- рисовать модальную форму поверх/в MDI;
- блокировать ввод в родителя;
- по закрытию отправлять `formEvent` с `Close` (или `close_modal`).

Если не делать клиентскую модальность, серверная блокировка всё равно не даст обрабатывать события родителя, но пользователь сможет кликать по нему и не понимать, почему ничего не происходит.

### 6.7. Очистка

- `PlatformChannel.terminate` → `DialogManager.stop_application(application_id)` или остановка всех известных `form_id`.
- `DialogSession.terminate` → закрыть/остановить детей.
- `close_modal` → удалить `ref` из `children` и `modal_registry`.
- `get_property` → по таймауту удалять `ref` из реестра, если он там есть.

## 7. Что именно переносить в первую очередь

Если цель — не «всё сразу», а рабочие модалки:

1. `open_opts` + `init` с опциями.
2. `create_form` с `mdi`/`modal`.
3. `open_modal_lua` / `close_modal` с raw `send`.
4. `modal`-флаг и `parent_pid` в `DialogSession`.
5. Таймауты и очистка.
6. Клиентская модальность.

Если цель — паритет с `ref_qt`:

7. `invoke_qt` + `get_prop` через мост.
8. `create_empty_form`, `open_lua`.
9. `list_*` + БД.
10. `mfa` (осторожно, это дыра в песочнице).
11. `sleep`.

## 8. Открытые вопросы

1. Нужна ли в `EPlatform` вообще серверная модальность, или достаточно клиентской блокировки?
2. Должен ли родительский `DialogSession` блокироваться на время модалки, или лучше складывать события в очередь и обрабатывать после закрытия?
3. Что делать с событиями, пришедшими родителю во время модалки: копить, дропать или отклонять?
4. Нужен ли `invoke_qt` в новом протоколе, или заменить его узкими командами (`setEnabled`, `setVisible`, `getProperty`)?
5. Как отслеживать дерево форм: в `DialogSession`, в `DialogManager` или в `Registry` по ключу `{app_id, parent_id}`?
6. Нужно ли сохранять `open_opts` в ETS, как в старом коде, или передавать их в аргументах `start_child`?
7. Что считать результатом модалки по умолчанию: `nil`, `:closed`, `%{}`?
8. Как быть с формой, которая закрыта пользователем крестиком, а не через `close_modal`?

## 9. Мини-словарь

- `busy` — флаг «форма сейчас обрабатывается».
- `open_opts` — опции, которые передаются в `init` дочерней формы (modal, mdi, parent, ref).
- `modal_registry` — соответствие `{app_id, form_id}` → `{parent_pid, ref}`.
- `invoke_qt` — команда клиенту вызвать метод/свойство Qt-объекта.
- `get_prop` — старый способ спросить свойство через `invoke_qt` + `get_prop_response`.
- `getProperty` — новый/текущий способ спросить свойство через отдельный command.
- `mfa` — вызов произвольной Elixir-функции из Lua.
