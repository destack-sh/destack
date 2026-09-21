import { assertNever } from "../error/error.ts";
import { type Query, type SQL, type SQLWrapper } from "drizzle-orm";
import { PgAsyncDatabase, type PgDialect } from "drizzle-orm/pg-core";
import { SQLiteAsyncDatabase, type SQLiteDialect } from "drizzle-orm/sqlite-core";
import type {
    BuildQueryResult,
    DBQueryConfig,
    TableRelationalConfig,
    TablesRelationalConfig,
} from "drizzle-orm/relations";
import { type DatabaseDriver } from "../database/driver.ts";
import type { SchemaCompiler } from "../dialect/compiler.ts";
import type { NativeRelations } from "../dialect/table.ts";
import type { TableRelations } from "../schema/relation.ts";
import type { Dialect } from "../dialect/dialect.ts";
import type { PreparedQuery } from "./select.ts";

/** Typed relational queries indexed by the schema's table properties. */
export type RelationalQueries<Relations extends Record<string, TableRelations>> = {
    [Name in keyof Relations]: RelationalQueryBuilder<
        NativeRelations<Dialect, Relations>,
        NativeRelations<Dialect, Relations>[Name]
    >;
};

/** Drizzle relational queries with the connection's transaction lifetime. */
export class RelationalQueryBuilder<
    Relations extends TablesRelationalConfig,
    Table extends TableRelationalConfig,
> {
    /** The selected native relational query builder. */
    readonly native: NativeRelationalBuilder;
    /** The connection or transaction that executes the query. */
    readonly connection: DatabaseDriver;

    /** Retain native query generation and transaction state. */
    constructor(native: NativeRelationalBuilder, connection: DatabaseDriver) {
        this.native = native;
        this.connection = connection;
    }

    /** Select related records using Drizzle's relational configuration. */
    findMany<Config extends DBQueryConfig<"many", Relations, Table> = {}>(
        configuration?: Config,
    ): RelationalQuery<BuildQueryResult<Relations, Table, Config>[]> {
        return new RelationalQuery(this.native.findMany(configuration), this.connection);
    }

    /** Select the first related record, if present. */
    findFirst<Config extends DBQueryConfig<"one", Relations, Table> = {}>(
        configuration?: Config,
    ): RelationalQuery<BuildQueryResult<Relations, Table, Config> | undefined> {
        return new RelationalQuery(this.native.findFirst(configuration), this.connection);
    }
}

/** An executable relational query using native SQL and result decoding. */
export class RelationalQuery<Result> implements PromiseLike<Result>, SQLWrapper {
    /** The native query and result mapper. */
    readonly native: NativeRelationalQuery;
    /** The connection or transaction that executes the query. */
    readonly connection: DatabaseDriver;

    /** Retain the native query and transaction state. */
    constructor(native: NativeRelationalQuery, connection: DatabaseDriver) {
        this.native = native;
        this.connection = connection;
    }

    /** Execute within the active transaction. */
    async execute(parameters?: Record<string, unknown>): Promise<Result> {
        this.connection.transaction?.assertActive();

        return (await this.connection.run(() => this.native.execute(parameters))) as Result;
    }

    /** Prepare a query without extending its transaction lifetime. */
    prepare(): PreparedQuery<Result> {
        const prepared = this.native.prepare();

        return {
            execute: async (parameters) => {
                this.connection.transaction?.assertActive();

                return (await this.connection.run(() => prepared.execute(parameters))) as Result;
            },
        };
    }

    /** Describe the generated SQL and parameters. */
    toSQL(): Query {
        return this.native.toSQL();
    }

    /** Embed the native relational query in an SQL expression. */
    getSQL(): SQL {
        return this.native.getSQL();
    }

    /** Await the native relational query. */
    then<Fulfilled = Result, Rejected = never>(
        fulfilled?: ((value: Result) => Fulfilled | PromiseLike<Fulfilled>) | null,
        rejected?: ((reason: unknown) => Rejected | PromiseLike<Rejected>) | null,
    ): PromiseLike<Fulfilled | Rejected> {
        return this.execute().then(fulfilled, rejected);
    }
}

/** The shared executable operations on Drizzle's dialect-specific queries. */
interface NativeRelationalQuery extends SQLWrapper {
    /** Execute the query with optional placeholders. */
    execute(parameters?: Record<string, unknown>): Promise<unknown>;
    /** Prepare the SQL and result mapper. */
    prepare(): { execute(parameters?: Record<string, unknown>): Promise<unknown> };
    /** Describe the generated SQL. */
    toSQL(): Query;
}

/** Native operations selected after checking the portable configuration type. */
interface NativeRelationalBuilder {
    /** Construct a multiple-row query. */
    findMany(configuration?: unknown): NativeRelationalQuery;
    /** Construct a single-row query. */
    findFirst(configuration?: unknown): NativeRelationalQuery;
}

/** Bind schema relations to the existing physical session. */
export function bindRelations<Relations extends Record<string, TableRelations>>(
    connection: DatabaseDriver,
    schema: SchemaCompiler,
    definitions: Relations,
): RelationalQueries<Relations> {
    // reuse the active session without allocating another client or connection
    const relations = schema.relations(definitions);
    let database;
    if (connection.native.dialect === "sqlite") {
        const native = connection.native.database as typeof connection.native.database & {
            dialect: SQLiteDialect;
        };
        database = new SQLiteAsyncDatabase(
            "async",
            native.dialect,
            connection.native.database._.session,
            relations,
            connection.native.database.forbidJsonb,
        );
    } else if (connection.native.dialect === "postgresql") {
        const native = connection.native.database as typeof connection.native.database & {
            dialect: PgDialect;
        };
        database = new PgAsyncDatabase(
            native.dialect,
            connection.native.database._.session,
            relations,
            false,
        );
    } else {
        return assertNever(connection.native);
    }

    // expose native builders through the shared lifetime checks
    const queries = database.query as unknown as Record<string, NativeRelationalBuilder>;

    return Object.fromEntries(
        Object.entries(queries).map(([name, query]) => [
            name,
            new RelationalQueryBuilder(query, connection),
        ]),
    ) as RelationalQueries<Relations>;
}
