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

  @impl true
  def init(opts) do
    state = %{
      application_id: opts.application_id,
      form_id: opts.form_id,
      lua_file: opts.lua_file,
      ui_file: opts.ui_file,
      lua_state: nil,
      widgets: %{},
      form_props: %{}
    }

    {:ok, state, {:continue, :boot}}
  end

  @impl true
  def handle_continue(:boot, state) do
    try do
      {:ok, lua_state} = LuaLogic.compile_file(state.lua_file)

      lua_state =
        if state.ui_file in [nil, ""] do
          {:ok, _res, lua_state} =
            Lua.call_function(lua_state, :init, [state.application_id, state.form_id])

          lua_state
        else
          {:ok, _res, lua_state} =
            Lua.call_function(lua_state, :init, [
              state.application_id,
              state.form_id,
              state.ui_file
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
      {:stop, :normal, state}
    else
      {:noreply, state}
    end
  end

  def handle_cast({:dispatch, %{"command" => "getProperty"} = payload}, state) do
    # Deliver the client's value to the Lua code blocked in its inner `receive`.
    send(self(), {:property_reply, payload["ref"], payload["value"]})
    {:noreply, state}
  end

  def handle_cast({:dispatch, _payload}, state) do
    {:noreply, state}
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

  def handle_info(:stop_form, state), do: {:stop, :normal, state}

  # Late/timeout getProperty replies and anything else — ignore.
  def handle_info(_msg, state), do: {:noreply, state}

  # -- helper used by Lua.Api -------------------------------------------------

  def broadcast(application_id, payload) do
    EPlatformWeb.Endpoint.broadcast("app_events:" <> application_id, "command", payload)
  end
end
