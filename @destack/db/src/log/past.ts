import { drizzle } from "drizzle-orm/postgres-js";
import type postgres from "postgres";
import type { NativeDatabase } from "../database/driver.ts";
import { DatabaseError } from "../error/error.ts";
import type { ConnectionClient, QueryClient } from "../sqlite/client.ts";
import { SqliteNative } from "../sqlite/native.ts";
import type { History } from "./history.ts";

/** Open native queries that read a database's logged tables as a history shows them, refusing writes. */
export function pastNative(native: NativeDatabase, history: History): NativeDatabase {
    // read through a SQLite client rewriting every statement
    if (native.dialect === "sqlite") {
        const client = (native.database as SqliteNative).$client;

        return { dialect: "sqlite", database: new SqliteNative(pastSqlite(client, history)) };
    }

    // read through a PostgreSQL client rewriting every statement
    const client = (native.database as unknown as { $client: postgres.Sql }).$client;

    return { dialect: "postgresql", database: drizzle({ client: pastPostgres(client, history) }) };
}

/** Wrap a SQLite connection so that every statement reads as a history shows it. */
function pastSqlite(
    client: ConnectionClient<unknown>,
    history: History,
): ConnectionClient<unknown> {
    return {
        ...reads(client, history),
        close: async () => {},
        transactionAsync: (operation) =>
            client.transactionAsync((inner) => operation(reads(inner, history))),
    };
}

/** Wrap a SQLite query client so that it rewrites every statement, refusing scripts. */
function reads(client: QueryClient<unknown>, history: History): QueryClient<unknown> {
    return {
        prepare: (statement) => client.prepare(history.rewrite(statement)),
        run: (statement, ...parameters) => client.run(history.rewrite(statement), ...parameters),
        all: (statement, ...parameters) => client.all(history.rewrite(statement), ...parameters),
        get: (statement, ...parameters) => client.get(history.rewrite(statement), ...parameters),
        exec: () =>
            Promise.reject(
                new DatabaseError("READ_ONLY", "a database as of a position only reads"),
            ),
    };
}

/** Wrap a PostgreSQL client so that every statement, in transactions too, reads as a history shows it. */
function pastPostgres(client: postgres.Sql, history: History): postgres.Sql {
    return new Proxy(client, {
        get(target, property, receiver) {
            // rewrite each statement the client runs
            if (property === "unsafe") {
                return (statement: string, ...rest: unknown[]) =>
                    (target.unsafe as (...values: unknown[]) => unknown)(
                        history.rewrite(statement),
                        ...rest,
                    );
            }
            // wrap each transaction's client
            else if (property === "begin") {
                return (...values: unknown[]) => {
                    const operation = values.pop() as (inner: postgres.Sql) => unknown;

                    return (target.begin as (...values: unknown[]) => unknown)(
                        ...values,
                        (inner: postgres.Sql) => operation(pastPostgres(inner, history)),
                    );
                };
            }

            return Reflect.get(target, property, receiver);
        },
    });
}
