import { useState, useEffect, useRef } from "react";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { motion, AnimatePresence } from "framer-motion";
import { Progress } from "@/components/ui/progress";
import { Switch } from "@/components/ui/switch";
import { Label } from "@/components/ui/label";
import Countdown from "react-countdown";
import { useParams } from "react-router-dom";

type Player = {
  id: string;
  name: string;
  x: number;
  y: number;
  rotation: number;
  color: string;
  health: number;
  entity_type: "PLAYER";
};

type Projectile = {
  id: string;
  x: number;
  y: number;
  rotation: number;
  entity_type: "PROJECTILE";
};

type GameState = {
  tick: string;
  players: Player[];
  entities: Projectile[];
};

/*const generateMockGameState = (): GameState => {
  const players: Player[] = [
    {
      id: "p1",
      name: "Alice",
      x: Math.round(Math.random() * 30),
      y: Math.round(Math.random() * 30),
      rotation: Math.random() * 360,
      color: "#ff0000",
      health: Math.floor(Math.random() * 100),
      entity_type: "PLAYER",
    },
    {
      id: "p2",
      name: "Bob",
      x: Math.round(Math.random() * 30),
      y: Math.round(Math.random() * 30),
      rotation: Math.random() * 360,
      color: "#00ff00",
      health: Math.floor(Math.random() * 100),
      entity_type: "PLAYER",
    },
    {
      id: "p3",
      name: "Charlie",
      x: Math.round(Math.random() * 30),
      y: Math.round(Math.random() * 30),
      rotation: Math.random() * 360,
      color: "#0000ff",
      health: Math.floor(Math.random() * 100),
      entity_type: "PLAYER",
    },
  ];

  const projectiles: Projectile[] = Array.from({ length: 5 }, (_, i) => ({
    id: `proj${i}`,
    x: Math.random() * 30,
    y: Math.random() * 30,
    rotation: Math.random() * 360,
    entity_type: "PROJECTILE",
  }));

  return {
    tick: Date.now().toString(),
    entities: projectiles,
    players: players,
  };
};*/

const calculateTrajectoryEndpoint = (
  x: number,
  y: number,
  rotation: number,
  length: number,
) => {
  const radians = (rotation * Math.PI) / 180;
  const endX = x + length * Math.cos(radians);
  const endY = y + length * Math.sin(radians);
  return { endX, endY };
};

