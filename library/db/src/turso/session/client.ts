/** Query operations shared by Turso connections and transaction handles. */
export interface QueryClient<Result> {
    /** Prepare a statement on this connection or transaction. */
    prepare(statement: string): Promise<Statement<Result>>;
    /** Execute a statement with bound parameters. */
    run(statement: string, ...parameters: unknown[]): Promise<Result>;
    /** Read rows as named objects. */
    all(statement: string, ...parameters: unknown[]): Promise<unknown[]>;
    /** Read the first row as a named object. */
    get(statement: string, ...parameters: unknown[]): Promise<unknown>;
}

/** A Turso connection that provides dedicated transaction handles. */
export interface ConnectionClient<Result> extends QueryClient<Result> {
    /** Execute a callback with exclusive transaction access. */
    transactionAsync<Value>(operation: (client: QueryClient<Result>) => Promise<Value>): {
        /** Begin a deferred transaction. */
        deferred(): Promise<Value>;
        /** Begin a write transaction. */
        immediate(): Promise<Value>;
        /** Begin an exclusive transaction. */
        exclusive(): Promise<Value>;
    };
}

/** A prepared statement scoped to its creating client. */
export interface Statement<Result> {
    /** Select positional or named result rows. */
    raw(enabled: boolean): Statement<Result>;
    /** Execute the prepared statement. */
    run(...parameters: unknown[]): Promise<Result>;
    /** Read every result row. */
    all(...parameters: unknown[]): Promise<unknown[]>;
    /** Read the first result row. */
    get(...parameters: unknown[]): Promise<unknown>;
}
