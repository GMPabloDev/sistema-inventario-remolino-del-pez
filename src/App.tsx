import { useState } from "react";
import reactLogo from "./assets/react.svg";
import { invoke } from "@tauri-apps/api/core";
import { testDatabaseConnection } from "./lib/database";
import "./App.css";

function App() {
  const [greetMsg, setGreetMsg] = useState("");
  const [name, setName] = useState("");
  const [databaseStatus, setDatabaseStatus] = useState("");

  async function greet() {
    setGreetMsg(await invoke("greet", { name }));
  }

  async function checkDatabaseConnection() {
    try {
      await testDatabaseConnection();
      setDatabaseStatus("Conexión SQLite establecida correctamente.");
    } catch (error) {
      console.error(error);
      setDatabaseStatus("No se pudo conectar con SQLite.");
    }
  }

  return (
    <main className="container">
      <h1>Welcome to Tauri + React</h1>

      <div className="row">
        <a href="https://vite.dev" target="_blank" rel="noreferrer">
          <img src="/vite.svg" className="logo vite" alt="Vite logo" />
        </a>
        <a href="https://tauri.app" target="_blank" rel="noreferrer">
          <img src="/tauri.svg" className="logo tauri" alt="Tauri logo" />
        </a>
        <a href="https://react.dev" target="_blank" rel="noreferrer">
          <img src={reactLogo} className="logo react" alt="React logo" />
        </a>
      </div>
      <p>Click on the Tauri, Vite, and React logos to learn more.</p>

      <form
        className="row"
        onSubmit={(e) => {
          e.preventDefault();
          greet();
        }}
      >
        <input
          id="greet-input"
          onChange={(e) => setName(e.currentTarget.value)}
          placeholder="Enter a name..."
        />
        <button type="submit">Greet</button>
      </form>
      <p>{greetMsg}</p>

      <button type="button" onClick={checkDatabaseConnection}>
        Probar conexión SQLite
      </button>
      <p>{databaseStatus}</p>
    </main>
  );
}

export default App;
