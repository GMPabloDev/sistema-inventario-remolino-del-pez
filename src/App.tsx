import { FormEvent, useCallback, useEffect, useState } from "react";

import {
  type AppError,
  changePassword,
  getAppStatus,
  getAuthStartup,
  isAppError,
  login,
  logout,
  type UserIdentity,
  retryDatabase,
} from "./lib/database";
import "./App.css";

type ViewState = "loading" | "bootstrap" | "login" | "password-change" | "shell" | "error";

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
  const [identity, setIdentity] = useState<UserIdentity | null>(null);
  const [temporaryPassword, setTemporaryPassword] = useState<string | null>(null);
  const [persistenceWarning, setPersistenceWarning] = useState(false);
  const [error, setError] = useState<AppError | null>(null);
  const [username, setUsername] = useState("");
  const [password, setPassword] = useState("");
  const [currentPassword, setCurrentPassword] = useState("");
  const [newPassword, setNewPassword] = useState("");
  const [busy, setBusy] = useState(false);

  const applyAuthResult = useCallback(
    (result: { identity: UserIdentity; persistenceWarning: boolean }) => {
      setIdentity(result.identity);
      setPersistenceWarning(result.persistenceWarning);
      setViewState(result.identity.mustChangePassword ? "password-change" : "shell");
    },
    [],
  );

  const readStatus = useCallback(async () => {
    setViewState("loading");
    setError(null);

    try {
      const status = await getAppStatus();
      setVersion(status.version);
      const startup = await getAuthStartup();
      setPersistenceWarning(startup.persistenceWarning);
      setIdentity(startup.identity);
      setTemporaryPassword(startup.temporaryPassword);
      if (startup.state === "bootstrap") {
        setViewState("bootstrap");
      } else if (startup.state === "authenticated" && startup.identity) {
        setViewState(startup.identity.mustChangePassword ? "password-change" : "shell");
      } else {
        setViewState("login");
      }
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
      await readStatus();
    } catch (retryError: unknown) {
      setError(asAppError(retryError));
      setViewState("error");
    }
  }, [readStatus]);

  const handleLogin = useCallback(
    async (event: FormEvent<HTMLFormElement>) => {
      event.preventDefault();
      setBusy(true);
      setError(null);
      try {
        const result = await login(username, password);
        setPassword("");
        applyAuthResult(result);
      } catch (loginError: unknown) {
        setPassword("");
        setError(asAppError(loginError));
      } finally {
        setBusy(false);
      }
    },
    [applyAuthResult, password, username],
  );

  const handlePasswordChange = useCallback(
    async (event: FormEvent<HTMLFormElement>) => {
      event.preventDefault();
      setBusy(true);
      setError(null);
      try {
        const result = await changePassword(
          identity?.mustChangePassword ? undefined : currentPassword,
          newPassword,
        );
        setCurrentPassword("");
        setNewPassword("");
        applyAuthResult(result);
      } catch (changeError: unknown) {
        setNewPassword("");
        setError(asAppError(changeError));
      } finally {
        setBusy(false);
      }
    },
    [applyAuthResult, currentPassword, identity?.mustChangePassword, newPassword],
  );

  const handleLogout = useCallback(async () => {
    setBusy(true);
    setError(null);
    try {
      await logout();
      setIdentity(null);
      setPassword("");
      setViewState("login");
    } catch (logoutError: unknown) {
      setError(asAppError(logoutError));
    } finally {
      setBusy(false);
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
          <button type="button" onClick={() => void handleRetry()} disabled={busy}>
            Reintentar
          </button>
        </section>
      </main>
    );
  }

  if (viewState === "bootstrap") {
    return (
      <main className="app-shell">
        <section className="status-card" aria-labelledby="bootstrap-title">
          <p className="eyebrow">Primer inicio</p>
          <h1 id="bootstrap-title">Administrador inicial creado</h1>
          <p>
            Copia esta contraseña temporal por un medio seguro. Solo se mostrará en esta operación.
          </p>
          <output className="temporary-password" aria-label="Contraseña temporal">
            {temporaryPassword}
          </output>
          <p>
            El usuario es <strong>{identity?.username}</strong>. Deberás cambiar la contraseña
            después de iniciar sesión.
          </p>
          <button
            type="button"
            onClick={() => {
              setTemporaryPassword(null);
              setError(null);
              setViewState("login");
            }}
          >
            Continuar al inicio de sesión
          </button>
        </section>
      </main>
    );
  }

  if (viewState === "login") {
    return (
      <main className="app-shell">
        <section className="status-card" aria-labelledby="login-title">
          <p className="eyebrow">Inventario Remolino del Pez</p>
          <h1 id="login-title">Iniciar sesión</h1>
          {persistenceWarning && (
            <p className="warning" role="status">
              La sesión se mantendrá solo mientras la aplicación esté abierta.
            </p>
          )}
          {error && (
            <p className="form-error" role="alert">
              {error.message}
            </p>
          )}
          <form className="auth-form" onSubmit={(event) => void handleLogin(event)}>
            <label>
              Nombre de usuario
              <input
                autoComplete="username"
                value={username}
                onChange={(event) => setUsername(event.target.value)}
                required
                minLength={3}
                maxLength={32}
                disabled={busy}
              />
            </label>
            <label>
              Contraseña
              <input
                type="password"
                autoComplete="current-password"
                value={password}
                onChange={(event) => setPassword(event.target.value)}
                required
                disabled={busy}
              />
            </label>
            <button type="submit" disabled={busy}>
              {busy ? "Validando…" : "Ingresar"}
            </button>
          </form>
        </section>
      </main>
    );
  }

  if (viewState === "password-change") {
    return (
      <main className="app-shell">
        <section className="status-card" aria-labelledby="password-title">
          <p className="eyebrow">Seguridad de la cuenta</p>
          <h1 id="password-title">Cambia tu contraseña</h1>
          <p>Debes definir una contraseña definitiva antes de acceder al sistema.</p>
          {error && (
            <p className="form-error" role="alert">
              {error.message}
            </p>
          )}
          <form className="auth-form" onSubmit={(event) => void handlePasswordChange(event)}>
            {!identity?.mustChangePassword && (
              <label>
                Contraseña actual
                <input
                  type="password"
                  value={currentPassword}
                  onChange={(event) => setCurrentPassword(event.target.value)}
                  required
                  disabled={busy}
                />
              </label>
            )}
            <label>
              Nueva contraseña
              <input
                type="password"
                value={newPassword}
                onChange={(event) => setNewPassword(event.target.value)}
                minLength={12}
                maxLength={128}
                required
                disabled={busy}
              />
            </label>
            <button type="submit" disabled={busy}>
              {busy ? "Guardando…" : "Guardar contraseña"}
            </button>
          </form>
        </section>
      </main>
    );
  }

  return (
    <main className="app-shell">
      <section className="status-card" aria-labelledby="ready-title">
        <span className="status-mark status-mark--ready" aria-hidden="true">
          ✓
        </span>
        <p className="eyebrow">Inventario Remolino del Pez</p>
        <h1 id="ready-title">Sesión activa</h1>
        <p>
          <strong>{identity?.displayName}</strong> · {identity?.role}
        </p>
        {persistenceWarning && (
          <p className="warning" role="status">
            La sesión no podrá restaurarse al reiniciar.
          </p>
        )}
        {error && (
          <p className="form-error" role="alert">
            {error.message}
          </p>
        )}
        <div className="shell-actions">
          <button
            type="button"
            onClick={() => {
              setError(null);
              setViewState("password-change");
            }}
            disabled={busy}
          >
            Cambiar contraseña
          </button>
          <button type="button" onClick={() => void handleLogout()} disabled={busy}>
            Cerrar sesión
          </button>
        </div>
        <p className="version">Versión {version}</p>
      </section>
    </main>
  );
}

export default App;
