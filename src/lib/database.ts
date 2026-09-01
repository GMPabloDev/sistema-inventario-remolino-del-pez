import { invoke } from "@tauri-apps/api/core";

export type AppErrorCode =
  | "APP_DATA_DIR_UNAVAILABLE"
  | "DATABASE_UNAVAILABLE"
  | "DATABASE_MIGRATION_FAILED"
  | "INTERNAL_ERROR";

export interface AppError {
  code: AppErrorCode;
  message: string;
  details?: Record<string, unknown>;
}

export interface AppStatus {
  state: "ready";
  version: string;
}

export function isAppError(value: unknown): value is AppError {
  if (!value || typeof value !== "object") {
    return false;
  }

  const candidate = value as Record<string, unknown>;
  return typeof candidate.code === "string" && typeof candidate.message === "string";
}

async function invokeApp<T>(command: string): Promise<T> {
  try {
    return await invoke<T>(command);
  } catch (error: unknown) {
    if (isAppError(error)) {
      throw error;
    }

    throw {
      code: "INTERNAL_ERROR",
      message: "Ocurrió un error interno. Inténtalo nuevamente.",
    } satisfies AppError;
  }
}

export function getAppStatus(): Promise<AppStatus> {
  return invokeApp<AppStatus>("get_app_status");
}

export function retryDatabase(): Promise<AppStatus> {
  return invokeApp<AppStatus>("retry_database");
}

export function testDatabaseConnection(): Promise<void> {
  return invokeApp<void>("test_database_connection");
}
