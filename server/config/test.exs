import Config

# We don't run a server during test. If one is required,
# you can enable the server option below.
config :e_platform, EPlatformWeb.Endpoint,
  http: [ip: {127, 0, 0, 1}, port: 4002],
  secret_key_base: "XjSh9vm5cd1ynY+7f0yMVm2Y8rUCwEf3pdQYImZtJ0vIkkbJ0GON79Mpczk9pEyd",
  server: false

# Print only warnings and errors during test
config :logger, level: :warning

# Initialize plugs at runtime for faster test compilation
config :phoenix, :plug_init_mode, :runtime

# Sort query params output of verified routes for robust url comparisons
config :phoenix,
  sort_verified_routes_query_params: true
