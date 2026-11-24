defmodule EPlatform.Application do
  # See https://hexdocs.pm/elixir/Application.html
  # for more information on OTP Applications
  @moduledoc false

  use Application

  @impl true
  def start(_type, _args) do
    children = [
      EPlatformWeb.Telemetry,
      {DNSCluster, query: Application.get_env(:e_platform, :dns_cluster_query) || :ignore},
      {Phoenix.PubSub, name: EPlatform.PubSub},
      {Registry, keys: :unique, name: EPlatform.DialogRegistry},
      {DynamicSupervisor, strategy: :one_for_one, name: EPlatform.DialogSupervisor},
      # Start to serve requests, typically the last entry
      EPlatformWeb.Endpoint
    ]

    # See https://hexdocs.pm/elixir/Supervisor.html
    # for other strategies and supported options
    opts = [strategy: :one_for_one, name: EPlatform.Supervisor]
    Supervisor.start_link(children, opts)
  end

  # Tell Phoenix to update the endpoint configuration
  # whenever the application is updated.
  @impl true
  def config_change(changed, _new, removed) do
    EPlatformWeb.Endpoint.config_change(changed, removed)
    :ok
  end
end
