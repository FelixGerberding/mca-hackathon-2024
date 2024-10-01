defmodule WebSocketClient do
  use WebSockex

  @fixed_payloads [
    %{action: "RIGHT"},
    %{action: "TURN", degrees: 0},
    %{action: "RIGHT"},
    %{action: "DOWN"},
    %{action: "DOWN"},
    %{action: "TURN", degrees: 270},
    %{action: "LEFT"},
    %{action: "LEFT"},
    %{action: "UP"},
    %{action: "UP"}
  ]

  def start_link(url) do
    WebSockex.start_link(url, __MODULE__, %{counter: 0})
  end

  def handle_connect(_conn, state) do
    IO.puts("WebSocket Client Connected.")
    {:ok, state}
  end

  def handle_frame({:text, msg}, state) do
    parsed_message = Jason.decode!(msg)
    IO.puts("Received new game state:\n#{Jason.encode!(parsed_message, pretty: true)}")

    payload = Enum.at(@fixed_payloads, rem(state.counter, length(@fixed_payloads)))
    message = Map.put(payload, :tick, parsed_message["tick"])

    IO.puts("Sending message #{inspect(message)}")
    {:reply, {:text, Jason.encode!(message)}, %{state | counter: state.counter + 1}}
  end

  def handle_disconnect(%{reason: {:local, reason}}, state) do
    IO.puts("Local close with reason: #{inspect(reason)}")
    {:ok, state}
  end

  def handle_disconnect(disconnect_map, state) do
    super(disconnect_map, state)
  end
end

defmodule Main do
  def main do
    base_address = System.get_env("BASE_ADDRESS")

    if is_nil(base_address) or base_address == "" do
      IO.puts(:stderr, "Cannot connect to '#{base_address}'")
      System.halt(1)
    end

    address = "#{base_address}?clientType=PLAYER&username=elixir-example-client"
    IO.puts("Starting client. Connecting to #{address}")

    {:ok, _pid} = WebSocketClient.start_link(address)

    # Keep the process alive
    Process.sleep(:infinity)
  end
end

Main.main()
