import { FormEvent, useCallback, useEffect, useState } from "react";

import {
  type AdminUser,
  type AppError,
  changePassword,
  createUser,
  getAppStatus,
  getAuthStartup,
  isAppError,
  listUsers,
  login,
  logout,
  resetUserPassword,
  retryDatabase,
  setUserActive,
  type UserIdentity,
  type UserRole,
  updateUser,
} from "./lib/database";
import "./App.css";

type ViewState =
  "loading" | "authenticating" | "bootstrap" | "login" | "password-change" | "shell" | "error";

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
  const [usersLoading, setUsersLoading] = useState(false);
  const [showUsers, setShowUsers] = useState(false);
  const [users, setUsers] = useState<AdminUser[]>([]);
  const [editingUser, setEditingUser] = useState<AdminUser | null>(null);
  const [userUsername, setUserUsername] = useState("");
  const [userDisplayName, setUserDisplayName] = useState("");
  const [userRole, setUserRole] = useState<UserRole>("WAREHOUSE_MANAGER");

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
      setViewState("authenticating");
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
      setShowUsers(false);
      setViewState("login");
    } catch (logoutError: unknown) {
      setError(asAppError(logoutError));
    } finally {
      setBusy(false);
    }
  }, []);

  const handleOpenUsers = useCallback(async () => {
    setBusy(true);
    setUsersLoading(true);
    setError(null);
    try {
      setUsers(await listUsers());
      setShowUsers(true);
    } catch (usersError: unknown) {
      setError(asAppError(usersError));
    } finally {
      setUsersLoading(false);
      setBusy(false);
    }
  }, []);

  const handleCreateUser = useCallback(
    async (event: FormEvent<HTMLFormElement>) => {
      event.preventDefault();
      setBusy(true);
      setError(null);
      try {
        const result = await createUser(userUsername, userDisplayName, userRole);
        setUsers((current) =>
          [...current, result.user].sort((left, right) =>
            left.username.localeCompare(right.username),
          ),
        );
        setTemporaryPassword(result.temporaryPassword);
        setUserUsername("");
        setUserDisplayName("");
      } catch (usersError: unknown) {
        setError(asAppError(usersError));
      } finally {
        setBusy(false);
      }
    },
    [userDisplayName, userRole, userUsername],
  );

  const handleUpdateUser = useCallback(
    async (event: FormEvent<HTMLFormElement>) => {
      event.preventDefault();
      if (!editingUser) return;
      setBusy(true);
      setError(null);
      try {
        const result = await updateUser(editingUser.id, userUsername, userDisplayName, userRole);
        setUsers((current) =>
          current.map((user) => (user.id === result.user.id ? result.user : user)),
        );
        setEditingUser(null);
      } catch (usersError: unknown) {
        setError(asAppError(usersError));
      } finally {
        setBusy(false);
      }
    },
    [editingUser, userDisplayName, userRole, userUsername],
  );

  const handleToggleUser = useCallback(async (user: AdminUser) => {
    if (!window.confirm(`${user.active ? "¿Desactivar" : "¿Activar"} a ${user.username}?`)) return;
    setBusy(true);
    setError(null);
    try {
      const result = await setUserActive(user.id, !user.active);
      setUsers((current) =>
        current.map((item) => (item.id === result.user.id ? result.user : item)),
      );
    } catch (usersError: unknown) {
      setError(asAppError(usersError));
    } finally {
      setBusy(false);
    }
  }, []);

  const handleResetUser = useCallback(async (user: AdminUser) => {
    if (!window.confirm(`¿Restablecer la contraseña de ${user.username}?`)) return;
    setBusy(true);
    setError(null);
    try {
      const result = await resetUserPassword(user.id);
      setUsers((current) =>
        current.map((item) => (item.id === result.user.id ? result.user : item)),
      );
      setTemporaryPassword(result.temporaryPassword);
    } catch (usersError: unknown) {
      setError(asAppError(usersError));
    } finally {
      setBusy(false);
    }
  }, []);

  const beginEditing = useCallback((user: AdminUser) => {
    setEditingUser(user);
    setUserUsername(user.username);
    setUserDisplayName(user.displayName);
    setUserRole(user.role);
    setError(null);
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

  if (viewState === "authenticating") {
    return (
      <main className="app-shell" aria-busy="true" aria-live="polite">
        <section className="status-card" aria-labelledby="authenticating-title">
          <span className="status-mark status-mark--loading" aria-hidden="true">
            …
          </span>
          <p className="eyebrow">Inventario Remolino del Pez</p>
          <h1 id="authenticating-title">Comprobando la sesión</h1>
          <p>Estamos verificando el acceso local. Espera un momento.</p>
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

  if (showUsers && identity?.role === "ADMIN") {
    return (
      <main className="app-shell">
        <section className="status-card user-management" aria-labelledby="users-title">
          <p className="eyebrow">Administración</p>
          <h1 id="users-title">Usuarios</h1>
          {error && (
            <p className="form-error" role="alert">
              {error.message}
            </p>
          )}
          {temporaryPassword && (
            <div className="temporary-delivery" role="status">
              <p>Contraseña temporal. Entrégala ahora; no volverá a mostrarse.</p>
              <output className="temporary-password" aria-label="Contraseña temporal">
                {temporaryPassword}
              </output>
              <button type="button" onClick={() => setTemporaryPassword(null)}>
                Ocultar contraseña
              </button>
            </div>
          )}
          <form
            className="auth-form"
            onSubmit={(event) =>
              void (editingUser ? handleUpdateUser(event) : handleCreateUser(event))
            }
          >
            <h2>{editingUser ? "Editar usuario" : "Crear usuario"}</h2>
            <label>
              Nombre de usuario
              <input
                value={userUsername}
                onChange={(event) => setUserUsername(event.target.value)}
                minLength={3}
                maxLength={32}
                required
                disabled={busy}
              />
            </label>
            <label>
              Nombre visible
              <input
                value={userDisplayName}
                onChange={(event) => setUserDisplayName(event.target.value)}
                maxLength={100}
                required
                disabled={busy}
              />
            </label>
            <label>
              Rol
              <select
                value={userRole}
                onChange={(event) => setUserRole(event.target.value as UserRole)}
                disabled={busy}
              >
                <option value="ADMIN">ADMIN</option>
                <option value="WAREHOUSE_MANAGER">WAREHOUSE_MANAGER</option>
              </select>
            </label>
            <div className="shell-actions">
              <button type="submit" disabled={busy}>
                {editingUser ? "Guardar cambios" : "Crear usuario"}
              </button>
              {editingUser && (
                <button type="button" onClick={() => setEditingUser(null)} disabled={busy}>
                  Cancelar
                </button>
              )}
            </div>
          </form>
          <div className="user-list" aria-live="polite">
            {usersLoading ? (
              <p role="status">Cargando usuarios…</p>
            ) : users.length === 0 ? (
              <p>No hay usuarios configurados.</p>
            ) : (
              users.map((user) => (
                <article className="user-row" key={user.id}>
                  <div>
                    <strong>{user.username}</strong>
                    <span>
                      {user.displayName} · {user.role} · {user.active ? "Activa" : "Inactiva"}
                    </span>
                  </div>
                  <div className="shell-actions">
                    <button type="button" onClick={() => beginEditing(user)} disabled={busy}>
                      Editar
                    </button>
                    <button
                      type="button"
                      onClick={() => void handleToggleUser(user)}
                      disabled={busy}
                    >
                      {user.active ? "Desactivar" : "Activar"}
                    </button>
                    <button
                      type="button"
                      onClick={() => void handleResetUser(user)}
                      disabled={busy}
                    >
                      Restablecer contraseña
                    </button>
                  </div>
                </article>
              ))
            )}
          </div>
          <button
            type="button"
            onClick={() => {
              setShowUsers(false);
              setEditingUser(null);
              setTemporaryPassword(null);
            }}
            disabled={busy}
          >
            Volver
          </button>
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
          {identity?.role === "ADMIN" && (
            <button type="button" onClick={() => void handleOpenUsers()} disabled={busy}>
              Gestionar usuarios
            </button>
          )}
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
