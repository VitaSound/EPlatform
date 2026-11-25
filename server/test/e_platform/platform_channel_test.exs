defmodule EPlatform.PlatformChannelTest do
  @moduledoc "End-to-end: channel → DialogSession → Lua → broadcasts back to the client."
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

  defp start_dialog(socket, ui_file, lua_file \\ "test_01.lua") do
    ref = push(socket, "start_dialog", %{"lua_file" => lua_file, "ui_file" => ui_file})
    assert_reply ref, :ok, %{"form_id" => form_id}
    form_id
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

  test "start_app boots the main session and applies menu/toolbar/status", %{socket: socket} do
    ref = push(socket, "start_app", %{"lua_file" => "test_main.lua"})
    assert_reply ref, :ok, %{"form_id" => main_id, "role" => "main"}

    assert_push "command", %{
      "command" => "setAppProperty",
      "property" => "windowTitle",
      "source_form_id" => ^main_id,
      "role" => "main"
    }

    assert_push "command", %{"command" => "setAppProperty", "property" => "statusVisible"}
    assert_push "command", %{"command" => "setAppProperty", "property" => "statusMessage"}

    assert_push "command", %{
      "command" => "setAppProperty",
      "property" => "menu",
      "source_form_id" => ^main_id
    }

    assert_push "command", %{
      "command" => "setAppProperty",
      "property" => "toolBar",
      "source_form_id" => ^main_id
    }
  end

  test "main session menu action opens an MDI form", %{socket: socket} do
    ref = push(socket, "start_app", %{"lua_file" => "test_main.lua"})
    assert_reply ref, :ok, %{"form_id" => main_id}

    # Drain the main window initialization commands.
    for _ <- 1..5 do
      assert_push "command", %{"command" => "setAppProperty"}
    end

    push(socket, "send_event", %{
      "command" => "menuAction",
      "form_id" => main_id,
      "action" => "open_lua",
      "lua_file" => "test_form_move.lua",
      "mdi" => true,
      "modal" => false,
      "modal_scope" => "none"
    })

    assert_push "command", %{
      "command" => "createForm",
      "mode" => "full",
      "placement" => "mdi",
      "role" => "form",
      "mdi" => true
    }
  end

  test "main menu exit closes the application", %{socket: socket} do
    ref = push(socket, "start_app", %{"lua_file" => "test_main.lua"})
    assert_reply ref, :ok, %{"form_id" => main_id}

    for _ <- 1..5 do
      assert_push "command", %{"command" => "setAppProperty"}
    end

    push(socket, "send_event", %{
      "command" => "menuAction",
      "form_id" => main_id,
      "action" => "exit"
    })

    assert_push "command", %{"command" => "closeApplication"}
  end

  test "standalone start_dialog sends top-level placement", %{socket: socket} do
    ref =
      push(socket, "start_dialog", %{
        "lua_file" => "test_form_move.lua",
        "ui_file" => "test_dialog01.ui",
        "mode" => "standalone"
      })

    assert_reply ref, :ok, %{"form_id" => form_id}

    assert_push "command", %{
      "command" => "createForm",
      "form_id" => ^form_id,
      "placement" => "top-level",
      "mode" => "standalone"
    }
  end

  test "start_dialog creates the form and Lua drives widgets", %{socket: socket} do
    form_id = start_dialog(socket, "test_dialog01.ui")

    assert_push "command", %{"command" => "createForm", "form_id" => ^form_id, "ui_xml" => xml}
    assert xml =~ "QDialog"

    # init/3 sets properties
    assert_push "command", %{
      "command" => "setProperty",
      "object_name" => "label_1",
      "property" => "text",
      "value" => "A="
    }

    assert_push "command", %{
      "command" => "setFormProperty",
      "property" => "windowTitle",
      "value" => "Тестовая форма"
    }

    # consume the init-time empty value so the click result matches below
    assert_push "command", %{
      "command" => "setProperty",
      "object_name" => "lineEdit_3",
      "property" => "text",
      "value" => ""
    }

    # click the multiply button → Lua reads two line edits and writes the result
    push(socket, "send_event", %{
      "form_id" => form_id,
      "command" => "formEvent",
      "widget_name" => "pushButton_1",
      "event" => "Clicked"
    })

    assert_push "command", %{
      "command" => "getProperty",
      "object_name" => "lineEdit_1",
      "ref" => ref1
    }

    push(socket, "send_event", %{
      "form_id" => form_id,
      "command" => "getProperty",
      "ref" => ref1,
      "value" => "22"
    })

    assert_push "command", %{
      "command" => "getProperty",
      "object_name" => "lineEdit_2",
      "ref" => ref2
    }

    push(socket, "send_event", %{
      "form_id" => form_id,
      "command" => "getProperty",
      "ref" => ref2,
      "value" => "33"
    })

    assert_push "command", %{
      "command" => "setProperty",
      "object_name" => "lineEdit_3",
      "property" => "text",
      "value" => value
    }

    assert value in [726, 726.0]
  end

  test "modal child blocks parent and returns its result", %{socket: socket} do
    parent_id = start_dialog(socket, "test_dialog01.ui", "test_modal_parent.lua")

    assert_push "command", %{"command" => "createForm", "form_id" => ^parent_id}

    assert_push "command", %{
      "command" => "setFormProperty",
      "form_id" => ^parent_id,
      "property" => "windowTitle",
      "value" => "Modal parent"
    }

    assert_push "command", %{
      "command" => "setProperty",
      "form_id" => ^parent_id,
      "object_name" => "lineEdit_1",
      "property" => "text",
      "value" => "waiting"
    }

    assert_push "command", %{"command" => "createForm", "form_id" => child_id, "modal" => true}
    assert child_id != parent_id

    assert_push "command", %{
      "command" => "setFormProperty",
      "form_id" => ^child_id,
      "property" => "windowTitle",
      "value" => "Modal child"
    }

    push(socket, "send_event", %{
      "form_id" => child_id,
      "command" => "formEvent",
      "widget_name" => "pushButton_1",
      "event" => "Clicked"
    })

    assert_push "command", %{"command" => "closeForm", "form_id" => ^child_id}

    assert_push "command", %{
      "command" => "setProperty",
      "form_id" => ^parent_id,
      "object_name" => "lineEdit_1",
      "property" => "text",
      "value" => "result:ok"
    }
  end

  test "killing one session restarts only that form", %{socket: socket} do
    form_a = start_dialog(socket, "test_dialog01.ui")
    form_b = start_dialog(socket, "test_dialog02.ui")

    assert_push "command", %{"command" => "createForm", "form_id" => ^form_a}
    assert_push "command", %{"command" => "createForm", "form_id" => ^form_b}

    {:ok, pid_a} = DialogManager.whereis(socket.assigns.application_id, form_a)
    {:ok, pid_b} = DialogManager.whereis(socket.assigns.application_id, form_b)

    monitor = Process.monitor(pid_a)
    Process.exit(pid_a, :kill)
    assert_receive {:DOWN, ^monitor, :process, ^pid_a, :killed}, 1_000

    application_id = socket.assigns.application_id

    wait_until(fn ->
      case DialogManager.whereis(application_id, form_a) do
        {:ok, pid} -> pid != pid_a and Process.alive?(pid)
        _ -> false
      end
    end)

    {:ok, restarted} = DialogManager.whereis(application_id, form_a)
    assert restarted != pid_a
    assert Process.alive?(restarted)
    assert_push "command", %{"command" => "createForm", "form_id" => ^form_a}, 2_000

    # the other session must be the same process and still answering events
    assert {:ok, ^pid_b} = DialogManager.whereis(application_id, form_b)
    assert Process.alive?(pid_b)

    push(socket, "send_event", %{
      "form_id" => form_b,
      "command" => "formEvent",
      "widget_name" => "pushButton_1",
      "event" => "Clicked"
    })

    assert_push "command", %{"command" => "getProperty", "form_id" => ^form_b}, 2_000
  end

  test "Close stops only the addressed form session", %{socket: socket} do
    form_a = start_dialog(socket, "test_dialog01.ui")
    form_b = start_dialog(socket, "test_dialog02.ui")

    assert_push "command", %{"command" => "createForm", "form_id" => ^form_a}
    assert_push "command", %{"command" => "createForm", "form_id" => ^form_b}

    application_id = socket.assigns.application_id
    {:ok, pid_b} = DialogManager.whereis(application_id, form_b)

    push(socket, "send_event", %{
      "form_id" => form_a,
      "command" => "formEvent",
      "widget_name" => form_a,
      "event" => "Close"
    })

    wait_until(fn -> DialogManager.whereis(application_id, form_a) == {:error, :not_found} end)
    Process.sleep(100)
    assert DialogManager.whereis(application_id, form_a) == {:error, :not_found}
    assert {:ok, ^pid_b} = DialogManager.whereis(application_id, form_b)
    assert Process.alive?(pid_b)
  end

  test "stop_application stops every session of the application", %{socket: socket} do
    form_a = start_dialog(socket, "test_dialog01.ui")
    form_b = start_dialog(socket, "test_dialog02.ui")

    assert_push "command", %{"command" => "createForm", "form_id" => ^form_a}
    assert_push "command", %{"command" => "createForm", "form_id" => ^form_b}

    application_id = socket.assigns.application_id
    assert {:ok, _} = DialogManager.whereis(application_id, form_a)
    assert {:ok, _} = DialogManager.whereis(application_id, form_b)

    DialogManager.stop_application(application_id)

    wait_until(fn ->
      DialogManager.whereis(application_id, form_a) == {:error, :not_found} and
        DialogManager.whereis(application_id, form_b) == {:error, :not_found}
    end)
  end

  test "channel terminate stops all sessions it started", %{socket: socket, app_id: app_id} do
    Process.flag(:trap_exit, true)

    form_id = start_dialog(socket, "test_dialog01.ui")
    {:ok, pid} = DialogManager.whereis(app_id, form_id)
    assert Process.alive?(pid)

    leave(socket)

    :timer.sleep(200)
    refute Process.alive?(pid)
    assert {:error, :not_found} = DialogManager.whereis(app_id, form_id)
  end
end
