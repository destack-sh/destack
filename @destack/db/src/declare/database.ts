import { defineSchema, schema } from "@destack/schema";
import { ModuleMetadata, type Package } from "@destack/package";
import {
    defineResourceKind,
    ResourceDeclaration,
    type Connector,
    type ResourceBinding,
} from "@destack/resource";
import type { ResourceContext } from "@destack/resource/context";
import type { DatabaseConnection } from "../database/connection.ts";
import { connectors } from "#connector";
import { TABLE, type Table } from "../table/table.ts";
import { expandTrees } from "../tree/tree.ts";
import { DatabaseState, declareState } from "../migration/state.ts";
import { Relations } from "../query/relation.ts";
import type { Model } from "../query/model.ts";

/** A database's resource settings. */
export const DatabaseSpec = defineSchema(
    schema.object({
        /** The SQL names of the tables the database copies from their owning service. */
        copies: schema.array(schema.string().min(1)),
    }),
);
/** A database's resource settings. */
export type DatabaseSpec = schema.Infer<typeof DatabaseSpec>;

/** The database resource kind: tables planned toward the union of their declared states. */
export const DatabaseKind = defineResourceKind("database", {
    spec: DatabaseSpec,
    state: DatabaseState,
});
/** A named database dependency. */
export type DatabaseDescription = schema.Infer<typeof DatabaseKind.description>;

/** Opens databases of one provider inside a workload, typed by each declaration's models. */
export interface DatabaseConnector {
    /** The provider code the connector connects to, such as sqlite. */
    readonly code: string;
    /** Open a database the caller disposes, refusing one lacking its declaration's tables. */
    connect<Models extends Readonly<Record<string, Model>>>(
        binding: ResourceBinding,
        declaration: Database<Models>,
    ): Promise<DatabaseConnection<Models> & AsyncDisposable>;
}

/** A database declaration. */
export class Database<
    Models extends Readonly<Record<string, Model>> = Readonly<Record<string, Model>>,
> extends ResourceDeclaration<DatabaseConnection<Models>, DatabaseDescription> {
    /** The tables the database keeps, its own and its copies, referencing tables kept elsewhere without foreign keys. */
    readonly tables: readonly Table[];
    /** The relations of the tables, which relational reads name. */
    readonly relations: Relations<Models>;

    /** Create the declaration. */
    constructor(
        owner: Package,
        description: DatabaseDescription,
        tables: readonly Table[],
        relations: Relations<Models>,
    ) {
        super(owner, description);
        this.tables = tables;
        this.relations = relations;
    }

    /** The connectors opening databases on the running runtime. */
    override get connectors(): Readonly<
        Record<string, Connector<DatabaseConnection<Models>, this>>
    > {
        return connectors;
    }

    /** Decide whether the database keeps a table's rows as copies from their owning service. */
    copies(table: Table): boolean {
        return this.spec.copies.includes(table[TABLE].sqlName);
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
    override get(context: ResourceContext): DatabaseConnection<Models> {
        return context.get(this);
    }

    /** List the required tables a connected database has not applied. */
    check(connection: DatabaseConnection): Promise<string[]> {
        return connection.unapplied(this.tables);
    }
}

/** A database as authored. */
export interface DatabaseDefinition<
    Models extends Readonly<Record<string, Model>> = Readonly<Record<string, Model>>,
> {
    /** The package-local database name. */
    readonly name: string;
    /** The tables the database owns, referencing tables kept elsewhere without foreign keys. */
    readonly tables: readonly Table[];
    /** The tables the database copies from their owning service, which its follows fill, none by default. */
    readonly copies?: readonly Table[];
    /** The relations of the tables. */
    readonly relations?: Relations<Models>;
}

/** Declare a database of the tables it owns and the tables it copies from their owning service. */
export function defineDatabase<
    Models extends Readonly<Record<string, Model>> = Readonly<Record<string, Model>>,
>(definition: DatabaseDefinition<Models>, module?: ModuleMetadata): Database<Models> {
    // reject duplicate SQL names within and across the owned and copied tables
    const owner = ModuleMetadata.require(module, "defineDatabase").package;
    const names = new Map<string, Table>();
    const copies = expandTrees(definition.copies ?? []);
    for (const table of [...expandTrees(definition.tables), ...copies]) {
        const { sqlName } = table[TABLE];
        if (names.has(sqlName)) {
            throw new TypeError(`duplicate SQL table: ${sqlName}`);
        }
        names.set(sqlName, table);
    }

    // require relations among the database's tables
    const relations = definition.relations ?? new Relations<Models>();
    const kept = new Set(names.values());
    for (const [name, table] of Object.entries(relations.tables)) {
        if (!kept.has(table)) {
            throw new TypeError(`relations name table ${name}, which the database does not keep`);
        }
    }

    // validate the resource description
    const description = DatabaseKind.description.parse({
        name: definition.name,
        kind: "database",
        spec: { copies: copies.map((table) => table[TABLE].sqlName) },
    });

    return new Database(owner, description, [...names.values()], relations);
}
