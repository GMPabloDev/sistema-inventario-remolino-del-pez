import { invoke } from "@tauri-apps/api/core";

export function testDatabaseConnection(): Promise<void> {
  return invoke("test_database_connection");
}
