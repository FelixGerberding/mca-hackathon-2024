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
  def main(args) do
    case args do
      [base_address | _] ->
        run_client(base_address)
      [] ->
        IO.puts(:stderr, "Error: Base address not provided")
        IO.puts(:stderr, "Usage: elixir websocket_client.exs <base_address>")
        System.halt(1)
    end
  end

  defp run_client(base_address) do
    if base_address == "" do
      IO.puts(:stderr, "Cannot connect to '#{base_address}'")
      System.halt(1)
    end

    address = "#{base_address}?clientType=PLAYER&username=elixir-example-client"
    IO.puts("Starting client. Connecting to #{address}")

    case WebSocketClient.start_link(address) do
      {:ok, _pid} ->
        IO.puts("Connection established successfully.")
        # Keep the process alive
        Process.sleep(:infinity)
      {:error, %WebSockex.RequestError{code: code, message: message}} ->
        IO.puts(:stderr, "Failed to connect: HTTP #{code} - #{message}")
        IO.puts(:stderr, "Please check the server address and your network connection.")
        System.halt(1)
      {:error, reason} ->
        IO.puts(:stderr, "Failed to connect: #{inspect(reason)}")
        IO.puts(:stderr, "Please check your configuration and try again.")
        System.halt(1)
    end
  end
end

Main.main(System.argv())