export default function Game() {
  const { lobbyId } = useParams();
  const [gameState, setGameState] = useState<GameState | null>(null);
  const [showGrid, setShowGrid] = useState(false);
  const [showTrajectory, setShowTrajectory] = useState(false);
  const wsRef = useRef<WebSocket | null>(null);

  useEffect(() => {
    /*const mockWebSocket = {
      onopen: null as (() => void) | null,
      onmessage: null as ((event: { data: string }) => void) | null,
      onerror: null as ((error: Event) => void) | null,
      onclose: null as (() => void) | null,
      close: () => {},
    };

    wsRef.current = mockWebSocket as unknown as WebSocket;

    if (mockWebSocket.onopen) {
      mockWebSocket.onopen();
    }

    const interval = setInterval(() => {
      const mockData = generateMockGameState();
      if (mockWebSocket.onmessage) {
        mockWebSocket.onmessage({ data: JSON.stringify(mockData) });
      }
    }, 5000);

    return () => {
      clearInterval(interval);
      if (mockWebSocket.onclose) {
        mockWebSocket.onclose();
      }
    };*/

    wsRef.current = new WebSocket(
      `ws://localhost:8080/lobby/${lobbyId}?clientType=SPECTATOR&username=Test`,
    );
  }, [lobbyId]);

  useEffect(() => {
    if (wsRef.current) {
      wsRef.current.onopen = () => {
        console.log("WebSocket connection established");
      };

      wsRef.current.onmessage = (event) => {
        const newGameState: GameState = JSON.parse(event.data);

        setGameState({
          tick: Date.now().toString(),
          players: newGameState.players,
          entities: newGameState.entities,
        });
      };

      wsRef.current.onerror = (error) => {
        console.error("WebSocket error:", error);
      };

      wsRef.current.onclose = () => {
        console.log("WebSocket connection closed");
      };
    }
  }, []);

  if (!gameState) {
    return <div>Connecting to game server...</div>;
  }

  const players = gameState.players;

  const renderGrid = () => {
    if (!showGrid) return null;

    const gridLines = [];
    for (let i = 1; i < 31; i++) {
      gridLines.push(
        <line
          key={`v${i}`}
          x1={i * 10 - 5}
          y1="0"
          x2={i * 10 - 5}
          y2="300"
          stroke="rgba(0,0,0,0.2)"
          strokeWidth="0.5"
        />,
        <line
          key={`h${i}`}
          x1="0"
          y1={i * 10 - 5}
          x2="300"
          y2={i * 10 - 5}
          stroke="rgba(0,0,0,0.2)"
          strokeWidth="0.5"
        />,
      );
    }
    return gridLines;
  };

  return (
    <div className="w-full max-w-6xl mx-auto p-4">
      <h2 className="text-2xl font-bold mb-4">Game Visualization</h2>
      <div className="flex flex-col md:flex-row gap-4">
        <Card className="w-full md:w-1/2">
          <CardContent className="p-4">
            <div className="flex items-center space-x-4 mb-4">
              <div className="flex items-center space-x-2">
                <Switch
                  id="show-grid"
                  checked={showGrid}
                  onCheckedChange={setShowGrid}
                />
                <Label htmlFor="show-grid">Show Grid</Label>
              </div>
              <div className="flex items-center space-x-2">
                <Switch
                  id="show-trajectory"
                  checked={showTrajectory}
                  onCheckedChange={setShowTrajectory}
                />
                <Label htmlFor="show-trajectory">
                  Show Projectile Trajectory
                </Label>
              </div>
            </div>
            <svg
              width="100%"
              height="100%"
              viewBox="0 0 300 300"
              className="border border-gray-300"
            >
              {renderGrid()}
              <AnimatePresence>
                {gameState.players.map((entity) => {
                  return (
                    <motion.g
                      key={entity.id}
                      initial={{ opacity: 0 }}
                      animate={{
                        opacity: 1,
                        x: entity.x * 10,
                        y: entity.y * 10,
                      }}
                      exit={{ opacity: 0 }}
                      transition={{ type: "spring", stiffness: 100 }}
                    >
                      <line
                        x1="0"
                        y1="0"
                        x2="15"
                        y2="0"
                        stroke="black"
                        strokeWidth="1"
                        transform={`rotate(${entity.rotation})`}
                      />
                      <circle r="5" fill={entity.color} />
                      <text
                        y="-10"
                        textAnchor="middle"
                        fill="black"
                        fontSize="8"
                      >
                        {entity.name}
                      </text>
                    </motion.g>
                  );
                })}
                ;
                {gameState.entities.map((entity) => {
                  // Projectile as an arrow
                  const { endX: nextTurnX, endY: nextTurnY } =
                    calculateTrajectoryEndpoint(
                      entity.x,
                      entity.y,
                      entity.rotation,
                      10,
                    );

                  const { endX, endY } = calculateTrajectoryEndpoint(
                    nextTurnX,
                    nextTurnY,
                    entity.rotation,
                    100,
                  );

                  return (
                    <motion.g key={entity.id}>
                      {showTrajectory && (
                        <>
                          <motion.line
                            x1={entity.x * 10}
                            y1={entity.y * 10}
                            x2={nextTurnX * 10}
                            y2={nextTurnY * 10}
                            key={`${entity.id}-next-trajectory`}
                            stroke="rgba(0,0,0,1)"
                            strokeWidth="1"
                            initial={{ pathLength: 0 }}
                            animate={{ pathLength: 1 }}
                            transition={{ duration: 0.5 }}
                          />
                          <motion.line
                            x1={entity.x * 10}
                            y1={entity.y * 10}
                            x2={endX * 10}
                            y2={endY * 10}
                            key={`${entity.id}-trajectory`}
                            stroke="rgba(0,0,0,0.3)"
                            strokeWidth="1"
                            initial={{ pathLength: 0 }}
                            animate={{ pathLength: 1 }}
                            transition={{ duration: 0.5 }}
                          />
                        </>
                      )}
                      <motion.path
                        d="M-5,-2 L5,0 L-5,2 Z"
                        fill="black"
                        initial={{ opacity: 0 }}
                        animate={{
                          opacity: 1,
                          x: entity.x * 10,
                          y: entity.y * 10,
                          rotate: entity.rotation,
                        }}
                        exit={{ opacity: 0 }}
                        transition={{ type: "spring", stiffness: 200 }}
                      />
                    </motion.g>
                  );
                })}
                ;
              </AnimatePresence>
            </svg>
            <div className="mt-4">
              <h3 className="text-lg font-semibold">
                Current Tick:{" "}
                <code className="bg-gray-800 text-white rounded px-2 ">
                  {gameState.tick}
                </code>
                <Countdown
                  key={gameState.tick}
                  date={Date.now() + 2000 - 200}
                  intervalDelay={200}
                  precision={3}
                  renderer={(props) => (
                    <Progress value={(props.total / (5000 - 200)) * 100} />
                  )}
                />
              </h3>
            </div>
          </CardContent>
        </Card>
        <div className="w-full md:w-1/2 space-y-4">
          {players.map((player) => (
            <Card key={player.id}>
              <CardHeader>
                <CardTitle className="flex items-center gap-2">
                  <div
                    className="w-4 h-4 rounded-full"
                    style={{ backgroundColor: player.color }}
                  ></div>
                  {player.name}
                </CardTitle>
              </CardHeader>
              <CardContent>
                <div className="space-y-2">
                  <div className="flex justify-between">
                    <span>Health:</span>
                    <span>{player.health}%</span>
                  </div>
                  <Progress value={player.health} className="w-full" />
                  <div className="flex justify-between">
                    <span>Position:</span>
                    <span>
                      ({player.x.toFixed(2)}, {player.y.toFixed(2)})
                    </span>
                  </div>
                  <div className="flex justify-between">
                    <span>Rotation:</span>
                    <span>{player.rotation.toFixed(2)}°</span>
                  </div>
                </div>
              </CardContent>
            </Card>
          ))}
        </div>
      </div>
    </div>
  );
}
