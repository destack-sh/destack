import type { ConnectionClient } from "./session/client.ts";
import { SQLiteAsyncDatabase } from "drizzle-orm/sqlite-core";
import type { DrizzleSQLiteConfig } from "drizzle-orm/sqlite-core/utils";
import type { AnyRelations, EmptyRelations } from "drizzle-orm/relations";
import { ConnectionSession } from "./session/connection.ts";
import { ConnectionState, DatabaseConnection } from "../database/connection.ts";
import { DatabaseDriver } from "../database/driver.ts";
import { SQLiteSchemaCompiler } from "./compiler.ts";
import type { NativeRelations } from "../dialect/table.ts";
import type { TableRelations } from "../schema/relation.ts";
import { type DatabaseSchema, orderSchemas } from "../schema/schema.ts";
import type { Table } from "../table/table.ts";
import type { SQL } from "drizzle-orm";

/** Portable queries with an owned Turso connection. */
export class Database<
    Relations extends Record<string, TableRelations> = {},
    Client extends ConnectionClient<unknown> = ConnectionClient<unknown>,
> extends DatabaseConnection<"sqlite", Relations> {
    /** The underlying Turso connection. */
    readonly $client: Client;
    /** The explicit native SQL API. */
    readonly native: Connection<NativeRelations<"sqlite", Relations>, Client>;

    /** Bind logical tables to a Turso database. */
    constructor(
        client: Client,
        schema: DatabaseSchema<Record<string, Table>, Relations> | readonly Table[],
        options: Omit<DrizzleSQLiteConfig<EmptyRelations>, "relations"> = {},
    ) {
        // order the schema tables before compiling them
        const definition = Array.isArray(schema)
            ? undefined
            : (schema as DatabaseSchema<Record<string, Table>, Relations>);
        const tables = definition
            ? orderSchemas([definition]).flatMap((schema) => Object.values(schema.tables))
            : (schema as readonly Table[]);
        const logical = definition?.relations ?? ({} as Relations);
        const declarations = new SQLiteSchemaCompiler(tables);
        const relations = declarations.relations(logical);
        const native = new Connection(client, { ...options, relations });
        super(
            new DatabaseDriver({ dialect: "sqlite", database: native }, new ConnectionState()),
            declarations,
            logical,
        );
        this.$client = client;
        this.native = native;
    }

    /** Execute an explicit SQL statement. */
    async run(statement: SQL): Promise<RunResult<Client>> {
        return await this.state.run(() => this.native.run(this.schema.expression(statement)));
    }

    /** Close the physical database. */
    close(): Promise<void> {
        return this.state.close(() => this.$client.close());
    }
}

/** Native Drizzle queries over a Turso connection. */
export class Connection<
    Relations extends AnyRelations = EmptyRelations,
    Client extends ConnectionClient<unknown> = ConnectionClient<unknown>,
> extends SQLiteAsyncDatabase<"async", RunResult<Client>, Relations> {
    /** The underlying Turso connection. */
    readonly $client: Client;

    /** Connect native query builders to the physical database. */
    constructor(client: Client, options: DrizzleSQLiteConfig<Relations> = {}) {
        // open a session over the relations and client
        const relations = options.relations ?? ({} as Relations);
        const session = new ConnectionSession<RunResult<Client>, Relations>(
            client as ConnectionClient<RunResult<Client>>,
            relations,
            options,
        );
        super("async", session.dialect, session, relations);
        this.$client = client;
    }
}

/** The result of executing a Turso statement. */
type RunResult<Client extends ConnectionClient<unknown>> = Awaited<ReturnType<Client["run"]>>;
