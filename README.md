# mca-hackathon-2024

## Idea

This project was used as part of a 2 day hackathon in a small friend group.
It implements a small websocket-based game.
Idea is that each attendee implements their own bot contesting in various tournament rounds throughout the hackathon.

## Code structure

Implementation of the visualization app is done in the [`client/src` directory](./client/src/).
It is a React.js app, which receives the game updates and renders all entities in a grid.

The server is written in Rust and can be found in the [`server/src` directory](./server/src/).

## Documenation

Documentation is available in the [`docs` folder](./docs/).

To get familiar with the exchanged message format, checkout the [Flow of information document](./docs/game-server/README.md).

To learn more about the management API, which can be used administer lobbies, have a look at the [Management API document](./docs/management-api/README.md).

Some [example clients in various languages](./docs/example-clients/) are provided to serve as a starting point for your own implementation.

## Run Local

### Prerequisites

1. Install [Node.js](https://nodejs.org/en/download/package-manager)
1. Install [Rust](https://www.rust-lang.org/tools/install)
1. Install the [Bruno http client](https://www.usebruno.com/downloads)

### Start Basic Local Setup

1. Install the necessary dependencies of the visualization app: `pnpm install` in `./client`
1. Start the app: `npm run start:app:local` in `./`
1. Start the local server: `npm run start:server:local` in `./`
1. Connect to the default lobby: Open http://localhost:5173/9ec2a984-b5bf-4a13-89fd-53c0d9cafef6
1. Connect an example client: `npm run npm run start:node-js-circle-walker:local` in `./`
1. Open the [Bruno collection](./docs/management-api/bruno/Management%20API/)
1. Select the local environment
1. Start the default lobby using the Bruno collection's "Start Lobby" request
