import type { DriverValue } from "../table/column.ts";

/** The statements of a SQLite connection or one of its transactions. */
export interface QueryClient {
    /** Read every row as an array of values, integers exact where the engine keeps them. */
    values(sql: string, parameters: readonly DriverValue[]): Promise<unknown[][]>;
    /** Read every row by column name. */
    all(sql: string, parameters: readonly DriverValue[]): Promise<unknown[]>;
    /** Run a statement for its effect. */
    run(sql: string, parameters: readonly DriverValue[]): Promise<void>;
    /** Run a script of statements. */
    exec(script: string): Promise<void>;
    /** Run work in a nested transaction at a depth, undoing only its writes when it fails. */
    nest<Value>(depth: number, operation: (client: QueryClient) => Promise<Value>): Promise<Value>;
}

/** A SQLite connection with dedicated transactions. */
export interface ConnectionClient extends QueryClient {
    /** Close the physical connection. */
    close(): Promise<void>;
    /** Run a callback with exclusive transaction access. */
    transactionAsync<Value>(operation: (client: QueryClient) => Promise<Value>): {
        /** Begin a deferred transaction. */
        deferred(): Promise<Value>;
        /** Begin a write transaction. */
        immediate(): Promise<Value>;
        /** Begin an exclusive transaction. */
        exclusive(): Promise<Value>;
    };
}

/** Work run one piece at a time, in arrival order. */
export class WorkQueue {
    /** The tail of the waiting work. */
    #tail: Promise<unknown> = Promise.resolve();

    /** Run work after the earlier work. */
    run<Value>(work: () => Promise<Value>): Promise<Value> {
        const result = this.#tail.then(work);
        this.#tail = result.catch(() => undefined);

        return result;
    }
}

/** Nested transactions as SQL savepoints, for clients that accept savepoint statements. */
export const Savepoint = {
    /** Run work in a savepoint at a depth, rolling back to it when the work fails. */
    async run<Value>(
        client: QueryClient,
        depth: number,
        operation: (client: QueryClient) => Promise<Value>,
    ): Promise<Value> {
        const name = `"destack_savepoint_${depth}"`;
        await client.run(`SAVEPOINT ${name}`, []);
        try {
            const result = await operation(client);
            await client.run(`RELEASE SAVEPOINT ${name}`, []);

            return result;
        } catch (error) {
            // keep both failures when the savepoint cannot be restored
            try {
                await client.run(`ROLLBACK TO SAVEPOINT ${name}`, []);
                await client.run(`RELEASE SAVEPOINT ${name}`, []);
            } catch (rollback) {
                throw new AggregateError(
                    [error, rollback],
                    "SQLite savepoint and rollback failed",
                    {
                        cause: rollback,
                    },
                );
            }

            throw error;
        }
    },
};
