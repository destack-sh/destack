import { defineSchema, schema } from "@destack/schema";
import { ModuleMetadata, type Package } from "@destack/package";
import { defineResourceKind, ResourceDeclaration } from "@destack/resource";
import type { ResourceContext } from "@destack/resource/context";
import type { DatabaseConnection } from "../database/connection.ts";
import { connectors } from "#connector";
import { TABLE, type Table } from "../table/table.ts";
import { expandTrees } from "../tree/tree.ts";
import { DatabaseState, declareState } from "../migration/state.ts";
import { DatabaseTier } from "./tier.ts";
export type { DatabaseConnection } from "../database/connection.ts";

export { DatabaseTier } from "./tier.ts";

/** A database's resource settings. */
export const DatabaseSpec = defineSchema(schema.object({ tier: DatabaseTier }));
/** A database's resource settings. */
export type DatabaseSpec = schema.Infer<typeof DatabaseSpec>;

/** The database resource kind: tables planned toward the union of their declared states. */
export const DatabaseKind = defineResourceKind("database", {
    spec: DatabaseSpec,
    state: DatabaseState,
});
/** A named database dependency. */
export type DatabaseDescription = schema.Infer<typeof DatabaseKind.description>;

/** A database declaration. */
export class Database extends ResourceDeclaration<DatabaseConnection, DatabaseDescription> {
    /** The tables the database holds, referencing tables held elsewhere without foreign keys. */
    readonly tables: readonly Table[];

    /** Create the declaration. */
    constructor(owner: Package, description: DatabaseDescription, tables: readonly Table[]) {
        super(owner, description);
        this.tables = tables;
    }

    /** The connectors opening databases on the running runtime. */
    override get connectors() {
        return connectors;
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

/**
 * Declare a database.
 *
 * A database holds tables of its own tier and of wider ones, whose rows it replicates from their home.
 */
export function defineDatabase(definition: DatabaseDefinition, module?: ModuleMetadata): Database {
    // reject duplicate SQL names and tables of a narrower tier
    const owner = ModuleMetadata.require(module, "defineDatabase").package;
    const tier = definition.tier ?? "zonal";
    const tiers = DatabaseTier.options;
    const names = new Map<string, Table>();
    for (const table of expandTrees(definition.tables)) {
        const { sqlName, tier: declared } = table[TABLE];
        const existing = names.get(sqlName);
        if (existing && existing !== table) {
            throw new TypeError(`duplicate SQL table: ${sqlName}`);
        }
        if (declared !== undefined && tiers.indexOf(declared) > tiers.indexOf(tier)) {
            throw new TypeError(`${declared} table ${sqlName} in a ${tier} database`);
        }
        names.set(sqlName, table);
    }

    // validate the resource description
    const description = DatabaseKind.description.parse({
        name: definition.name,
        kind: "database",
        spec: { tier },
    });

    return new Database(owner, description, [...names.values()]);
}
