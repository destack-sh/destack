import {
    SQLiteAsyncPreparedQuery,
    SQLiteAsyncSession,
    SQLiteDialect,
} from "drizzle-orm/sqlite-core";
import type { SQLiteAsyncPreparedQueryConfig } from "drizzle-orm/sqlite-core";
import type { AnyRelations } from "drizzle-orm/relations";
import { DefaultLogger, type Logger, NoopLogger } from "drizzle-orm/logger";
import type { DrizzleSQLiteConfig } from "drizzle-orm/sqlite-core/utils";
import type { QueryClient, Statement } from "./client.ts";

/** Execute Drizzle queries through a SQLite connection or transaction handle. */
export abstract class Session<Result, Relations extends AnyRelations>
    extends SQLiteAsyncSession<"async", Result, Relations> {
    /** The connection or transaction that executes statements. */
    readonly client: QueryClient<Result>;
    /** Query logging, cache, and relation options. */
    readonly options: DrizzleSQLiteConfig<Relations>;
    /** The query logger shared by this session's statements. */
    readonly logger: Logger;

    /** Retain the query client and compiler options. */
    constructor(client: QueryClient<Result>, options: DrizzleSQLiteConfig<Relations>) {
        super(new SQLiteDialect({ useJitMappers: options.jit ?? false }), "async");
        this.client = client;
        this.options = options;
        this.logger = options.logger === true
            ? new DefaultLogger()
            : options.logger || new NoopLogger();
    }

    /** Compile query execution while retaining Drizzle's result mapping. */
    prepareQuery(
        ...arguments_: Parameters<SQLiteAsyncSession<"async", Result, Relations>["prepareQuery"]>
    ) {
        const [query, mode, prepare, method, mapper, metadata, cache] = arguments_;
        const client = this.client;
        let statement: Promise<Statement<Result>> | undefined;

        // retain prepared statements only when requested by the query builder
        return new SQLiteAsyncPreparedQuery<
            SQLiteAsyncPreparedQueryConfig & { type: "async"; run: Result }
        >(
            "async",
            method,
            {
                run: async (parameters) => {
                    if (!prepare) return client.run(query.sql, ...parameters);
                    statement ??= client.prepare(query.sql);
                    return (await statement).run(...parameters);
                },
                all: async (parameters) => {
                    if (!prepare && mode !== "arrays") return client.all(query.sql, ...parameters);
                    statement ??= client.prepare(query.sql);
                    return (await statement).safeIntegers(mode === "arrays").raw(mode === "arrays")
                        .all(...parameters);
                },
                get: async (parameters) => {
                    if (!prepare && mode !== "arrays") return client.get(query.sql, ...parameters);
                    statement ??= client.prepare(query.sql);
                    return (await statement).safeIntegers(mode === "arrays").raw(mode === "arrays")
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
