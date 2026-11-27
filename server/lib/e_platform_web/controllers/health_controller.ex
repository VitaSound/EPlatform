defmodule EPlatformWeb.HealthController do
  use EPlatformWeb, :controller

  def show(conn, _params) do
    json(conn, %{status: "ok"})
  end
end
