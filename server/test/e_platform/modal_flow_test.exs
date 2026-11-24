defmodule EPlatform.ModalFlowTest do
  @moduledoc """
  Modal/event-loop matrix: who may receive events while a modal is open,
  which events queue behind a blocked parent, and how results travel back
  through nested modal layers.
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

  defp start_dialog(socket, ui_file, lua_file) do
    ref = push(socket, "start_dialog", %{"lua_file" => lua_file, "ui_file" => ui_file})
    assert_reply ref, :ok, %{"form_id" => form_id}
    form_id
  end

  defp send_event(socket, form_id, widget, event) do
    push(socket, "send_event", %{
      "form_id" => form_id,
      "command" => "formEvent",
      "widget_name" => widget,
      "event" => event
    })
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

  defp expect_parent_boot(_socket, parent_id) do
    assert_push "command", %{"command" => "createForm", "form_id" => ^parent_id}

    assert_push "command", %{
      "command" => "setFormProperty",
      "form_id" => ^parent_id,
      "property" => "windowTitle"
    }

    assert_push "command", %{
      "command" => "setProperty",
      "form_id" => ^parent_id,
      "object_name" => "label_1",
      "property" => "text",
      "value" => "ready"
    }

    assert_push "command", %{
      "command" => "setProperty",
      "form_id" => ^parent_id,
      "object_name" => "label_2",
      "property" => "text",
      "value" => "idle"
    }
  end

  defp open_flow_child(socket, parent_id) do
    send_event(socket, parent_id, "pushButton_1", "Clicked")

    assert_push "command", %{
      "command" => "setProperty",
      "form_id" => ^parent_id,
      "object_name" => "label_1",
      "property" => "text",
      "value" => "opening"
    }

    assert_push "command", %{"command" => "createForm", "form_id" => child_id, "modal" => true}
    assert child_id != parent_id

    assert_push "command", %{
      "command" => "setFormProperty",
      "form_id" => ^child_id,
      "property" => "windowTitle"
    }

    assert_push "command", %{
      "command" => "setProperty",
      "form_id" => ^child_id,
      "object_name" => "label_1",
      "property" => "text",
      "value" => "child-ready"
    }

    child_id
  end

  test "parent events queue while a modal child is open and drain after close", %{
    socket: socket
  } do
    parent_id = start_dialog(socket, "test_dialog01.ui", "test_modal_flow.lua")
    expect_parent_boot(socket, parent_id)
    child_id = open_flow_child(socket, parent_id)

    send_event(socket, parent_id, "pushButton_2", "Clicked")

    refute_receive %Phoenix.Socket.Message{
                     event: "command",
                     payload: %{
                       "command" => "setProperty",
                       "form_id" => ^parent_id,
                       "object_name" => "label_2",
                       "property" => "text",
                       "value" => "queued"
                     }
                   },
                   200

    send_event(socket, child_id, "pushButton_1", "Clicked")

    assert_push "command", %{"command" => "closeForm", "form_id" => ^child_id}

    assert_push "command", %{
      "command" => "setProperty",
      "form_id" => ^parent_id,
      "object_name" => "label_1",
      "property" => "text",
      "value" => "closed:ok"
    }

    assert_push "command", %{
      "command" => "setProperty",
      "form_id" => ^parent_id,
      "object_name" => "label_2",
      "property" => "text",
      "value" => "queued"
    }
  end

  test "sibling form still handles events and getProperty while a modal is open", %{
    socket: socket
  } do
    sibling_id = start_dialog(socket, "test_dialog03.ui", "test_modal_sibling.lua")

    assert_push "command", %{"command" => "createForm", "form_id" => ^sibling_id}

    assert_push "command", %{
      "command" => "setFormProperty",
      "form_id" => ^sibling_id,
      "property" => "windowTitle"
    }

    assert_push "command", %{
      "command" => "setProperty",
      "form_id" => ^sibling_id,
      "object_name" => "cellInfo",
      "property" => "text",
      "value" => "sibling-ready"
    }

    parent_id = start_dialog(socket, "test_dialog01.ui", "test_modal_flow.lua")
    expect_parent_boot(socket, parent_id)
    child_id = open_flow_child(socket, parent_id)

    send_event(socket, sibling_id, "toggleScroll", "Clicked")

    assert_push "command", %{
      "command" => "getProperty",
      "form_id" => ^sibling_id,
      "object_name" => "cellInfo",
      "property" => "text",
      "ref" => widget_ref
    }

    push(socket, "send_event", %{
      "form_id" => sibling_id,
      "command" => "getProperty",
      "ref" => widget_ref,
      "value" => "sibling-value"
    })

    assert_push "command", %{
      "command" => "getFormProperty",
      "form_id" => ^sibling_id,
      "property" => "windowTitle",
      "ref" => form_ref
    }

    push(socket, "send_event", %{
      "form_id" => sibling_id,
      "command" => "getFormProperty",
      "ref" => form_ref,
      "value" => "Modal sibling"
    })

    assert_push "command", %{
      "command" => "setProperty",
      "form_id" => ^sibling_id,
      "object_name" => "cellInfo",
      "property" => "text",
      "value" => "sibling:sibling-value/Modal sibling"
    }

    send_event(socket, child_id, "pushButton_1", "Clicked")
    assert_push "command", %{"command" => "closeForm", "form_id" => ^child_id}

    assert_push "command", %{
      "command" => "setProperty",
      "form_id" => ^parent_id,
      "object_name" => "label_1",
      "property" => "text",
      "value" => "closed:ok"
    }
  end

  test "nested modals keep each layer isolated and drain their own queues", %{socket: socket} do
    parent_id = start_dialog(socket, "test_dialog01.ui", "test_modal_nested_parent.lua")
    expect_parent_boot(socket, parent_id)

    # Parent opens the first modal layer.
    send_event(socket, parent_id, "pushButton_1", "Clicked")

    assert_push "command", %{
      "command" => "setProperty",
      "form_id" => ^parent_id,
      "object_name" => "label_1",
      "property" => "text",
      "value" => "opening"
    }

    assert_push "command", %{"command" => "createForm", "form_id" => child_id, "modal" => true}

    assert_push "command", %{
      "command" => "setFormProperty",
      "form_id" => ^child_id,
      "property" => "windowTitle"
    }

    assert_push "command", %{
      "command" => "setProperty",
      "form_id" => ^child_id,
      "object_name" => "label_1",
      "property" => "text",
      "value" => "child-ready"
    }

    # Child opens the second modal layer.
    send_event(socket, child_id, "pushButton_1", "Clicked")

    assert_push "command", %{
      "command" => "setProperty",
      "form_id" => ^child_id,
      "object_name" => "label_1",
      "property" => "text",
      "value" => "grandchild-opening"
    }

    assert_push "command", %{
      "command" => "createForm",
      "form_id" => grandchild_id,
      "modal" => true
    }

    assert grandchild_id != child_id

    assert_push "command", %{
      "command" => "setFormProperty",
      "form_id" => ^grandchild_id,
      "property" => "windowTitle"
    }

    assert_push "command", %{
      "command" => "setProperty",
      "form_id" => ^grandchild_id,
      "object_name" => "cellInfo",
      "property" => "text",
      "value" => "grandchild-ready"
    }

    # Both blocked layers receive events; both must queue.
    send_event(socket, parent_id, "pushButton_2", "Clicked")
    send_event(socket, child_id, "pushButton_2", "Clicked")

    refute_receive %Phoenix.Socket.Message{
                     event: "command",
                     payload: %{
                       "command" => "setProperty",
                       "form_id" => ^parent_id,
                       "object_name" => "label_2",
                       "property" => "text",
                       "value" => "queued"
                     }
                   },
                   200

    # The active layer closes; results unwind child -> parent.
    send_event(socket, grandchild_id, "close", "Clicked")

    assert_push "command", %{"command" => "closeForm", "form_id" => ^grandchild_id}

    assert_push "command", %{
      "command" => "setProperty",
      "form_id" => ^child_id,
      "object_name" => "label_1",
      "property" => "text",
      "value" => "grand:grand-ok"
    }

    assert_push "command", %{"command" => "closeForm", "form_id" => ^child_id}

    assert_push "command", %{
      "command" => "setProperty",
      "form_id" => ^parent_id,
      "object_name" => "label_1",
      "property" => "text",
      "value" => "closed:child:grand-ok"
    }

    assert_push "command", %{
      "command" => "setProperty",
      "form_id" => ^parent_id,
      "object_name" => "label_2",
      "property" => "text",
      "value" => "queued"
    }

    # The child's queued event must be discarded once it starts closing.
    refute_receive %Phoenix.Socket.Message{
                     event: "command",
                     payload: %{"command" => "closeForm", "form_id" => ^child_id}
                   },
                   200
  end

  test "modal child can read widget properties while the parent is blocked", %{
    socket: socket
  } do
    parent_id = start_dialog(socket, "test_dialog01.ui", "test_modal_property_parent.lua")

    assert_push "command", %{"command" => "createForm", "form_id" => ^parent_id}

    assert_push "command", %{
      "command" => "setFormProperty",
      "form_id" => ^parent_id,
      "property" => "windowTitle"
    }

    assert_push "command", %{
      "command" => "setProperty",
      "form_id" => ^parent_id,
      "object_name" => "label_1",
      "property" => "text",
      "value" => "ready"
    }

    send_event(socket, parent_id, "pushButton_1", "Clicked")

    assert_push "command", %{
      "command" => "setProperty",
      "form_id" => ^parent_id,
      "object_name" => "label_1",
      "property" => "text",
      "value" => "opening"
    }

    assert_push "command", %{"command" => "createForm", "form_id" => child_id, "modal" => true}

    assert_push "command", %{
      "command" => "setFormProperty",
      "form_id" => ^child_id,
      "property" => "windowTitle"
    }

    assert_push "command", %{
      "command" => "setProperty",
      "form_id" => ^child_id,
      "object_name" => "label_1",
      "property" => "text",
      "value" => "child-ready"
    }

    send_event(socket, child_id, "pushButton_1", "Clicked")

    assert_push "command", %{
      "command" => "getProperty",
      "form_id" => ^child_id,
      "object_name" => "lineEdit_1",
      "property" => "text",
      "ref" => ref
    }

    push(socket, "send_event", %{
      "form_id" => child_id,
      "command" => "getProperty",
      "ref" => ref,
      "value" => "abc"
    })

    assert_push "command", %{
      "command" => "setProperty",
      "form_id" => ^child_id,
      "object_name" => "label_2",
      "property" => "text",
      "value" => "read:abc"
    }

    assert_push "command", %{"command" => "closeForm", "form_id" => ^child_id}

    assert_push "command", %{
      "command" => "setProperty",
      "form_id" => ^parent_id,
      "object_name" => "label_1",
      "property" => "text",
      "value" => "closed:ok"
    }
  end

  test "client Close on a modal child releases the parent and stops the child", %{
    socket: socket,
    app_id: app_id
  } do
    parent_id = start_dialog(socket, "test_dialog01.ui", "test_modal_flow.lua")
    expect_parent_boot(socket, parent_id)
    child_id = open_flow_child(socket, parent_id)

    send_event(socket, child_id, child_id, "Close")

    assert_push "command", %{
      "command" => "setProperty",
      "form_id" => ^parent_id,
      "object_name" => "label_1",
      "property" => "text",
      "value" => "closed:nil"
    }

    wait_until(fn -> DialogManager.whereis(app_id, child_id) == {:error, :not_found} end)
  end

  test "closeApplication during a modal stops every session", %{
    socket: socket,
    app_id: app_id
  } do
    sibling_id = start_dialog(socket, "test_dialog03.ui", "test_modal_sibling.lua")

    assert_push "command", %{"command" => "createForm", "form_id" => ^sibling_id}

    assert_push "command", %{
      "command" => "setFormProperty",
      "form_id" => ^sibling_id,
      "property" => "windowTitle"
    }

    assert_push "command", %{
      "command" => "setProperty",
      "form_id" => ^sibling_id,
      "object_name" => "cellInfo",
      "property" => "text",
      "value" => "sibling-ready"
    }

    parent_id = start_dialog(socket, "test_dialog01.ui", "test_modal_flow.lua")
    expect_parent_boot(socket, parent_id)
    child_id = open_flow_child(socket, parent_id)

    send_event(socket, sibling_id, "alwaysOn", "Clicked")

    assert_push "command", %{"command" => "closeApplication", "application_id" => ^app_id}

    wait_until(fn ->
      DialogManager.whereis(app_id, parent_id) == {:error, :not_found} and
        DialogManager.whereis(app_id, child_id) == {:error, :not_found} and
        DialogManager.whereis(app_id, sibling_id) == {:error, :not_found}
    end)
  end

  test "parent death closes the modal child and releases its monitor", %{
    socket: socket,
    app_id: app_id
  } do
    parent_id = start_dialog(socket, "test_dialog01.ui", "test_modal_flow.lua")
    expect_parent_boot(socket, parent_id)
    child_id = open_flow_child(socket, parent_id)

    assert {:ok, _child_pid} = DialogManager.whereis(app_id, child_id)

    DialogManager.stop(app_id, parent_id)

    assert_push "command", %{"command" => "closeForm", "form_id" => ^child_id}
    wait_until(fn -> DialogManager.whereis(app_id, child_id) == {:error, :not_found} end)
  end

  test "crashed modal child releases the parent and closes on the client", %{
    socket: socket,
    app_id: app_id
  } do
    parent_id = start_dialog(socket, "test_dialog01.ui", "test_modal_flow.lua")
    expect_parent_boot(socket, parent_id)
    child_id = open_flow_child(socket, parent_id)

    {:ok, child_pid} = DialogManager.whereis(app_id, child_id)
    Process.exit(child_pid, :kill)

    assert_push "command", %{"command" => "closeForm", "form_id" => ^child_id}

    assert_push "command", %{
      "command" => "setProperty",
      "form_id" => ^parent_id,
      "object_name" => "label_1",
      "property" => "text",
      "value" => "closed:error"
    }

    wait_until(fn -> DialogManager.whereis(app_id, child_id) == {:error, :not_found} end)
  end

  test "modal child boot failure reports dialogError and releases the parent", %{
    socket: socket,
    app_id: app_id
  } do
    parent_id = start_dialog(socket, "test_dialog01.ui", "test_modal_broken_parent.lua")

    assert_push "command", %{"command" => "createForm", "form_id" => ^parent_id}

    assert_push "command", %{
      "command" => "setFormProperty",
      "form_id" => ^parent_id,
      "property" => "windowTitle"
    }

    assert_push "command", %{
      "command" => "setProperty",
      "form_id" => ^parent_id,
      "object_name" => "label_1",
      "property" => "text",
      "value" => "ready"
    }

    send_event(socket, parent_id, "pushButton_1", "Clicked")

    assert_push "command", %{
      "command" => "setProperty",
      "form_id" => ^parent_id,
      "object_name" => "label_1",
      "property" => "text",
      "value" => "opening"
    }

    assert_push "command", %{
      "command" => "dialogError",
      "form_id" => child_id,
      "error" => error
    }

    assert error =~ "boom"

    assert_push "command", %{"command" => "closeForm", "form_id" => ^child_id}

    assert_push "command", %{
      "command" => "setProperty",
      "form_id" => ^parent_id,
      "object_name" => "label_1",
      "property" => "text",
      "value" => "closed:error"
    }

    wait_until(fn -> DialogManager.whereis(app_id, child_id) == {:error, :not_found} end)
  end
end
