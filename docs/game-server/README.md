# Flow of information

This document is meant to the describe the flow of information and processing across server and various clients.

## Actors

We will distinguish between three types of actors:

1. Server
1. Player client
1. Visualization client

### Server

The server manages lobbies and active connections and is responsible to keep track of the game state as well as updating it.
The server will regurlarly update connected clients of a lobby with the new game state, after processing updates.

### Player Client

A player client will connect to the server and join a lobby.
Once connected the player client will be provided with regular updates of the game state (once per turn).
The player client is responsible to process the game state and return an action to update the game state for _his_ controlled entity.

### Visualization Client

The visualization client will also connect to the server and join a lobby.
The visualization client will play no role in updating the game state and can be imagined as a passive spectator.
The server will nonetheless share game state updates with the visualization client.
The game state will then be used to render a graphical representation of the entities positions, actions, etc.

## Format

The server will provide a websocket interface for clients to connect to.
The websockets will be used to update the clients as well as to receive updates from player clients.
All information exchange will use a common JSON schema.

## Messages

This section is concerned about which messages are exchanged between clients and server.

### Client Connection

Clients are expected to present the following information during initial connection.

- `clientType: ["PLAYER" | "SPECTATOR"]`
- `lobby: <UUID>`
- `username: <string>` (optional for spectators)

The parameters are meant to be encoded in the URL used for connection.
The following schema is to be used.

`ws://<gamehost>/{lobby}?clientType=<PLAYER|SPECTATOR>&username=<string>`

After successful connection, the server will send a "client hello" to let the player that his connection was established.
The mesasge will also contain the player's UUID, which will be used during game state updates (see [Game Update section](#game-update)).
If connection is not possible, the server will close the web socket connection and provide a reason in the socket's close message.

_Example for successful connection:_

```json
{
  "success": true,
  "message": "Connection successful.",
  "player_id": "113b09b7-6b8e-48b5-8e20-84ce16ae7901"
}
```

### Game Update

A game update of the server will look like the following.
The `tick` id is used to clearly identify a turn and map it back to the player responses.

_Example game state update:_

```json
{
  "tick": "0fb3b3d2-fc1d-40dc-a2cf-24baaef6ddb1",
  "tick_length_milli_seconds": 2000,
  "players": [
    {
      "entity_type": "PLAYER",
      "id": "113b09b7-6b8e-48b5-8e20-84ce16ae7901",
      "name": "node-js-example-client",
      "x": 14,
      "y": 14,
      "rotation": 0,
      "color": "#FF0000",
      "health": 100,
      "last_action_success": true,
      "error_message": ""
    }
  ],
  "entities": [
    {
      "id": "aafc1830-af30-4580-ba34-285daab262c7",
      "previous_x": 14,
      "previous_y": 20,
      "x": 14,
      "y": 26,
      "travel_distance": 6,
      "direction": 0
    }
  ],
  "spectators": 1
}
```

`players` lists all players, their current position, rotation and health.
Furhtermore, it is indicated whether a player's last action was successful or not.
More details can be found in the [Error Handling section](#error-handling).

`entities` lists all projectiles. The information includes their previous position, current position, the direction and how many units they will travel into their current direction during the next turn.

### Player Actions

#### Submitting Actions

Per turn a player can select from one of the following actions:

- `TURN` (0 - 360°)
  - additional parameter: `degrees` (new direction the player should look at)
- `SHOOT`
- `TURN`
- `UP`
- `DOWN`
- `LEFT`
- `RIGHT`

The value of `tick` should match the id returned as part of the prior game update.

_Example to shoot:_

```json
{
  "tick": "a3d1dbfc-a490-4cbd-bb42-d33a4d80e94a",
  "action": "SHOOT"
}
```

_Example to move upwards:_

```json
{
  "tick": "a3d1dbfc-a490-4cbd-bb42-d33a4d80e94a",
  "action": "UP"
}
```

_Example to turn the player towards 273°:_

```json
{
  "tick": "a3d1dbfc-a490-4cbd-bb42-d33a4d80e94a",
  "action": "TURN",
  "degrees": "273"
}
```

#### Error Handling

The server may deny a player's action, due to a multitude of reasons.
For example:

- Missing parameters (think about the `degrees` from above)
- Moving into the questioned direction is not allowed (the player is already at the edge of the playing field)
- The client used an outdated `tick`

If the last player action was denied, the server will indicate this by setting the `last_action_success` parameter for that player to `false`.
The `error_message` parameter will contain the reason why the message was denied.
