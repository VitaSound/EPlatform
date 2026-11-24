defmodule EPlatform.Lua.Logic do
  @moduledoc "Compiles Lua scripts from priv/lua_modules with a sandboxed API."
  require Logger

  def compile_file(file_name) do
    priv_dir = :code.priv_dir(:e_platform)
    path = "#{priv_dir}/lua_modules/#{file_name}"

    case File.read(path) do
      {:ok, content} ->
        compile_code(content)

      {:error, _} ->
        raise "Ошибка чтения Lua-файла: #{file_name}"
    end
  end

  def compile_code(lua_code) do
    priv_dir = :code.priv_dir(:e_platform)
    path = "#{priv_dir}/lua_modules/?.lua"

    lua_state =
      Lua.new(exclude: [[:package], [:require]])
      |> Lua.set_lua_paths([path])
      |> Lua.load_api(EPlatform.Lua.Api)

    case Lua.eval!(lua_state, lua_code, []) do
      {:error, reason} ->
        Logger.error("Ошибка загрузки Lua-кода: #{inspect(reason)}")
        {:error, reason}

      {_result, lua_state} ->
        {:ok, lua_state}
    end
  end
end
