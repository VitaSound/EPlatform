defmodule EPlatform.DialogSession do
  @moduledoc """
  One GenServer per open form. Owns the Lua state of the form, mirrors the
  widget/form properties set from Lua, and forwards client events to
  `onEvent`. Runs the Lua `init` in `handle_continue` so the channel is never
  blocked while the form boots.
  """

  use GenServer, restart: :transient
  require Logger

  alias EPlatform.Lua.Logic, as: LuaLogic

  def start_link(opts) do
    GenServer.start_link(__MODULE__, opts, name: via(opts.application_id, opts.form_id))
  end

  defp via(application_id, form_id) do
    {:via, Registry, {EPlatform.DialogRegistry, {application_id, form_id}}}
  end

  # `Lua.Api` runs in this process, so the process dictionary is the simplest
  # place to expose the modal parent/ref without threading a context argument
  # through every Lua function.
  defp expose_modal_context(state) do
    Process.put(:e_platform_session, %{
      form_id: state.form_id,
      role: state.role,
      mode: state.mode,
      mdi: state.mdi,
      modal: state.modal,
      modal_scope: state.modal_scope
    })

    if state.modal do
      Process.put(:e_platform_modal, %{
        parent: state.modal_parent,
        ref: state.modal_ref,
        parent_form_id: state.modal_parent_form_id,
        mdi: state.mdi
      })

      Process.put(:e_platform_modal_sent, false)
    end

    monitor =
      if state.modal and is_pid(state.modal_parent) do
        Process.monitor(state.modal_parent)
      end

    %{state | modal_monitor: monitor}
  end

  # An open_lua child is owned by the DialogSession that opened it: when that
  # session dies (channel gone), the child must die too. Monitors, not a stop
  # loop in `terminate/2`, because the DynamicSupervisor cannot process a
  # terminate_child call while it is waiting for us to finish terminating.
  defp expose_owner_monitor(state) do
    monitor =
      if is_pid(state.owner) do
        Process.monitor(state.owner)
      end

    %{state | owner_monitor: monitor}
  end

  @impl true
  def init(opts) do
    state = %{
      application_id: opts.application_id,
      form_id: opts.form_id,
      lua_file: opts.lua_file,
      ui_file: opts.ui_file,
      lua_state: nil,
      widgets: %{},
      form_props: %{},
      modal: Map.get(opts, :modal, false),
      mdi: Map.get(opts, :mdi, true),
      mode: Map.get(opts, :mode, :standalone),
      role: Map.get(opts, :role, :form),
      modal_scope: Map.get(opts, :modal_scope, "none"),
      modal_parent: Map.get(opts, :modal_parent),
      modal_ref: Map.get(opts, :modal_ref),
      modal_parent_form_id: Map.get(opts, :modal_parent_form_id),
      open_opts: Map.get(opts, :open_opts, %{}),
      owner: Map.get(opts, :owner),
      modal_monitor: nil,
      owner_monitor: nil
    }

    state = expose_modal_context(state)
    state = expose_owner_monitor(state)
    {:ok, state, {:continue, :boot}}
  end

  @impl true
  def handle_continue(:boot, state) do
    try do
      {:ok, lua_state} = LuaLogic.compile_file(state.lua_file)
      {encoded_open_opts, lua_state} = Lua.encode!(lua_state, state.open_opts)

      lua_state =
        if state.ui_file in [nil, ""] do
          {:ok, _res, lua_state} =
            Lua.call_function(lua_state, :init, [
              state.application_id,
              state.form_id,
              nil,
              encoded_open_opts
            ])

          lua_state
        else
          {:ok, _res, lua_state} =
            Lua.call_function(lua_state, :init, [
              state.application_id,
              state.form_id,
              state.ui_file,
              encoded_open_opts
            ])

          lua_state
        end

      {:noreply, %{state | lua_state: lua_state}}
    rescue
      e ->
        Logger.error("DialogSession boot failed form=#{state.form_id}: #{Exception.message(e)}")

        broadcast(state.application_id, %{
          "command" => "dialogError",
          "application_id" => state.application_id,
          "form_id" => state.form_id,
          "error" => Exception.message(e)
        })

        {:stop, :normal, state}
    end
  end

  @impl true
  def handle_cast({:dispatch, %{"command" => "formEvent"} = payload}, state) do
    # A form that is already closing must not run queued events that arrived
    # while it was blocked in a modal call. They are simply discarded.
    if Process.get(:e_platform_closing) do
      {:stop, :normal, state}
    else
      widget_name = payload["widget_name"] || payload["object_name"]
      event = payload["event"] || payload["property"]

      state =
        try do
          {_status, _res, lua_state} =
            Lua.call_function(state.lua_state, :onEvent, [widget_name, event])

          %{state | lua_state: lua_state}
        rescue
          e ->
            Logger.error("onEvent failed form=#{state.form_id}: #{Exception.message(e)}")
            state
        end

      if event == "Close" do
        _ = EPlatform.Lua.Api.notify_modal(nil)
        {:stop, :normal, state}
      else
        {:noreply, state}
      end
    end
  end

  def handle_cast({:dispatch, %{"command" => "menuAction"} = payload}, state) do
    if Process.get(:e_platform_closing) do
      {:stop, :normal, state}
    else
      args = [
        payload["action"] || "",
        payload["lua_file"] || "",
        payload["ui_file"] || "",
        payload["mdi"] == true,
        payload["modal"] == true,
        payload["modal_scope"] || "application"
      ]

      state =
        try do
          {_status, _res, lua_state} =
            Lua.call_function(state.lua_state, :onMenuAction, args)

          %{state | lua_state: lua_state}
        rescue
          e ->
            Logger.error("onMenuAction failed form=#{state.form_id}: #{Exception.message(e)}")
            state
        end

      {:noreply, state}
    end
  end

  def handle_cast({:dispatch, %{"command" => "getProperty"} = payload}, state) do
    # Deliver the client's value to the Lua code blocked in its inner `receive`.
    send(self(), {:property_reply, payload["ref"], payload["value"]})
    {:noreply, state}
  end

  def handle_cast({:dispatch, _payload}, state) do
    if Process.get(:e_platform_closing) do
      {:stop, :normal, state}
    else
      {:noreply, state}
    end
  end

  @impl true
  def handle_info({:widget_property, form_id, object_name, property, value}, state)
      when form_id == state.form_id do
    {:noreply, put_in(state, [:widgets, {object_name, property}], value)}
  end

  def handle_info({:form_property, form_id, property, value}, state)
      when form_id == state.form_id do
    {:noreply, put_in(state, [:form_props, property], value)}
  end

  def handle_info({:DOWN, ref, :process, _pid, _reason}, state)
      when ref == state.modal_monitor do
    # The parent disappeared while this modal was open; close the child on the
    # client as well. The parent is gone, so there is nobody to notify.
    broadcast(state.application_id, %{
      "command" => "closeForm",
      "application_id" => state.application_id,
      "form_id" => state.form_id
    })

    {:stop, :normal, state}
  end

  def handle_info({:DOWN, ref, :process, _pid, _reason}, state)
      when ref == state.owner_monitor do
    # The session that opened us via open_lua is gone (its channel closed):
    # shut this child down as well, cascading to any grandchildren.
    {:stop, :normal, state}
  end

  def handle_info(:stop_form, state) do
    _ = EPlatform.Lua.Api.notify_modal(nil)
    {:stop, :normal, state}
  end

  # Late/timeout getProperty replies and anything else — ignore.
  def handle_info(_msg, state), do: {:noreply, state}

  # -- helper used by Lua.Api -------------------------------------------------

  def broadcast(application_id, payload) do
    EPlatformWeb.Endpoint.broadcast("app_events:" <> application_id, "command", payload)
  end
end
