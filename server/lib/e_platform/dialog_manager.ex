defmodule EPlatform.DialogManager do
  @moduledoc """
  Public API over the DynamicSupervisor of `EPlatform.DialogSession`.
  One session = one open form on the client (isolation by design: a crash
  of one session never affects the others).
  """

  alias EPlatform.DialogSession

  @doc """
  Starts a dialog session (Lua script + form) for the given application.

  `opts` may contain `:modal`, `:mdi`, `:modal_parent`, `:modal_ref`,
  `:modal_parent_form_id` and `:open_opts`. The default starts a normal,
  non-modal form.
  """
  def start_dialog(application_id, lua_file, ui_file, opts \\ %{}) do
    form_id = EPlatform.ID.generate()

    session_opts =
      Map.merge(
        %{
          application_id: application_id,
          form_id: form_id,
          lua_file: lua_file,
          ui_file: ui_file,
          modal: false,
          mdi: true
        },
        opts
      )

    spec = {DialogSession, session_opts}

    case DynamicSupervisor.start_child(EPlatform.DialogSupervisor, spec) do
      {:ok, _pid} -> {:ok, form_id}
      {:error, reason} -> {:error, reason}
    end
  end

  @doc """
  Starts a modal dialog session and returns its `form_id` and pid.

  The caller (usually the parent `DialogSession` process) is responsible for
  waiting for `{:modal_result, ref, result}`. The child sends that raw message
  on close, so the parent's `GenServer` is not blocked by `GenServer.call/3`.
  """
  def start_modal(application_id, lua_file, ui_file, opts) do
    form_id = EPlatform.ID.generate()

    session_opts =
      Map.merge(
        %{
          application_id: application_id,
          form_id: form_id,
          lua_file: lua_file,
          ui_file: ui_file,
          modal: true,
          mdi: true
        },
        opts
      )
      |> Map.put(:modal, true)

    spec = %{
      id: {DialogSession, form_id},
      start: {DialogSession, :start_link, [session_opts]},
      restart: :temporary,
      type: :worker
    }

    case DynamicSupervisor.start_child(EPlatform.DialogSupervisor, spec) do
      {:ok, pid} -> {:ok, form_id, pid}
      {:error, reason} -> {:error, reason}
    end
  end

  @doc "Looks up a session pid by ids."
  def whereis(application_id, form_id) do
    case Registry.lookup(EPlatform.DialogRegistry, {application_id, form_id}) do
      [{pid, _}] -> {:ok, pid}
      [] -> {:error, :not_found}
    end
  end

  @doc "Fire-and-forget dispatch of a client event into the session."
  def dispatch(application_id, form_id, payload) do
    with {:ok, pid} <- whereis(application_id, form_id) do
      GenServer.cast(pid, {:dispatch, payload})
    end
  end

  @doc "Delivers a getProperty reply as a raw message so it can be picked up by an in-flight Lua `receive`."
  def reply_property(application_id, form_id, ref, value) do
    with {:ok, pid} <- whereis(application_id, form_id) do
      send(pid, {:property_reply, ref, value})
      :ok
    end
  end

  @doc "Stops a single session (used when the owning client channel goes away)."
  def stop(application_id, form_id) do
    with {:ok, pid} <- whereis(application_id, form_id) do
      DynamicSupervisor.terminate_child(EPlatform.DialogSupervisor, pid)
    end
  end

  @doc "Stops every session of an application (`close_application`)."
  def stop_application(application_id) do
    pids =
      Registry.select(EPlatform.DialogRegistry, [
        {{{application_id, :_}, :"$1", :_}, [], [:"$1"]}
      ])

    Enum.each(pids, fn pid ->
      DynamicSupervisor.terminate_child(EPlatform.DialogSupervisor, pid)
    end)
  end
end
