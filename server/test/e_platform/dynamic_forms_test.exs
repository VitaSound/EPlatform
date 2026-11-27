defmodule EPlatform.DynamicFormsTest do
  @moduledoc "Plan 44: declarative + imperative forms without on-disk .ui."
  use ExUnit.Case, async: false

  import Phoenix.ChannelTest

  @endpoint EPlatformWeb.Endpoint

  setup do
    app_id = "app-" <> EPlatform.ID.generate()
    {:ok, socket} = connect(EPlatformWeb.UserSocket, %{})
    {:ok, _, socket} = subscribe_and_join(socket, "app:" <> app_id)
    %{socket: socket, app_id: app_id}
  end

  defp start_dialog(socket, lua_file) do
    ref = push(socket, "start_dialog", %{"lua_file" => lua_file, "ui_file" => ""})
    assert_reply ref, :ok, %{"form_id" => form_id}
    form_id
  end

  test "declarative demo emits createForm with inline ui_xml", %{socket: socket} do
    form_id = start_dialog(socket, "test_dyn_declarative.lua")

    assert_push "command", %{
      "command" => "createForm",
      "form_id" => ^form_id,
      "ui_xml" => ui_xml
    }

    assert is_binary(ui_xml)
    assert ui_xml =~ ~s(name="DynDeclarative")
    assert ui_xml =~ ~s(name="nameEdit")
    assert ui_xml =~ ~s(name="ok")
    refute File.exists?(
             Path.join(:code.priv_dir(:e_platform), "qt_forms/test_dyn_declarative.ui")
           )
  end

  test "imperative demo emits empty shell then addLayout/addWidget", %{socket: socket} do
    form_id = start_dialog(socket, "test_dyn_imperative.lua")

    assert_push "command", %{
      "command" => "createEmptyForm",
      "form_id" => ^form_id
    }

    assert_push "command", %{
      "command" => "addLayout",
      "form_id" => ^form_id,
      "class" => "QVBoxLayout",
      "object_name" => "root"
    }

    assert_push "command", %{
      "command" => "addWidget",
      "form_id" => ^form_id,
      "class" => "QLabel",
      "object_name" => "title",
      "parent" => "root"
    }

    assert_push "command", %{
      "command" => "addWidget",
      "form_id" => ^form_id,
      "class" => "QPushButton",
      "object_name" => "ok",
      "parent" => "row",
      "properties" => %{"text" => "OK"}
    }
  end
end
