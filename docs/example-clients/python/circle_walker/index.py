import asyncio
import websockets
import json
import sys
import time

# Check that address was passed to process
if len(sys.argv) < 2 or sys.argv[1] == "":
    print(f"Cannot connect to '{sys.argv[1] if len(sys.argv) > 1 else ''}'")
    sys.exit(1)

base_address = sys.argv[1]
address = f"{base_address}?clientType=PLAYER&username=python-example-client"
print(f"Starting client. Connecting to {address}")

fixed_payloads = [
    {"action": "RIGHT"},
    {"action": "TURN", "degrees": 0},
    {"action": "RIGHT"},
    {"action": "DOWN"},
    {"action": "DOWN"},
    {"action": "TURN", "degrees": 270},
    {"action": "LEFT"},
    {"action": "LEFT"},
    {"action": "UP"},
    {"action": "UP"},
]

i = 0


async def client():
    global i
    async with websockets.connect(address) as websocket:
        print("WebSocket Client Connected.")

        try:
            # Handle initial connection message
            initial_message = await websocket.recv()
            initial_data = json.loads(initial_message)
            print("Initial connection message:",
                  json.dumps(initial_data, indent=2))

            if not initial_data.get('success'):
                print("Connection unsuccessful. Exiting.")
                return

            player_id = initial_data.get('player_id')
            print(f"Connected successfully. Player ID: {player_id}")

            # Main game loop
            async for message in websocket:
                parsed_message = json.loads(message)
                print("Received new game state:\n",
                      json.dumps(parsed_message, indent=2))

                response = {
                    "tick": parsed_message["tick"],
                    **fixed_payloads[i % len(fixed_payloads)]
                }
                print("Sending message", response)
                await websocket.send(json.dumps(response))
                time.sleep(0.2)
                i += 1

        except websockets.exceptions.ConnectionClosed as e:
            print("Connection closed:", e)
        except Exception as e:
            print("Connection Error:", e)

asyncio.get_event_loop().run_until_complete(client())
