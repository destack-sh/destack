import { SQL } from "bun";

// pool management
const pools = new Map<string, SQL>();

/** Get a SQL connection for a given database. */
export async function getPostgres(database: { url: string; tls?: boolean }): Promise<SQL> {
  const pool = pools.get(database.url);
  if (pool) {
    return pool;
  }
  const newPool = new SQL(database.url, {
    tls: database.tls,
  });
  pools.set(database.url, newPool);
  return newPool;
}

/** Close a SQL connection for a given database. */
export async function closePostgresPool(database: { url: string; tls?: boolean }): Promise<void> {
  const pool = pools.get(database.url);
  if (pool) {
    await pool.close();
    pools.delete(database.url);
  }
}
