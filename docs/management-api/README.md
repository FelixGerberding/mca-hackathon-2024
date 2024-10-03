# Management API Documentation

This document provides an overview of the Management API, including descriptions and usage examples for each endpoint. The Management API allows you to manage lobbies, including creating, retrieving, updating, and managing the status of lobbies.

## Variables

Variables are referred to using the `{{variable_name}}` notation.

_Variables used below:_

- Lobby ID
  - **Key:** `lobby_id`
  - **Value:** The unique identifier for a lobby (to be provided by the user when performing operations on specific lobbies).
- Base URL
  - **Key:** `url`
  - **Value:** Base URL to connect to the API.
    - `http://localhost:8081` for local development
    - `https://mca-client.felix.codes` for production

## Authentication

No authentication is required for the management API.

## Endpoints

### 1. Create Lobby

**Endpoint:** `POST /lobbies`

This endpoint is used to create a new lobby. No parameters are required in the request body.
After creation, the lobby will be in the "PENDING" status.
Players can connect to the lobby, but the server will post no regular game updates yet.

_Request Example:_

```
POST {{url}}/lobbies
```

### 2. Get Lobbies

**Endpoint:** `GET /lobbies`

This endpoint retrieves a list of all lobbies.
No parameters are required in the request.
This is used for displaying a list of available lobbies to the user.

_Request Example:_

```
GET {{url}}/lobbies
```

### 3. Restart Lobby

**Endpoint:** `PATCH /lobbies/{{lobby_id}}`

This endpoint updates the status of a lobby to "PENDING," indicating that the lobby is open and awaiting participants.
This is used when the lobby is closed and needs to be reopened.

_Request Example:_

```
PATCH {{url}}/lobbies/{{lobby_id}}
Content-Type: application/json

{
  "status": "PENDING"
}
```

### 4. Start Lobby

**Endpoint:** `PATCH /lobbies/{{lobby_id}}`

This endpoint updates the status of a lobby to "RUNNING," indicating that the lobby has started.
As soon as the lobby is running, clients need to submit their actions to the server.

_Request Example:_

```
PATCH {{url}}/lobbies/{{lobby_id}}
Content-Type: application/json

{
  "status": "RUNNING"
}
```

### 5. Stop Lobby

**Endpoint:** `PATCH /lobbies/{{lobby_id}}`

This endpoint updates the status of a lobby to "FINISHED," indicating that the lobby has ended.

_Request Example:_

```
PATCH {{url}}/lobbies/{{lobby_id}}
Content-Type: application/json

{
  "status": "FINISHED"
}
```
