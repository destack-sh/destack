import { sql } from "drizzle-orm";
import {
    SQLiteAsyncPreparedQuery,
    SQLiteAsyncSession,
    SQLiteAsyncTransaction,
    SQLiteDialect,
} from "drizzle-orm/sqlite-core";
import type {
    SQLiteAsyncPreparedQueryConfig,
    SQLiteTransactionConfig,
} from "drizzle-orm/sqlite-core";
import type { AnyRelations } from "drizzle-orm/relations";
import { DefaultLogger, type Logger, NoopLogger } from "drizzle-orm/logger";
import type { DrizzleSQLiteConfig } from "drizzle-orm/sqlite-core/utils";
import type { ConnectionClient, QueryClient, Statement } from "./client.ts";

/** Run Drizzle queries through a SQLite connection or transaction. */
export abstract class Session<Result, Relations extends AnyRelations> extends SQLiteAsyncSession<
    "async",
    Result,
    Relations
> {
    /** The executing connection or transaction. */
    readonly client: QueryClient<Result>;
    /** The Drizzle options. */
    readonly options: DrizzleSQLiteConfig<Relations>;
    /** The query logger. */
    readonly logger: Logger;

    /** Create the session. */
    constructor(client: QueryClient<Result>, options: DrizzleSQLiteConfig<Relations>) {
        // configure the dialect, client and logger
        super(new SQLiteDialect({ useJitMappers: options.jit ?? false }), "async");
        this.client = client;
        this.options = options;
        this.logger =
            options.logger === true ? new DefaultLogger() : options.logger || new NoopLogger();
    }

    /** Execute a script in one call. */
    async exec(script: string): Promise<void> {
        await this.client.exec(script);
    }

    /** Prepare a query with Drizzle's result mapping. */
    prepareQuery(
        ...arguments_: Parameters<SQLiteAsyncSession<"async", Result, Relations>["prepareQuery"]>
    ) {
        // read the query arguments
        const [query, mode, prepare, method, mapper, metadata, cache] = arguments_;
        const client = this.client;
        let statement: Promise<Statement<Result>> | undefined;

        // keep prepared statements only when asked
        return new SQLiteAsyncPreparedQuery<
            SQLiteAsyncPreparedQueryConfig & { type: "async"; run: Result }
        >(
            "async",
            method,
            {
                run: async (parameters) => {
                    if (!prepare) {
                        return client.run(query.sql, ...parameters);
                    }
                    statement ??= client.prepare(query.sql);

                    return (await statement).run(...parameters);
                },
                all: async (parameters) => {
                    if (!prepare && mode !== "arrays") {
                        return client.all(query.sql, ...parameters);
                    }
                    statement ??= client.prepare(query.sql);

                    return (await statement)
                        .safeIntegers(mode === "arrays")
                        .raw(mode === "arrays")
                        .all(...parameters);
                },
                get: async (parameters) => {
                    if (!prepare && mode !== "arrays") {
                        return client.get(query.sql, ...parameters);
                    }
                    statement ??= client.prepare(query.sql);

                    return (await statement)
                        .safeIntegers(mode === "arrays")
                        .raw(mode === "arrays")
                        .get(...parameters);
                },
                values: async (parameters) => {
                    statement ??= client.prepare(query.sql);

                    return (await statement).raw(true).all(...parameters);
                },
            },
            query,
            mapper,
            mode,
            this.logger,
            this.options.cache,
            metadata,
            cache,
        );
    }
}

/** A session that starts SQLite transactions. */
export class ConnectionSession<Result, Relations extends AnyRelations> extends Session<
    Result,
    Relations
> {
    /** The physical connection. */
    readonly connection: ConnectionClient<Result>;
    /** The relations of queries and transactions. */
    readonly relations: Relations;

    /** Create the session. */
    constructor(
        client: ConnectionClient<Result>,
        relations: Relations,
        options: DrizzleSQLiteConfig<Relations>,
    ) {
        super(client, options);
        this.connection = client;
        this.relations = relations;
    }

    /** Commit or roll back a callback in a transaction. */
    transaction<Value>(
        operation: (transaction: Transaction<Result, Relations>) => Promise<Value>,
        configuration?: SQLiteTransactionConfig,
    ): Promise<Value> {
        const transaction = this.connection.transactionAsync(async (client) => {
            const session = new TransactionSession(client, this.relations, this.options, 0);

            return await operation(
                new Transaction("async", session.dialect, session, this.relations),
            );
        });

        return transaction[configuration?.behavior ?? "deferred"]();
    }
}

/** Drizzle queries in one transaction or savepoint. */
export class Transaction<Result, Relations extends AnyRelations> extends SQLiteAsyncTransaction<
    "async",
    Result,
    Relations
> {}

/** Queries and savepoints in one SQLite transaction. */
export class TransactionSession<Result, Relations extends AnyRelations> extends Session<
    Result,
    Relations
> {
    /** The relations of nested transactions. */
    readonly relations: Relations;
    /** The current savepoint depth. */
    readonly depth: number;

    /** Create the session. */
    constructor(
        client: QueryClient<Result>,
        relations: Relations,
        options: DrizzleSQLiteConfig<Relations>,
        depth: number,
    ) {
        super(client, options);
        this.relations = relations;
        this.depth = depth;
    }

    /** Run a nested transaction in a savepoint. */
    async transaction<Value>(
        operation: (transaction: Transaction<Result, Relations>) => Promise<Value>,
    ): Promise<Value> {
        const name = sql.identifier(`destack_savepoint_${this.depth}`);
        await this.run(sql`SAVEPOINT ${name}`);
        try {
            const session = new TransactionSession(
                this.client,
                this.relations,
                this.options,
                this.depth + 1,
            );
            const result = await operation(
                new Transaction("async", session.dialect, session, this.relations),
            );
            await this.run(sql`RELEASE SAVEPOINT ${name}`);

            return result;
        } catch (error) {
            // keep both failures when the savepoint cannot be restored
            try {
                await this.run(sql`ROLLBACK TO SAVEPOINT ${name}`);
                await this.run(sql`RELEASE SAVEPOINT ${name}`);
            } catch (rollback) {
                throw new AggregateError([error, rollback], "SQLite savepoint and rollback failed");
            }

            throw error;
        }
    }
}
