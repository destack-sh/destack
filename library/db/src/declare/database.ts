import { defineSchema, schema } from "@destack/schema";
import { defineResourceSchema, Resource } from "@destack/resource";
import type { ResourceContext } from "@destack/resource/context";
import { Dialect } from "../dialect/dialect.ts";
import type { DatabaseConnection } from "../database/connection.ts";
import type { DatabaseSchema } from "../schema/schema.ts";
import type { Table } from "../table/table.ts";
import type { TableRelations } from "../schema/relation.ts";
export type { DatabaseConnection } from "../database/connection.ts";

/** The SQL dialect used by a database. */
export const DatabaseSpec = defineSchema(
    schema.object({
        /** The dialect used by queries and migrations. */
        dialect: Dialect,
    }),
);
/** The SQL dialect used by a database. */
export type DatabaseSpec = schema.Infer<typeof DatabaseSpec>;

/** A named database dependency. */
export const DatabaseDeclaration = defineResourceSchema("database", 1, DatabaseSpec);
/** A named database dependency. */
export type DatabaseDeclaration = schema.Infer<typeof DatabaseDeclaration>;

/** An inert database declaration with invocation-scoped connection access. */
export class Database extends Resource<DatabaseConnection, DatabaseDeclaration> {
    /** Retrieve the authorized connection with source-inferred schema queries. */
    override get<Relations extends Record<string, TableRelations> = {}>(
        context: ResourceContext,
        schema?: DatabaseSchema<Record<string, Table>, Relations>,
    ): DatabaseConnection<Dialect, Relations> {
        const connection = context.get(this);

        // require the host binding to match the declared SQL dialect
        if (connection.connection.native.dialect !== this.spec.dialect) {
            throw new TypeError(
                `Database ${this.name} requires ${this.spec.dialect}, received ${connection.connection.native.dialect}.`,
            );
        }

        return schema
            ? connection.bind(schema)
            : (connection as DatabaseConnection<Dialect, Relations>);
    }
}

/** Declare a database dependency. */
export function defineDatabase(
    declaration: Omit<DatabaseDeclaration, "kind" | "version">,
): Database {
    const description = DatabaseDeclaration.parse({ ...declaration, kind: "database", version: 1 });

    return new Database(description);
}
