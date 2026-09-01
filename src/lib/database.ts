import Database from "@tauri-apps/plugin-sql";

const databasePath = "sqlite:inventory.db";

let databasePromise: Promise<Database> | undefined;

export function getDatabase(): Promise<Database> {
  databasePromise ??= Database.load(databasePath);
  return databasePromise;
}

export async function testDatabaseConnection(): Promise<void> {
  const database = await getDatabase();
  await database.select<{ value: number }>("SELECT 1 AS value");
}
