# Flow of information

This document is meant to the describe the flow of information and processing across server and various clients.

## Actors

We will distinguishe three types of actors:

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
The visualuzation client will play no role in updating the game state and can be imagined as a passive spectator.
The server will nonetheless share game state updates with the visualization client.
The game state will then be used to render a graphical representation of the entities positions, actions, etc.

## Format

The server will provide a websocket interface for clients to connect to.
The websockets will be used to update the clients as well as to receive updates from player clients.
All information exchange will use a common JSON schema.

## Messages

> **Note** As a starting point to establish the interface for communicaiton, we will focus on player connection and movement

### Client Connection

Clients are expected to present the following information during initial connection.

- `clientType: ["PLAYER" | "SPECTATOR"]`
- `lobby: <UUID>`
- `username: <string>` (optional for spectators)

The parameters are meant to be encoded in the URL used for connection.
The following schema is to be used.

`ws://<gamehost>/{lobby}?clientType=<PLAYER|SPECTATOR>&username=<string>`

### Game Update

A game update of the server will look like the following (for now).
The `tick` id is used to clearly identify a turn and map it back the player responses.

```json
{
  "tick": "<UUID>",
  "entities": [
    {
      "entityType": "PLAYER",
      "id": "<UUID>",
      "name": "<string>",
      "x": "<int>",
      "y": "<int>",
      "rotation": "<int>",
      "color": "<hex",
      "health": "<int>",
      "lastActionSuccess": "<boolean>",
      "errorMessage": "<string>"
    },
    ...
  ]
}
```

### Player Action

Per turn a player can select from one of the following actions:

- TURN (0 - 360°)
- MOVE UP (1 square)
- MOVE DOWN (1 square)
- MOVE LEFT (1 square)
- MOVE RIGHT (1 square)

The value of `tick` should match the id returned as part of the prior game update.

```json
{
  "tick": "<UUID>",
  "action": "<string>"
}
```
