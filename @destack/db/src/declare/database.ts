import { schema } from "@destack/schema";
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
import { DatabaseState } from "../migration/state.ts";
import { Relations } from "../query/relation.ts";
import type { Model } from "../query/model.ts";

/** A database's resource settings. */
export const DatabaseSpec = schema.object({
    /** The SQL names of the tables the database replicates from the package that declares them. */
    replicated: schema.array(schema.string().min(1)),
});
/** A database's resource settings. */
export type DatabaseSpec = schema.Infer<typeof DatabaseSpec>;

/** The database resource kind: tables planned toward the union of their declared states. */
export const DatabaseKind = defineResourceKind("database", {
    spec: DatabaseSpec,
    state: DatabaseState,
});

/** A declared database and its required tables, as the manifest records it. */
export const DatabaseDescription = DatabaseKind.description.extend(DatabaseState.shape);
/** A declared database and its required tables, as the manifest records it. */
export type DatabaseDescription = schema.Infer<typeof DatabaseDescription>;

/** Opens databases of one provider inside a workload, typed by each declaration's models. */
export interface DatabaseConnector {
    /** The provider code the connector connects to, such as sqlite. */
    readonly code: string;
    /** Open a database the caller disposes, refusing one lacking its declaration's tables. */
    readonly connect: <Models extends Readonly<Record<string, Model>>>(
        binding: ResourceBinding,
        declaration: Database<Models>,
    ) => Promise<DatabaseConnection<Models> & AsyncDisposable>;
}

/** A database declaration. */
export class Database<
    Models extends Readonly<Record<string, Model>> = Readonly<Record<string, Model>>,
> extends ResourceDeclaration<
    DatabaseConnection<Models>,
    schema.Infer<typeof DatabaseKind.description>
> {
    /** The database's declared and replicated tables, without foreign keys to others. */
    readonly tables: readonly Table[];
    /** The relations of the tables, which relational reads follow. */
    readonly relations: Relations<Models>;
    /** The SQL names of the tables the database replicates from the package that declares them. */
    readonly replicated: ReadonlySet<string>;
    /** The required tables' description, described on first read. */
    #state: DatabaseState | undefined;

    /** Create the declaration. */
    constructor(
        owner: Package,
        description: schema.Infer<typeof DatabaseKind.description>,
        tables: readonly Table[],
        relations: Relations<Models>,
    ) {
        // store the tables, their relations and the replicated tables' names
        super(owner, description);
        this.tables = tables;
        this.relations = relations;
        this.replicated = new Set(description.spec.replicated);
    }

    /** The connectors opening databases on the running runtime. */
    override get connectors(): Readonly<
        Record<string, Connector<DatabaseConnection<Models>, this>>
    > {
        return connectors;
    }

    /** Decide whether the database stores a table's rows as a replica. */
    replicates(table: Table): boolean {
        return this.replicated.has(table[TABLE].sqlName);
    }

    /** Describe the required tables once, since the declaration never changes. */
    override state(): DatabaseState {
        this.#state ??= DatabaseState.of(this.tables, { replicated: this.replicated });

        return this.#state;
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
    /** The database's declared tables, referencing tables stored elsewhere without foreign keys. */
    readonly tables: readonly Table[];
    /** The tables the database replicates from their declaring package, filled by follows. */
    readonly replicated?: readonly Table[];
    /** The relations of the tables. */
    readonly relations?: Relations<Models>;
}

/** Declare a database of its tables and those it replicates from their declaring package. */
export function defineDatabase<
    Models extends Readonly<Record<string, Model>> = Readonly<Record<string, Model>>,
>(definition: DatabaseDefinition<Models>, module?: ModuleMetadata): Database<Models> {
    // reject duplicate SQL names within and across the declared and replicated tables
    const owner = ModuleMetadata.require(module, "defineDatabase").package;
    const names = new Map<string, Table>();
    const replicated = expandTrees(definition.replicated ?? []);
    for (const table of [...expandTrees(definition.tables), ...replicated]) {
        const { sqlName } = table[TABLE];
        if (names.has(sqlName)) {
            throw new TypeError(`duplicate SQL table: ${sqlName}`);
        }
        names.set(sqlName, table);
    }

    // require the relations among the database's tables
    const relations = definition.relations ?? new Relations<Models>();
    const kept = new Set(names.values());
    for (const [name, table] of Object.entries(relations.tables)) {
        if (!kept.has(table)) {
            throw new TypeError(
                `relations reference table ${name}, but the database does not declare it`,
            );
        }
    }

    // validate the resource description
    const description = DatabaseKind.description.parse({
        name: definition.name,
        kind: "database",
        spec: { replicated: replicated.map((table) => table[TABLE].sqlName) },
    });

    return new Database(owner, description, [...names.values()], relations);
}
