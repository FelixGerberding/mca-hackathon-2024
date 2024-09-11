import { StrictMode } from "react";
import { createRoot } from "react-dom/client";
import Game from "./game.tsx";
import "./index.css";
import { BrowserRouter as Router, Route, Routes } from "react-router-dom";
import StartPage from "./start-page.tsx";
createRoot(document.getElementById("root")!).render(
  <StrictMode>
    <Router>
      <Routes>
        <Route path="/" element={<StartPage />} />
        <Route path="/:lobbyId" element={<Game />} />
      </Routes>
    </Router>
  </StrictMode>,
);
