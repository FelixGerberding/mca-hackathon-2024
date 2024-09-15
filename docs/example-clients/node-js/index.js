const WebSocketClient = require("websocket").client;

// some Node.js magic to get every token on stdin without pressing enter.
process.stdin.setRawMode(true);
process.stdin.resume();
process.stdin.setEncoding("utf8");

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

let socketConnection = null;
let currentTick = null;

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
  connection.on("message", function (message) {
    let parsedMessage = JSON.parse(message.utf8Data);
    console.log(
      "Received new game state:\n",
      JSON.stringify(parsedMessage, null, 2)
    );

    currentTick = parsedMessage.tick;
  });
});

// Connect to lobby
client.connect(address);

process.stdin.on("data", (key) => {
  console.log(`\n\nKey '${key}' (length ${key.length}) pressed.\n\n`);

  // Match Ctrl + C to abort the program. Is otherwise prevented by `process.stdin.setRawMode(true);`.
  if (key === "\u0003") {
    process.exit();
  }

  // Match regular key presses
  if (key.length === 1) {
    switch (key) {
      case "w":
        sendMessage({
          socketConnection,
          tick: currentTick,
          payload: {
            action: "UP",
          },
        });
        break;
      case "s":
        sendMessage({
          socketConnection,
          tick: currentTick,
          payload: {
            action: "DOWN",
          },
        });
        break;
      case "a":
        sendMessage({
          socketConnection,
          tick: currentTick,
          payload: {
            action: "LEFT",
          },
        });
        break;
      case "d":
        sendMessage({
          socketConnection,
          tick: currentTick,
          payload: {
            action: "RIGHT",
          },
        });
        break;
      case " ":
        sendMessage({
          socketConnection,
          tick: currentTick,
          payload: {
            action: "SHOOT",
          },
        });
        break;
      default:
        console.log(`\nKey ${key} triggers no special action.`);
    }
  } else {
    // Arrow keys have length three for some reason.
    switch (key.charCodeAt(2)) {
      case 65: // UP
        sendMessage({
          socketConnection,
          tick: currentTick,
          payload: {
            action: "TURN",
            degrees: 0,
          },
        });
        break;
      case 66: // DOWN
        sendMessage({
          socketConnection,
          tick: currentTick,
          payload: {
            action: "TURN",
            degrees: 180,
          },
        });
        break;
      case 67: // RIGHT
        sendMessage({
          socketConnection,
          tick: currentTick,
          payload: {
            action: "TURN",
            degrees: 90,
          },
        });
        break;
      case 68: // LEFT
        sendMessage({
          socketConnection,
          tick: currentTick,
          payload: {
            action: "TURN",
            degrees: 270,
          },
        });
        break;
      default:
        console.log("Don't press this button.");
    }
  }
});

const sendMessage = ({ socketConnection, tick, payload }) => {
  if (socketConnection == null) {
    console.log("There is no connection that can be used.");
    return;
  }

  if (tick == null) {
    console.log("There is no message to respond to.");
    return;
  }

  const message = {
    ...payload,
    tick,
  };

  console.log("Sending message", message);

  socketConnection.send(JSON.stringify(message));

  currentTick = null;
};
