import { cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import type { AppError } from "./lib/database";
import { getAppStatus, getAuthStartup, listUsers, retryDatabase } from "./lib/database";
import App from "./App";

vi.mock("./lib/database", () => ({
  getAppStatus: vi.fn(),
  isAppError: (value: unknown) =>
    Boolean(value) &&
    typeof value === "object" &&
    typeof (value as { code?: unknown }).code === "string" &&
    typeof (value as { message?: unknown }).message === "string",
  retryDatabase: vi.fn(),
  getAuthStartup: vi.fn(),
  listUsers: vi.fn(),
}));

const mockedGetAppStatus = vi.mocked(getAppStatus);
const mockedRetryDatabase = vi.mocked(retryDatabase);
const mockedGetAuthStartup = vi.mocked(getAuthStartup);
const mockedListUsers = vi.mocked(listUsers);

const databaseError: AppError = {
  code: "DATABASE_UNAVAILABLE",
  message: "La base de datos no está disponible.",
};

beforeEach(() => {
  vi.clearAllMocks();
  mockedGetAuthStartup.mockResolvedValue({
    state: "login",
    identity: null,
    temporaryPassword: null,
    persistenceWarning: false,
  });
});

afterEach(() => {
  cleanup();
});

describe("App shell", () => {
  it("muestra el estado de carga mientras consulta el backend", async () => {
    let resolveStatus: (status: { state: "ready"; version: string }) => void = () => undefined;
    mockedGetAppStatus.mockReturnValue(
      new Promise((resolve) => {
        resolveStatus = resolve;
      }),
    );

    render(<App />);

    expect(screen.getByRole("heading", { name: "Preparando la aplicación" })).toBeTruthy();

    resolveStatus({ state: "ready", version: "0.1.0" });
    await waitFor(() => {
      expect(screen.getByRole("heading", { name: "Iniciar sesión" })).toBeTruthy();
    });
  });

  it("muestra un error seguro y permite reintentar", async () => {
    mockedGetAppStatus.mockRejectedValueOnce(databaseError).mockResolvedValue({
      state: "ready",
      version: "0.1.0",
    });
    mockedRetryDatabase.mockResolvedValue({ state: "ready", version: "0.1.0" });
    mockedGetAuthStartup.mockResolvedValue({
      state: "login",
      identity: null,
      temporaryPassword: null,
      persistenceWarning: false,
    });

    render(<App />);

    expect(await screen.findByRole("alert")).toBeTruthy();
    expect(screen.getByText(databaseError.message)).toBeTruthy();

    fireEvent.click(screen.getByRole("button", { name: "Reintentar" }));

    await waitFor(() => {
      expect(mockedRetryDatabase).toHaveBeenCalledOnce();
      expect(screen.getByRole("heading", { name: "Iniciar sesión" })).toBeTruthy();
    });
  });

  it("oculta la administración para WAREHOUSE_MANAGER", async () => {
    mockedGetAppStatus.mockResolvedValue({ state: "ready", version: "0.1.0" });
    mockedGetAuthStartup.mockResolvedValue({
      state: "authenticated",
      identity: {
        id: "manager-id",
        username: "manager",
        displayName: "Encargado",
        role: "WAREHOUSE_MANAGER",
        mustChangePassword: false,
      },
      temporaryPassword: null,
      persistenceWarning: false,
    });

    render(<App />);

    expect(await screen.findByRole("heading", { name: "Sesión activa" })).toBeTruthy();
    expect(screen.queryByRole("button", { name: "Gestionar usuarios" })).toBeNull();
  });

  it("permite a ADMIN abrir la gestión de usuarios", async () => {
    mockedGetAppStatus.mockResolvedValue({ state: "ready", version: "0.1.0" });
    mockedGetAuthStartup.mockResolvedValue({
      state: "authenticated",
      identity: {
        id: "admin-id",
        username: "admin",
        displayName: "Administrador",
        role: "ADMIN",
        mustChangePassword: false,
      },
      temporaryPassword: null,
      persistenceWarning: false,
    });
    mockedListUsers.mockResolvedValue([]);

    render(<App />);
    fireEvent.click(await screen.findByRole("button", { name: "Gestionar usuarios" }));

    expect(await screen.findByRole("heading", { name: "Usuarios" })).toBeTruthy();
    expect(mockedListUsers).toHaveBeenCalledOnce();
    expect(screen.getByText("No hay usuarios configurados.")).toBeTruthy();
  });

  it("muestra la versión cuando el backend está listo", async () => {
    mockedGetAppStatus.mockResolvedValue({ state: "ready", version: "0.1.0" });
    mockedGetAuthStartup.mockResolvedValue({
      state: "authenticated",
      identity: {
        id: "user-id",
        username: "admin",
        displayName: "Administrador",
        role: "ADMIN",
        mustChangePassword: false,
      },
      temporaryPassword: null,
      persistenceWarning: false,
    });

    render(<App />);

    expect(await screen.findByText("Versión 0.1.0", { selector: "p" })).toBeTruthy();
  });
});
