import { useCallback, useEffect, useState } from "react";

import { type AppError, getAppStatus, isAppError, retryDatabase } from "./lib/database";
import "./App.css";

type ViewState = "loading" | "ready" | "error";

function asAppError(error: unknown): AppError {
  if (isAppError(error)) {
    return error;
  }

  return {
    code: "INTERNAL_ERROR",
    message: "Ocurrió un error interno. Inténtalo nuevamente.",
  };
}

function App() {
  const [viewState, setViewState] = useState<ViewState>("loading");
  const [version, setVersion] = useState("");
  const [error, setError] = useState<AppError | null>(null);

  const readStatus = useCallback(async () => {
    setViewState("loading");
    setError(null);

    try {
      const status = await getAppStatus();
      setVersion(status.version);
      setViewState("ready");
    } catch (statusError: unknown) {
      setError(asAppError(statusError));
      setViewState("error");
    }
  }, []);

  const handleRetry = useCallback(async () => {
    setViewState("loading");
    setError(null);

    try {
      const status = await retryDatabase();
      setVersion(status.version);
      setViewState("ready");
    } catch (retryError: unknown) {
      setError(asAppError(retryError));
      setViewState("error");
    }
  }, []);

  useEffect(() => {
    void readStatus();
  }, [readStatus]);

  if (viewState === "loading") {
    return (
      <main className="app-shell" aria-busy="true" aria-live="polite">
        <section className="status-card" aria-labelledby="loading-title">
          <span className="status-mark status-mark--loading" aria-hidden="true">
            …
          </span>
          <p className="eyebrow">Inventario Remolino del Pez</p>
          <h1 id="loading-title">Preparando la aplicación</h1>
          <p>Estamos preparando el almacenamiento local. Espera un momento.</p>
        </section>
      </main>
    );
  }

  if (viewState === "error" && error) {
    return (
      <main className="app-shell" aria-live="assertive">
        <section
          className="status-card status-card--error"
          role="alert"
          aria-labelledby="error-title"
        >
          <span className="status-mark status-mark--error" aria-hidden="true">
            !
          </span>
          <p className="eyebrow">Inventario Remolino del Pez</p>
          <h1 id="error-title">No se pudo iniciar la aplicación</h1>
          <p>{error.message}</p>
          <button type="button" onClick={() => void handleRetry()}>
            Reintentar
          </button>
        </section>
      </main>
    );
  }

  return (
    <main className="app-shell" aria-live="polite">
      <section className="status-card" aria-labelledby="ready-title">
        <span className="status-mark status-mark--ready" aria-hidden="true">
          ✓
        </span>
        <p className="eyebrow">Inventario Remolino del Pez</p>
        <h1 id="ready-title">Aplicación lista</h1>
        <p>El almacenamiento local está preparado para los módulos del inventario.</p>
        <p className="version">Versión {version}</p>
      </section>
    </main>
  );
}

export default App;
