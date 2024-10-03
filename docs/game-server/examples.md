## Example Messages for Client-Server Communication

### Establishing a Websocket Connection

`wss://{gamehost}/lobby/{lobby_id}?clientType={PLAYER|SPECTATOR}&username={string}`

### Client to Server Messages

#### 1. Joining a Lobby

```json
{
  "action": "join_lobby",
  "lobby_id": "123e4567-e89b-12d3-a456-426614174000",
  "client_type": "PLAYER",
  "username": "Player1"
}
```

**Description:** Client requests to join a specific lobby as a player with the username "Player1".

#### 2. Submitting a TURN Action

```json
{
  "action": "TURN",
  "degrees": 90
}
```

**Description:** Client requests to turn 90 degrees.

#### 3. Submitting a MOVE UP Action

```json
{
  "action": "UP"
}
```

**Description:** Client requests to move up by one unit.

#### 4. Submitting a MOVE DOWN Action

```json
{
  "action": "DOWN"
}
```

**Description:** Client requests to move down by one unit.

#### 5. Submitting a MOVE RIGHT Action

```json
{
  "action": "RIGHT"
}
```

**Description:** Client requests to move right by one unit.

#### 6. Submitting a MOVE LEFT Action

```json
{
  "action": "LEFT"
}
```

**Description:** Client requests to move left by one unit.

### Server to Client Messages

#### 1. Acknowledging Lobby Join

```json
{
  "status": "success",
  "message": "Joined lobby successfully",
  "lobby_id": "123e4567-e89b-12d3-a456-426614174000"
}
```

**Description:** Server confirms that the client has successfully joined the lobby.

#### 2. Error Message for Invalid Action

```json
{
  "status": "error",
  "message": "Cannot move UP, because player is at border of field",
  "action": "UP"
}
```

**Description:** Server informs the client that their move up action failed because the player is at the border of the field.

#### 3. Broadcasting Game State Update

```json
{
  "tick": "789e4567-e89b-12d3-a456-426614174001",
  "tick_length_milli_seconds": 2000,
  "entities": [],
  "players": [
    {
      "id": "a3bb189e-8bf9-3888-9912-ace4e6543002",
      "name": "Player1",
      "x": 6,
      "y": 6,
      "rotation": 90,
      "color": "#FF00FF",
      "health": 100,
      "last_action_success": true,
      "error_message": ""
    },
    {
      "id": "b2cb2a44-29d6-422e-b327-6e892b4fa5c6",
      "name": "Player2",
      "x": 5,
      "y": 5,
      "rotation": 0,
      "color": "#00FF00",
      "health": 100,
      "last_action_success": true,
      "error_message": ""
    }
  ]
}
```

**Description:** Server sends an updated game state to all clients, indicating the new positions and statuses of all players.

#### 4. Game End Message

```json
{
  "status": "finished",
  "message": "Game has ended. Maximum rounds reached."
}
```

**Description:** Server informs the clients that the game has ended because the maximum number of rounds has been reached.
