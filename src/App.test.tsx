import { cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import type { AppError } from "./lib/database";
import { getAppStatus, retryDatabase } from "./lib/database";
import App from "./App";

vi.mock("./lib/database", () => ({
  getAppStatus: vi.fn(),
  isAppError: (value: unknown) =>
    Boolean(value) &&
    typeof value === "object" &&
    typeof (value as { code?: unknown }).code === "string" &&
    typeof (value as { message?: unknown }).message === "string",
  retryDatabase: vi.fn(),
}));

const mockedGetAppStatus = vi.mocked(getAppStatus);
const mockedRetryDatabase = vi.mocked(retryDatabase);

const databaseError: AppError = {
  code: "DATABASE_UNAVAILABLE",
  message: "La base de datos no está disponible.",
};

beforeEach(() => {
  vi.clearAllMocks();
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
      expect(screen.getByRole("heading", { name: "Aplicación lista" })).toBeTruthy();
    });
  });

  it("muestra un error seguro y permite reintentar", async () => {
    mockedGetAppStatus.mockRejectedValue(databaseError);
    mockedRetryDatabase.mockResolvedValue({ state: "ready", version: "0.1.0" });

    render(<App />);

    expect(await screen.findByRole("alert")).toBeTruthy();
    expect(screen.getByText(databaseError.message)).toBeTruthy();

    fireEvent.click(screen.getByRole("button", { name: "Reintentar" }));

    await waitFor(() => {
      expect(mockedRetryDatabase).toHaveBeenCalledOnce();
      expect(screen.getByRole("heading", { name: "Aplicación lista" })).toBeTruthy();
    });
  });

  it("muestra la versión cuando el backend está listo", async () => {
    mockedGetAppStatus.mockResolvedValue({ state: "ready", version: "0.1.0" });

    render(<App />);

    expect(await screen.findByText("Versión 0.1.0", { selector: "p" })).toBeTruthy();
  });
});
