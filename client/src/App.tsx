import React, { useEffect, useState } from "react";
import "./App.css";
import { ArrowUp, Rocket } from "lucide-react";

const GRID_SIZE = 30;

const playerColors = ["red", "blue", "green", "purple"];

interface PlayerProps {
  x: number;
  y: number;
  direction: "N" | "NE" | "E" | "SE" | "S" | "SW" | "W" | "NW";
  id: number;
}

interface ProjectileProps {
  x: number;
  y: number;
  direction: "N" | "NE" | "E" | "SE" | "S" | "SW" | "W" | "NW";
}

const Player: React.FC<PlayerProps> = ({ x, y, direction, id }) => {
  const rotation = {
    N: "-45deg", // NE adjusted to point N
    NE: "0deg", // No adjustment needed, as it's already NE
    E: "45deg", // NE adjusted to point E
    SE: "90deg", // NE adjusted to point SE
    S: "135deg", // NE adjusted to point S
    SW: "180deg", // NE adjusted to point SW
    W: "225deg", // NE adjusted to point W
    NW: "270deg", // NE adjusted to point NW
  };
  return (
    <div
      className="player"
      style={{
        color: playerColors[id - 1],
        transform: `rotate(${rotation[direction]})`,
      }}
    >
      <Rocket />
    </div>
  );
};

const Projectile: React.FC<ProjectileProps> = ({ x, y, direction }) => {
  const rotation = {
    N: "0deg",
    NE: "45deg",
    E: "90deg",
    SE: "135deg",
    S: "180deg",
    SW: "225deg",
    W: "270deg",
    NW: "315deg",
  };
  return (
    <div
      className="projectile"
      style={{ transform: `rotate(${rotation[direction]})` }}
    >
      <ArrowUp />
    </div>
  );
};

const App: React.FC = () => {
  const [players, setPlayers] = useState<PlayerProps[]>([
    { x: 0, y: 0, direction: "N", id: 1 },
    { x: 10, y: 0, direction: "E", id: 2 },
    { x: 0, y: 10, direction: "S", id: 3 },
    { x: 10, y: 10, direction: "W", id: 4 },
  ]);
  const [, setWs] = useState<WebSocket | null>(null);
  const [projectiles] = useState<ProjectileProps[]>([
    { x: 11, y: 0, direction: "E" },
  ]);

  useEffect(() => {
    const socket = new WebSocket("ws://localhost:8080");
    setWs(socket);

    socket.onmessage = (message) => {
      const parsedMessage = JSON.parse(message.data);
      if (parsedMessage.type === "gameState") {
        setPlayers(parsedMessage.data.players);
      }
    };

    socket.onclose = () => {
      console.log("WebSocket connection closed");
    };

    return () => {
      socket.close();
    };
  }, []);

  const renderGrid = () => {
    const grid = [];
    for (let y = 0; y < GRID_SIZE; y++) {
      const row = [];
      for (let x = 0; x < GRID_SIZE; x++) {
        row.push(
          <div className="cell" key={`${x}-${y}`}>
            {players.map(
              (player) =>
                player.x === x &&
                player.y === y && <Player key={player.id} {...player} />,
            )}
            {projectiles.map(
              (projectile, index) =>
                projectile.x === x &&
                projectile.y === y && (
                  <Projectile key={`projectile-${index}`} {...projectile} />
                ),
            )}
          </div>,
        );
      }
      grid.push(
        <div className="row" key={y}>
          {row}
        </div>,
      );
    }
    return grid;
  };

  return <div className="game">{renderGrid()}</div>;
};

export default App;
