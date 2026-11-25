defmodule EPlatformWeb.PlatformChannel do
  @moduledoc """
  One channel per application ("app:<id>"). Relays `command` broadcasts from
  dialog sessions to the client and routes client events back into the right
  `EPlatform.DialogSession`. All sessions started through this channel are
  stopped when the channel goes away (browser tab closed).
  """
  use Phoenix.Channel
  require Logger

  alias EPlatform.DialogManager

  @impl true
  def join("app:" <> application_id, _params, socket) do
    Phoenix.PubSub.subscribe(EPlatform.PubSub, "app_events:" <> application_id)

    socket =
      socket
      |> assign(:application_id, application_id)
      |> assign(:main_form_id, nil)
      |> assign(:form_ids, MapSet.new())

    {:ok, %{"application_id" => application_id}, socket}
  end

  @impl true
  def handle_in("start_app", params, socket) do
    application_id = socket.assigns.application_id
    lua_file = params["lua_file"] || ""

    opts = %{
      mode: :full,
      role: :main,
      mdi: false,
      ui_file: params["ui_file"] || "",
      open_opts: params["open_opts"] || %{}
    }

    case DialogManager.start_app(application_id, lua_file, opts) do
      {:ok, form_id} ->
        socket =
          socket
          |> assign(:main_form_id, form_id)
          |> assign(:form_ids, MapSet.put(socket.assigns.form_ids, form_id))

        {:reply, {:ok, %{"form_id" => form_id, "role" => "main"}}, socket}

      {:error, reason} ->
        {:reply, {:error, %{"reason" => inspect(reason)}}, socket}
    end
  end

  @impl true
  def handle_in("start_dialog", params, socket) do
    application_id = socket.assigns.application_id
    lua_file = params["lua_file"] || ""
    ui_file = params["ui_file"] || ""
    standalone = params["mode"] == "standalone"

    opts = %{
      modal: params["modal"] == true,
      mdi: if(standalone, do: false, else: params["mdi"] != false),
      mode: if(standalone, do: :standalone, else: :full),
      role: :form,
      modal_scope: params["modal_scope"] || "none",
      open_opts: params["open_opts"] || %{}
    }

    case DialogManager.start_dialog(application_id, lua_file, ui_file, opts) do
      {:ok, form_id} ->
        socket = assign(socket, :form_ids, MapSet.put(socket.assigns.form_ids, form_id))
        {:reply, {:ok, %{"form_id" => form_id}}, socket}

      {:error, reason} ->
        {:reply, {:error, %{"reason" => inspect(reason)}}, socket}
    end
  end

  def handle_in("send_event", payload, socket) do
    application_id = socket.assigns.application_id
    form_id = payload["form_id"]

    case payload["command"] do
      command when command in ["getProperty", "getFormProperty"] ->
        DialogManager.reply_property(application_id, form_id, payload["ref"], payload["value"])

      _other ->
        DialogManager.dispatch(application_id, form_id, payload)
    end

    {:reply, {:ok, %{}}, socket}
  end

  def handle_in(_event, _payload, socket) do
    {:reply, {:error, %{"reason" => "unknown_event"}}, socket}
  end

  # Broadcast from a DialogSession → push to this client.
  @impl true
  def handle_info(%Phoenix.Socket.Broadcast{event: "command", payload: payload}, socket) do
    push(socket, "command", payload)
    {:noreply, socket}
  end

  def handle_info(_msg, socket), do: {:noreply, socket}

  @impl true
  def terminate(_reason, socket) do
    application_id = socket.assigns[:application_id]
    form_ids = socket.assigns[:form_ids] || MapSet.new()

    if application_id do
      Enum.each(form_ids, fn form_id ->
        DialogManager.stop(application_id, form_id)
      end)
    end

    :ok
  end
end
