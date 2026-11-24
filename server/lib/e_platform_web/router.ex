defmodule EPlatformWeb.Router do
  use EPlatformWeb, :router

  pipeline :api do
    plug :accepts, ["json"]
  end

  scope "/api", EPlatformWeb do
    pipe_through :api
  end
end
