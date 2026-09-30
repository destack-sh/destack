/** The query operations of SQLite connections and transactions. */
export interface QueryClient<Result> {
    /** Prepare a statement on this connection or transaction. */
    prepare(statement: string): Promise<Statement<Result>>;
    /** Execute a statement with bound parameters. */
    run(statement: string, ...parameters: unknown[]): Promise<Result>;
    /** Read rows as named objects. */
    all(statement: string, ...parameters: unknown[]): Promise<unknown[]>;
    /** Read the first row as a named object. */
    get(statement: string, ...parameters: unknown[]): Promise<unknown>;
    /** Execute a script of statements without returning rows. */
    exec(script: string): Promise<unknown>;
    /** Run work in a nested transaction at a depth, undoing only its writes when it fails. */
    nest<Value>(
        depth: number,
        operation: (client: QueryClient<Result>) => Promise<Value>,
    ): Promise<Value>;
}

/** A SQLite connection with dedicated transactions. */
export interface ConnectionClient<Result> extends QueryClient<Result> {
    /** Close the physical connection. */
    close(): Promise<void>;
    /** Run a callback with exclusive transaction access. */
    transactionAsync<Value>(operation: (client: QueryClient<Result>) => Promise<Value>): {
        /** Begin a deferred transaction. */
        deferred(): Promise<Value>;
        /** Begin a write transaction. */
        immediate(): Promise<Value>;
        /** Begin an exclusive transaction. */
        exclusive(): Promise<Value>;
    };
}

/** A prepared statement of its creating client. */
export interface Statement<Result> {
    /** Read integers exactly. */
    safeIntegers(enabled: boolean): Statement<Result>;
    /** Select positional or named rows. */
    raw(enabled: boolean): Statement<Result>;
    /** Execute the prepared statement. */
    run(...parameters: unknown[]): Promise<Result>;
    /** Read every result row. */
    all(...parameters: unknown[]): Promise<unknown[]>;
    /** Read the first result row. */
    get(...parameters: unknown[]): Promise<unknown>;
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
    async run<Result, Value>(
        client: QueryClient<Result>,
        depth: number,
        operation: (client: QueryClient<Result>) => Promise<Value>,
    ): Promise<Value> {
        const name = `"destack_savepoint_${depth}"`;
        await client.run(`SAVEPOINT ${name}`);
        try {
            const result = await operation(client);
            await client.run(`RELEASE SAVEPOINT ${name}`);

            return result;
        } catch (error) {
            // keep both failures when the savepoint cannot be restored
            try {
                await client.run(`ROLLBACK TO SAVEPOINT ${name}`);
                await client.run(`RELEASE SAVEPOINT ${name}`);
            } catch (rollback) {
                throw new AggregateError([error, rollback], "SQLite savepoint and rollback failed");
            }

            throw error;
        }
    },
};
