const WebSocketClient = require("websocket").client;

const client = new WebSocketClient();

// Check that address was passed to process
const baseAddress = process.argv[2];
if (baseAddress === undefined || baseAddress === "") {
  console.error(`Cannot connect to '${baseAddress}'`);
  process.exit(1);
}
const address =
  baseAddress + "?clientType=PLAYER&username=node-js-example-client";
console.log(`Starting client. Connecting to ${address}`);

const fixedPayloads = [
  { action: "RIGHT" },
  { action: "TURN", degrees: 0 },
  { action: "RIGHT" },
  { action: "DOWN" },
  { action: "DOWN" },
  { action: "TURN", degrees: 270 },
  { action: "LEFT" },
  { action: "LEFT" },
  { action: "UP" },
  { action: "UP" },
];

let i = 0;

// Handle connection
client.on("connect", function (connection) {
  console.log("WebSocket Client Connected.");

  // Make connection available in outside scope
  socketConnection = connection;

  connection.on("error", function (error) {
    console.log("Connection Error:", error);
  });

  // Handle close events
  connection.on("close", function (closeEvent) {
    console.log("Connection closed:", closeEvent);
  });

  // Handle incoming messages
  connection.on("message", function (incomingMessage) {
    let parsedMessage = JSON.parse(incomingMessage.utf8Data);
    console.log(
      "Received new game state:\n",
      JSON.stringify(parsedMessage, null, 2)
    );

    const message = {
        tick: parsedMessage.tick,
        ...fixedPayloads[i % fixedPayloads.length]
    }

    console.log("Sending message", message);

    connection.send(JSON.stringify(message));

    i++;
  });
});

// Connect to lobby
client.connect(address);
