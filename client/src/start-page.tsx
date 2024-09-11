import { useState, useEffect } from "react";
import { useNavigate } from "react-router-dom";

type Client = {
  name: string;
  color: string;
  entityType: "PLAYER";
};

type Lobby = {
  id: string;
  name: string;
  status: "PENDING" | "RUNNING" | "FINISHED";
  clients: Client[];
};

export default function StartPage() {
  const [lobbies, setLobbies] = useState<Lobby[]>([]);
  const navigate = useNavigate();

  useEffect(() => {
    const fetchLobbies = async () => {
      try {
        const response = await fetch("http://localhost:8081/lobbies");
        const data = await response.json();
        setLobbies(data.lobbies);
      } catch (error) {
        console.error("Error fetching lobbies:", error);
      }
    };

    fetchLobbies();
  }, []);

  const handleJoinLobby = (lobbyId: string) => {
    navigate(`/${lobbyId}`);
  };

  return (
    <div className="w-full max-w-6xl mx-auto p-4">
      <h2 className="text-2xl font-bold mb-4">Welcome to the Game</h2>
      <div className="space-y-4">
        {lobbies.length === 0 ? (
          <div>Loading lobbies...</div>
        ) : (
          lobbies.map((lobby) => (
            <div
              key={lobby.id}
              className="flex justify-between items-center p-4 border rounded"
            >
              <span>{lobby.id}</span>
              {lobby.clients.length !== 0 &&
                lobby.clients.map((player) => (
                  <span key={player.name} style={{ color: player.color }}>
                    {player.name}
                  </span>
                ))}
              <button
                className="bg-blue-500 text-white px-4 py-2 rounded"
                disabled={lobby.status !== "PENDING"}
                onClick={() => handleJoinLobby(lobby.id)}
              >
                Join
              </button>
            </div>
          ))
        )}
      </div>
    </div>
  );
}
