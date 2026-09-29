import { defineSchema, schema } from "@destack/schema";
import { declaringModule, type ModuleMetadata, type Package } from "@destack/package";
import { defineResourceSchema, Resource } from "@destack/resource";
import type { ResourceContext } from "@destack/resource/context";
import type { DatabaseConnection } from "../database/connection.ts";
import { providers } from "#provider";
import { TABLE, type Table } from "../table/table.ts";
import { expandTrees } from "../tree/tree.ts";
import { type DatabaseState, declareState } from "../migration/state.ts";
export type { DatabaseConnection } from "../database/connection.ts";

/** Where a database lives: once in the universe, once per region, or within one zone. */
export const DatabaseTier = defineSchema(schema.enum(["global", "regional", "zonal"]));
/** Where a database lives. */
export type DatabaseTier = schema.Infer<typeof DatabaseTier>;

/** A database's resource settings. */
export const DatabaseSpec = defineSchema(schema.object({ tier: DatabaseTier }));
/** A database's resource settings. */
export type DatabaseSpec = schema.Infer<typeof DatabaseSpec>;

/** A named database dependency. */
export const DatabaseDescription = defineResourceSchema("database", 1, DatabaseSpec);
/** A named database dependency. */
export type DatabaseDescription = schema.Infer<typeof DatabaseDescription>;

/** A database declaration. */
export class Database extends Resource<DatabaseConnection, DatabaseDescription> {
    /** The tables the database holds, referencing tables held elsewhere without foreign keys. */
    readonly tables: readonly Table[];

    /** Create the declaration. */
    constructor(owner: Package, description: DatabaseDescription, tables: readonly Table[]) {
        super(owner, description);
        this.tables = tables;
    }

    /** Open the providers holding databases on the running runtime. */
    override get providers() {
        return providers;
    }

    /** Describe the required tables. */
    override state(): DatabaseState {
        return {
            tables: {
                sqlite: declareState(this.tables, "sqlite"),
                postgresql: declareState(this.tables, "postgresql"),
            },
        };
    }

    /** Read the connection. */
    override get(context: ResourceContext): DatabaseConnection {
        return context.get(this);
    }

    /** Name the required tables a connected database has not applied. */
    check(connection: DatabaseConnection): Promise<string[]> {
        return connection.unapplied(this.tables);
    }
}

/** A database as authored. */
export interface DatabaseDefinition {
    /** The package-local database name. */
    readonly name: string;
    /** Where the database lives, within one zone when absent. */
    readonly tier?: DatabaseTier;
    /** The tables the database holds, referencing tables held elsewhere without foreign keys. */
    readonly tables: readonly Table[];
}

/** Declare a database. */
export function defineDatabase(definition: DatabaseDefinition, module?: ModuleMetadata): Database {
    // reject duplicate SQL names and tables of another tier
    const owner = declaringModule(module, "defineDatabase").package;
    const tier = definition.tier ?? "zonal";
    const names = new Map<string, Table>();
    for (const table of expandTrees(definition.tables)) {
        const { sqlName, tier: declared } = table[TABLE];
        const existing = names.get(sqlName);
        if (existing && existing !== table) {
            throw new TypeError(`duplicate SQL table: ${sqlName}`);
        }
        if (declared !== undefined && declared !== tier) {
            throw new TypeError(`${declared} table ${sqlName} in a ${tier} database`);
        }
        names.set(sqlName, table);
    }

    // validate the resource description
    const description = DatabaseDescription.parse({
        name: definition.name,
        kind: "database",
        version: 1,
        spec: { tier },
    });

    return new Database(owner, description, [...names.values()]);
}
