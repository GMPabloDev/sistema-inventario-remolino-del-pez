import { invoke } from "@tauri-apps/api/core";

export type AppErrorCode =
  | "APP_DATA_DIR_UNAVAILABLE"
  | "DATABASE_UNAVAILABLE"
  | "DATABASE_MIGRATION_FAILED"
  | "INTERNAL_ERROR"
  | "AUTH_INVALID_CREDENTIALS"
  | "AUTH_SESSION_REQUIRED"
  | "AUTH_SESSION_EXPIRED"
  | "AUTH_FORBIDDEN"
  | "AUTH_PASSWORD_CHANGE_REQUIRED"
  | "AUTH_PERSISTENCE_UNAVAILABLE"
  | "PASSWORD_VALIDATION_FAILED"
  | "USERNAME_ALREADY_EXISTS"
  | "USER_VALIDATION_FAILED"
  | "USER_NOT_FOUND"
  | "LAST_ACTIVE_ADMIN_REQUIRED"
  | "SELF_MANAGEMENT_NOT_ALLOWED";

export interface AppError {
  code: AppErrorCode;
  message: string;
  details?: Record<string, unknown>;
}

export interface AppStatus {
  state: "ready";
  version: string;
}

export type UserRole = "ADMIN" | "WAREHOUSE_MANAGER";

export interface UserIdentity {
  id: string;
  username: string;
  displayName: string;
  role: UserRole;
  mustChangePassword: boolean;
}

export interface AuthStartup {
  state: "bootstrap" | "login" | "authenticated";
  identity: UserIdentity | null;
  temporaryPassword: string | null;
  persistenceWarning: boolean;
}

export interface AuthResult {
  identity: UserIdentity;
  persistenceWarning: boolean;
}

export function isAppError(value: unknown): value is AppError {
  if (!value || typeof value !== "object") {
    return false;
  }

  const candidate = value as Record<string, unknown>;
  return typeof candidate.code === "string" && typeof candidate.message === "string";
}

async function invokeApp<T>(command: string, args?: Record<string, unknown>): Promise<T> {
  try {
    return await invoke<T>(command, args);
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

export function getAuthStartup(): Promise<AuthStartup> {
  return invokeApp<AuthStartup>("get_auth_startup");
}

export function login(username: string, password: string): Promise<AuthResult> {
  return invokeApp<AuthResult>("login", { request: { username, password } });
}

export function changePassword(
  currentPassword: string | undefined,
  newPassword: string,
): Promise<AuthResult> {
  return invokeApp<AuthResult>("change_password", {
    request: { currentPassword, newPassword },
  });
}

export function logout(): Promise<void> {
  return invokeApp<void>("logout");
}

export function getIdentity(): Promise<UserIdentity> {
  return invokeApp<UserIdentity>("get_identity");
}
