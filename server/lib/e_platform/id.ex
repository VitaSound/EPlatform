defmodule EPlatform.ID do
  @moduledoc "UUIDv4-like string identifiers (no Ecto dependency)."
  import Bitwise

  @spec generate() :: String.t()
  def generate do
    <<a::32, b::16, c::16, d::16, e::48>> = :crypto.strong_rand_bytes(16)

    :io_lib.format("~8.16.0b-~4.16.0b-~4.16.0b-~4.16.0b-~12.16.0b", [
      a,
      b,
      c ||| 0x4000,
      d ||| 0x8000,
      e
    ])
    |> IO.iodata_to_binary()
  end
end
