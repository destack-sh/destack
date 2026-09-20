import postgres from "postgres";
import { drizzle, type PostgresJsDatabase } from "drizzle-orm/postgres-js";
import type { EmptyRelations } from "drizzle-orm/relations";
import type { DrizzlePgConfig } from "drizzle-orm/pg-core/utils";
import { ConnectionState, DatabaseConnection } from "../database/connection.ts";
import { DatabaseDriver } from "../database/driver.ts";
import { PostgresSchemaCompiler } from "./compiler.ts";
import type { NativeRelations } from "../dialect/table.ts";
import type { TableRelations } from "../schema/relation.ts";
import { type DatabaseSchema, orderSchemas } from "../schema/schema.ts";
import type { Table } from "../table/table.ts";

/** Portable queries with an owned PostgreSQL connection pool. */
export class Database<Relations extends Record<string, TableRelations> = {}>
    extends DatabaseConnection<"postgresql", Relations> {
    /** The PostgreSQL connection pool. */
    readonly $client: postgres.Sql;
    /** The explicit native SQL API. */
    readonly native: PostgresJsDatabase<NativeRelations<"postgresql", Relations>>;

    /** Bind logical tables to a PostgreSQL connection pool. */
    constructor(
        client: postgres.Sql,
        schema: DatabaseSchema<Record<string, Table>, Relations> | readonly Table[],
        options: Omit<DrizzlePgConfig<EmptyRelations>, "relations"> = {},
    ) {
        const definition = Array.isArray(schema)
            ? undefined
            : schema as DatabaseSchema<Record<string, Table>, Relations>;
        const tables = definition
            ? orderSchemas([definition]).flatMap((schema) => Object.values(schema.tables))
            : schema as readonly Table[];
        const logical = definition?.relations ?? {} as Relations;
        const declarations = new PostgresSchemaCompiler(tables);
        const relations = declarations.relations(logical);
        const native = drizzle({ ...options, relations, client });
        super(
            new DatabaseDriver({ dialect: "postgresql", database: native }, new ConnectionState()),
            declarations,
            logical,
        );
        this.$client = client;
        this.native = native;
    }

    /** Close the connection pool after pending queries complete. */
    async close(): Promise<void> {
        await this.state.close(() => this.$client.end());
    }
}
