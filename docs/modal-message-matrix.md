# Модальные формы: слои и матрица сообщений

Документ фиксирует, как устроен event loop модальных окон в текущей реализации
(`EPlatform.DialogSession` + `EPlatform.Lua.Api`) и какие сообщения могут
приходить в формы на разных слоях.

## 1. Слои

Каждая открытая форма — отдельный `DialogSession` (GenServer). Модальность не
создаёт отдельного процесса-координатора: родитель просто блокируется в
`receive` внутри `open_modal_lua`, а дочерняя форма живёт в своём процессе.

```
L0  top-level form A        top-level form B (sibling)
      │
      └── L1 modal child ── L2 modal grandchild ── L3 ...
```

- **L0** — обычные формы, открытые через `start_dialog`.
- **L1** — модальный ребёнок, открытый из L0 через `open_modal_lua`.
- **L2** — модальный ребёнок L1 и т.д.
- Глубина не ограничена архитектурой; ограничение — таймаут `open_modal_lua`
  (300 с) и здравый смысл.

Ключевое свойство: **блокируется только процесс того слоя, который вызвал
`open_modal_lua`**. Соседние формы L0 и активный модальный слой продолжают
обрабатывать события.

## 2. Что происходит с событиями во время модалки

| Источник | Получатель | Поведение |
|---|---|---|
| Клиент | активный модальный слой (L1/L2/…) | обрабатывается сразу |
| Клиент | sibling-форма на L0 | обрабатывается сразу |
| Клиент | заблокированный родитель (L0/L1/…) | попадает в mailbox GenServer и ждёт возврата из `open_modal_lua` |
| Клиент | закрытая/закрывающаяся форма | отбрасывается по флагу `:e_platform_closing` |
| Lua | `getProperty` / `getFormProperty` из активного слоя | блокирующий `receive` с `ref`, клиент отвечает |
| Lua | `getProperty` / `getFormProperty` из sibling-формы | блокирующий `receive` с `ref`, клиент отвечает |
| Lua | `getProperty` из заблокированного родителя | невозможно инициировать до возврата модалки; уже начатое чтение завершится раньше |
| Lua | `setProperty` / `setFormProperty` из любого слоя | broadcast `command` клиенту |
| Lua | `close_form` / `finish_modal` | `closeForm` + результат родителю (для модалки) |
| Lua | `close_application` | `closeApplication` + остановка всех сессий приложения |
| Сервер | `createForm` модального ребёнка | `modal: true`, `mdi` (если задан) |
| Сервер | `dialogError` при падении `init` | broadcast в канал |

### Важное следствие

Родитель **не обрабатывает** события параллельно с модалкой: его mailbox
копится. После возврата из `open_modal_lua` GenServer разбирает очередь в
порядке прихода. Это сохраняет последовательность Lua-состояния и не ломает
блокирующие `getProperty`.

## 3. Результат модалки и закрытие

- `finish_modal(result)` → `close_modal` → `closeForm` + сырое сообщение
  `{:modal_result, ref, result}` родителю.
- `close_form()` у модального ребёнка → `closeForm` + `{:modal_result, ref, nil}`.
- Клиентский `Close` у модального ребёнка → `notify_modal(nil)` → родитель
  получает `nil`, ребёнок останавливается.
- Падение/убийство ребёнка → родитель получает `DOWN`, `open_modal_lua`
  возвращает `"error"`, клиенту уходит `closeForm`.
- Падение/остановка родителя → ребёнок получает `DOWN` по монитору, шлёт
  `closeForm` и останавливается.
- `closeApplication` → `closeApplication` всем, затем `stop_application`
  завершает все сессии, включая вложенные.

## 4. Покрытие тестами

`server/test/e_platform/modal_flow_test.exs`:

1. `parent events queue while a modal child is open and drain` — очередь
   событий заблокированного родителя и порядок разбора после модалки.
2. `sibling still handles events and getProperty while a modal is open` —
   sibling-форма принимает `Clicked`, читает widget- и form-свойства.
3. `nested modals keep each layer isolated and drain their own queues` —
   L0 → L1 → L2, очереди каждого слоя, размотка результатов.
4. `modal child can read widget properties while the parent is blocked` —
   `getProperty` из активного модального слоя.
5. `client Close on a modal child releases the parent and stops the child` —
   закрытие крестиком/событием `Close`.
6. `parent death closes the modal child and releases its monitor` —
   каскадная остановка ребёнка.
7. `crashed modal child releases the parent and closes on the client` —
   `DOWN` ребёнка, `closeForm`, возврат `"error"`.
8. `closeApplication during a modal stops every session` — закрытие всего
   приложения при открытой модалке.

## 5. Ограничения текущей реализации

- События заблокированного родителя не обрабатываются параллельно, а
  накапливаются в mailbox. Это осознанный выбор в пользу простоты и
  последовательного Lua-состояния.
- Модальный ребёнок создаётся с `restart: :temporary`; при падении он не
  перезапускается, а родитель получает `"error"`.
- `open_modal_lua` имеет таймаут 300 с; по таймауту ребёнок останавливается,
  клиенту уходит `closeForm`.
- `dialogError` для модального ребёнка не превращается в `modal_result`;
  родитель получает `DOWN`/`"error"` через монитор.
