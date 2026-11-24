defmodule EPlatform.LuaModulesTest do
  @moduledoc """
  Shared Lua modules: defaults, icons, table sugar and the Clicked fallback.
  """
  use ExUnit.Case, async: false

  import Phoenix.ChannelTest

  @endpoint EPlatformWeb.Endpoint

  setup do
    app_id = "app-" <> EPlatform.ID.generate()
    {:ok, socket} = connect(EPlatformWeb.UserSocket, %{})
    {:ok, _, socket} = subscribe_and_join(socket, "app:" <> app_id)
    %{socket: socket}
  end

  defp start_dialog(socket, ui_file, lua_file) do
    ref = push(socket, "start_dialog", %{"lua_file" => lua_file, "ui_file" => ui_file})
    assert_reply ref, :ok, %{"form_id" => form_id}
    form_id
  end

  test "form_common_lib applies defaults and table sugar", %{socket: socket} do
    form_id = start_dialog(socket, "abstract_table_form.ui", "test_common.lua")

    assert_push "command", %{"command" => "createForm", "form_id" => ^form_id}

    assert_push "command", %{
      "command" => "setFormProperty",
      "form_id" => ^form_id,
      "property" => "windowTitle",
      "value" => "Common form"
    }

    assert_push "command", %{
      "command" => "setFormProperty",
      "form_id" => ^form_id,
      "property" => "geometry",
      "value" => %{"x" => 120, "y" => 80, "w" => 800, "h" => 600}
    }

    assert_push "command", %{
      "command" => "setProperty",
      "form_id" => ^form_id,
      "object_name" => "btn_ok",
      "property" => "text",
      "value" => "ОК"
    }

    assert_push "command", %{
      "command" => "setProperty",
      "form_id" => ^form_id,
      "object_name" => "btn_ok",
      "property" => "icon",
      "value" => "img24:22"
    }

    assert_push "command", %{
      "command" => "setProperty",
      "form_id" => ^form_id,
      "object_name" => "tbtn_add",
      "property" => "toolTip",
      "value" => "Добавить"
    }

    assert_push "command", %{
      "command" => "setProperty",
      "form_id" => ^form_id,
      "object_name" => "tbl",
      "property" => "tableHeaders",
      "value" => %{
        "clear" => true,
        "columnCount" => 2,
        "horizontalHeaderLabels" => ["Name", "Value"],
        "columnWidths" => [240, 120]
      }
    }

    assert_push "command", %{
      "command" => "setProperty",
      "form_id" => ^form_id,
      "object_name" => "tbl",
      "property" => "tableData",
      "value" => %{
        "rowCount" => 2,
        "data" => [["one", "1"], ["two", "2"]]
      }
    }
  end

  test "handleEvent falls back from Clicked to MouseButtonRelease", %{socket: socket} do
    form_id = start_dialog(socket, "abstract_table_form.ui", "test_common.lua")

    assert_push "command", %{"command" => "createForm", "form_id" => ^form_id}

    push(socket, "send_event", %{
      "form_id" => form_id,
      "command" => "formEvent",
      "widget_name" => "btn_ok",
      "event" => "Clicked"
    })

    assert_push "command", %{"command" => "closeForm", "form_id" => ^form_id}
  end
end
