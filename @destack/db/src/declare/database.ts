import { defineSchema, schema } from "@destack/schema";
import { declaringModule, type ModuleMetadata, type Package } from "@destack/package";
import { defineResourceSchema, Resource } from "@destack/resource";
import type { ResourceContext } from "@destack/resource/context";
import type { DatabaseConnection } from "../database/connection.ts";
import { TABLE, type Table } from "../table/table.ts";
import { expandTrees } from "../tree/tree.ts";
import {
    type DatabaseState,
    declareState,
    readState,
    unappliedTables,
} from "../migration/state.ts";
export type { DatabaseConnection } from "../database/connection.ts";

/** A database's resource settings, none since every database supports every dialect. */
export const DatabaseSpec = defineSchema(schema.object({}));
/** A database's resource settings, none since every database supports every dialect. */
export type DatabaseSpec = schema.Infer<typeof DatabaseSpec>;

/** A named database dependency. */
export const DatabaseDescription = defineResourceSchema("database", 1, DatabaseSpec);
/** A named database dependency. */
export type DatabaseDescription = schema.Infer<typeof DatabaseDescription>;

/** An inert database declaration with invocation-scoped connection access. */
export class Database extends Resource<DatabaseConnection, DatabaseDescription> {
    /** The tables the database holds, with every table they reference. */
    readonly tables: readonly Table[];

    /** Retain the declaration and its tables. */
    constructor(owner: Package, description: DatabaseDescription, tables: readonly Table[]) {
        super(owner, description);
        this.tables = tables;
    }

    /** Describe the tables the database requires in every dialect. */
    override state(): DatabaseState {
        return {
            tables: {
                sqlite: declareState(this.tables, "sqlite"),
                postgresql: declareState(this.tables, "postgresql"),
            },
        };
    }

    /** Retrieve the authorized connection. */
    override get(context: ResourceContext): DatabaseConnection {
        return context.get(this);
    }

    /** Name the tables this declaration requires that a connected database has not applied. */
    async check(connection: DatabaseConnection): Promise<string[]> {
        return unappliedTables(
            await readState(connection),
            declareState(this.tables, connection.dialect),
        );
    }
}

/** A database as authored: its name and the tables it holds. */
export interface DatabaseDefinition {
    /** The package-local database name. */
    readonly name: string;
    /** The tables the database holds, including every table they reference. */
    readonly tables: readonly Table[];
}

/** Declare a database dependency and the tables it holds. */
export function defineDatabase(definition: DatabaseDefinition, module?: ModuleMetadata): Database {
    // reject tables declared twice under one SQL name
    const owner = declaringModule(module, "defineDatabase").package;
    const names = new Map<string, Table>();
    for (const table of expandTrees(definition.tables)) {
        const existing = names.get(table[TABLE].sqlName);
        if (existing && existing !== table) {
            throw new TypeError(`duplicate SQL table: ${table[TABLE].sqlName}`);
        }
        names.set(table[TABLE].sqlName, table);
    }

    // validate the resource description
    const description = DatabaseDescription.parse({
        name: definition.name,
        kind: "database",
        version: 1,
        spec: {},
    });

    return new Database(owner, description, [...names.values()]);
}
