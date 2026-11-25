defmodule EPlatform.LuaModulesTest do
  @moduledoc """
  Shared Lua modules: defaults, icons, table sugar and the Clicked fallback.
  """
  use ExUnit.Case, async: false

  import Phoenix.ChannelTest

  alias EPlatform.DialogManager

  @endpoint EPlatformWeb.Endpoint

  setup do
    app_id = "app-" <> EPlatform.ID.generate()
    {:ok, socket} = connect(EPlatformWeb.UserSocket, %{})
    {:ok, _, socket} = subscribe_and_join(socket, "app:" <> app_id)
    %{socket: socket, app_id: app_id}
  end

  defp wait_until(fun, attempts \\ 100) do
    cond do
      fun.() ->
        :ok

      attempts <= 0 ->
        flunk("condition was not met")

      true ->
        Process.sleep(20)
        wait_until(fun, attempts - 1)
    end
  end

  defp start_dialog(socket, ui_file, lua_file, opts \\ %{}) do
    params =
      %{"lua_file" => lua_file, "ui_file" => ui_file}
      |> Map.merge(opts)

    ref = push(socket, "start_dialog", params)
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

  test "open_opts reach init/4", %{socket: socket} do
    form_id =
      start_dialog(socket, "test_dialog01.ui", "test_open_opts.lua", %{
        "open_opts" => %{"title" => "From opts"}
      })

    assert_push "command", %{"command" => "createForm", "form_id" => ^form_id}

    assert_push "command", %{
      "command" => "setFormProperty",
      "form_id" => ^form_id,
      "property" => "windowTitle",
      "value" => "From opts"
    }
  end

  test "application open_lua starts a non-modal child with open_opts", %{socket: socket} do
    parent_id = start_dialog(socket, "test_dialog01.ui", "test_open_lua_parent.lua")

    assert_push "command", %{"command" => "createForm", "form_id" => ^parent_id}

    assert_push "command", %{
      "command" => "setProperty",
      "form_id" => ^parent_id,
      "object_name" => "label_1",
      "property" => "text",
      "value" => "child:" <> child_id
    }

    assert_push "command", %{
      "command" => "createForm",
      "form_id" => ^child_id,
      "mode" => "full",
      "placement" => "mdi"
    }

    assert_push "command", %{
      "command" => "setFormProperty",
      "form_id" => ^child_id,
      "property" => "windowTitle",
      "value" => "Child via open_lua"
    }
  end

  test "form resize flag and size constraints are sent", %{socket: socket} do
    form_id = start_dialog(socket, "test_dialog01.ui", "test_form_resize.lua")

    assert_push "command", %{"command" => "createForm", "form_id" => ^form_id}

    assert_push "command", %{
      "command" => "setFormProperty",
      "form_id" => ^form_id,
      "property" => "resizable",
      "value" => false
    }

    assert_push "command", %{
      "command" => "setFormProperty",
      "form_id" => ^form_id,
      "property" => "minimumSize",
      "value" => %{"w" => 200, "h" => 150}
    }

    assert_push "command", %{
      "command" => "setFormProperty",
      "form_id" => ^form_id,
      "property" => "maximumSize",
      "value" => %{"w" => 400, "h" => 300}
    }
  end

  test "form move/resize sugar sends position and size", %{socket: socket} do
    form_id = start_dialog(socket, "test_dialog01.ui", "test_form_move.lua")

    assert_push "command", %{"command" => "createForm", "form_id" => ^form_id}

    assert_push "command", %{
      "command" => "setFormProperty",
      "form_id" => ^form_id,
      "property" => "position",
      "value" => %{"x" => 40, "y" => 40}
    }

    assert_push "command", %{
      "command" => "setFormProperty",
      "form_id" => ^form_id,
      "property" => "size",
      "value" => %{"w" => 320, "h" => 200}
    }
  end

  test "app sugar sends browser chrome and reserves desktop-only properties", %{
    socket: socket,
    app_id: app_id
  } do
    _form_id = start_dialog(socket, "test_dialog01.ui", "test_app_sugar.lua")

    assert_push "command", %{
      "command" => "setAppProperty",
      "application_id" => ^app_id,
      "property" => "windowTitle",
      "value" => "Sugar title"
    }

    assert_push "command", %{
      "command" => "setAppProperty",
      "property" => "windowIcon",
      "value" => "data:image/svg+xml,<svg xmlns='http://www.w3.org/2000/svg'/>"
    }

    assert_push "command", %{
      "command" => "setAppProperty",
      "property" => "size",
      "value" => %{"w" => 640, "h" => 480}
    }

    assert_push "command", %{
      "command" => "setAppProperty",
      "property" => "position",
      "value" => %{"x" => 10, "y" => 20}
    }

    assert_push "command", %{
      "command" => "setAppProperty",
      "property" => "geometry",
      "value" => %{"x" => 1, "y" => 2, "w" => 640, "h" => 480}
    }

    assert_push "command", %{
      "command" => "setAppProperty",
      "property" => "centered",
      "value" => true
    }

    assert_push "command", %{
      "command" => "setAppProperty",
      "property" => "windowState",
      "value" => "maximized"
    }

    assert_push "command", %{
      "command" => "setAppProperty",
      "property" => "windowState",
      "value" => "minimized"
    }

    assert_push "command", %{
      "command" => "setAppProperty",
      "property" => "visible",
      "value" => true
    }
  end

  test "channel terminate stops open_lua children", %{socket: socket, app_id: app_id} do
    Process.flag(:trap_exit, true)

    parent_id = start_dialog(socket, "test_dialog01.ui", "test_open_lua_parent.lua")

    assert_push "command", %{"command" => "createForm", "form_id" => ^parent_id}

    assert_push "command", %{
      "command" => "setProperty",
      "form_id" => ^parent_id,
      "object_name" => "label_1",
      "property" => "text",
      "value" => "child:" <> child_id
    }

    {:ok, parent_pid} = DialogManager.whereis(app_id, parent_id)
    {:ok, child_pid} = DialogManager.whereis(app_id, child_id)
    parent_monitor = Process.monitor(parent_pid)
    child_monitor = Process.monitor(child_pid)

    leave(socket)

    assert_receive {:DOWN, ^parent_monitor, :process, ^parent_pid, _reason}, 2_000
    assert_receive {:DOWN, ^child_monitor, :process, ^child_pid, _reason}, 2_000

    wait_until(fn ->
      DialogManager.whereis(app_id, parent_id) == {:error, :not_found} and
        DialogManager.whereis(app_id, child_id) == {:error, :not_found}
    end)
  end
end
