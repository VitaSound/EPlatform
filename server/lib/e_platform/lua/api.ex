defmodule EPlatform.Lua.Api do
  @moduledoc """
  Functions exposed to Lua (`createForm`, `set_property`, ...).
  Executed inside the owning `EPlatform.DialogSession` process; every call
  broadcasts a `command` payload to the client channel.
  """
  use Lua.API
  require Logger

  alias EPlatform.DialogSession

  deflua(double(v), do: 2 * v)

  deflua create_form(application_id, form_id, form_file) do
    priv_dir = :code.priv_dir(:e_platform)
    path = "#{priv_dir}/qt_forms/#{form_file}"

    case File.read(path) do
      {:ok, content} ->
        DialogSession.broadcast(application_id, %{
          "command" => "createForm",
          "application_id" => application_id,
          "form_id" => form_id,
          "ui_xml" => content
        })

        form_id

      {:error, _} ->
        raise "Ошибка чтения файла ui: #{form_file}"
    end
  end

  deflua close_form(application_id, form_id) do
    DialogSession.broadcast(application_id, %{
      "command" => "closeForm",
      "application_id" => application_id,
      "form_id" => form_id
    })

    send(self(), :stop_form)
    0
  end

  deflua close_application(application_id) do
    DialogSession.broadcast(application_id, %{
      "command" => "closeApplication",
      "application_id" => application_id
    })

    _ = Task.start(fn -> EPlatform.DialogManager.stop_application(application_id) end)
    0
  end

  deflua set_property(application_id, form_id, object_name, property, value), state do
    value = decode_value(state, value)

    DialogSession.broadcast(application_id, %{
      "command" => "setProperty",
      "application_id" => application_id,
      "form_id" => form_id,
      "object_name" => object_name,
      "property" => property,
      "value" => to_map(value)
    })

    # Mirror inside the owning session process (we run inside it).
    send(self(), {:widget_property, form_id, object_name, property, to_map(value)})

    0
  end

  deflua set_form_property(application_id, form_id, property, value), state do
    value = decode_value(state, value)

    DialogSession.broadcast(application_id, %{
      "command" => "setFormProperty",
      "application_id" => application_id,
      "form_id" => form_id,
      "property" => property,
      "value" => to_map(value)
    })

    send(self(), {:form_property, form_id, property, to_map(value)})

    0
  end

  deflua get_property(application_id, form_id, object_name, property) do
    ref = EPlatform.ID.generate()

    DialogSession.broadcast(application_id, %{
      "command" => "getProperty",
      "application_id" => application_id,
      "form_id" => form_id,
      "object_name" => object_name,
      "property" => property,
      "ref" => ref
    })

    # The reply arrives as {:property_reply, ref, value} in this process'
    # mailbox (sent by the channel via DialogManager.reply_property/4) and is
    # matched here even while we are inside a Lua call.
    receive do
      {:property_reply, ^ref, value} ->
        normalize_property_value(value)
    after
      5_000 ->
        "timeout"
    end
  end

  deflua get_form_property(application_id, form_id, property) do
    ref = EPlatform.ID.generate()

    DialogSession.broadcast(application_id, %{
      "command" => "getFormProperty",
      "application_id" => application_id,
      "form_id" => form_id,
      "property" => property,
      "ref" => ref
    })

    receive do
      {:property_reply, ^ref, value} ->
        normalize_property_value(value)
    after
      5_000 ->
        "timeout"
    end
  end

  # -- helpers ----------------------------------------------------------------

  defp decode_value(state, {:tref, _} = value) do
    state
    |> Lua.decode!(value)
    |> to_map()
  end

  defp decode_value(_state, value), do: value

  defp normalize_property_value(value) when is_binary(value) do
    case Float.parse(value) do
      {f, ""} -> if f == trunc(f), do: trunc(f), else: f
      _ -> value
    end
  end

  defp normalize_property_value(value), do: value

  def to_map(value) when is_list(value) do
    cond do
      # Lua array `{ "a", "b" }` decodes to `[{1, "a"}, {2, "b"}]` —
      # render it as a JSON array, not a map with integer keys.
      pair_list?(value) and sequence?(value) ->
        value
        |> Enum.sort_by(fn {k, _v} -> k end)
        |> Enum.map(fn {_k, v} -> to_map(v) end)

      # Lua map `{ x = 1 }` decodes to `[{ "x", 1 }]` — string keys
      # (integer/atom keys are stringified for JSON).
      pair_list?(value) ->
        Map.new(value, fn {k, v} -> {to_key(k), to_map(v)} end)

      true ->
        Enum.map(value, &to_map/1)
    end
  end

  def to_map(value) when is_map(value) do
    Map.new(value, fn {k, v} -> {to_key(k), to_map(v)} end)
  end

  def to_map(value), do: value

  defp pair_list?(list) do
    list != [] and
      Enum.all?(list, fn
        {_k, _v} -> true
        _ -> false
      end)
  end

  # Keys are the consecutive integers 1..n (a proper Lua sequence).
  defp sequence?(list) do
    list
    |> Enum.map(fn {k, _v} -> k end)
    |> Enum.sort()
    |> consecutive_from_one?()
  end

  defp consecutive_from_one?([]), do: true

  defp consecutive_from_one?([k | rest]) when is_integer(k) and k >= 1 do
    consecutive_from_one?(rest, k + 1)
  end

  defp consecutive_from_one?(_keys), do: false

  defp consecutive_from_one?([], _next), do: true

  defp consecutive_from_one?([k | rest], next) when k == next do
    consecutive_from_one?(rest, next + 1)
  end

  defp consecutive_from_one?(_keys, _next), do: false

  defp to_key(key) when is_binary(key), do: key
  defp to_key(key) when is_atom(key), do: Atom.to_string(key)
  defp to_key(key) when is_integer(key), do: Integer.to_string(key)
  defp to_key(key) when is_float(key), do: :erlang.float_to_binary(key, [:short])
end
