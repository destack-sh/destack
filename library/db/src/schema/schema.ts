import { ResourceName } from "@destack/resource";
import type { Table } from "../table/table.ts";
import type { TableRelations } from "./relation.ts";

/** A named collection of tables and its committed migration history. */
export interface DatabaseSchema<
    Tables extends Record<string, Table> = Record<string, Table>,
    Relations extends Record<string, TableRelations> = {},
> {
    /** The stable schema name, unique within its database. */
    readonly name: string;
    /** The tables managed by this schema. */
    readonly tables: Tables;
    /** Schemas whose tables must exist before this schema is applied. */
    readonly dependencies?: readonly DatabaseSchema[];
    /** Relationships available through this schema's query API. */
    readonly relations?: Relations;
    /** The migration root, containing one history directory per supported dialect. */
    readonly migrations: URL;
}

/** Declare a schema without opening a database or applying migrations. */
export function defineDatabaseSchema<
    Tables extends Record<string, Table>,
    Relations extends Record<string, TableRelations> = {},
>(
    definition: DatabaseSchema<Tables, Relations>,
): DatabaseSchema<Tables, Relations> {
    ResourceName.parse(definition.name);

    return definition;
}

/** Order schema dependencies before their consumers and reject conflicting histories. */
export function orderSchemas(schemas: readonly DatabaseSchema[]): DatabaseSchema[] {
    const ordered: DatabaseSchema[] = [];
    const visited = new Map<string, DatabaseSchema>();
    const active = new Set<DatabaseSchema>();

    // traverse each root with one shared history-name index
    for (const schema of schemas) visitSchema(schema, visited, active, ordered);

    return ordered;
}

/** Append a schema after all of its dependencies. */
function visitSchema(
    schema: DatabaseSchema,
    visited: Map<string, DatabaseSchema>,
    active: Set<DatabaseSchema>,
    ordered: DatabaseSchema[],
): void {
    if (active.has(schema)) {
        throw new TypeError(`Cyclic database schema dependency: ${schema.name}.`);
    }
    const previous = visited.get(schema.name);
    if (previous && previous !== schema) {
        throw new TypeError(`Conflicting database schema: ${schema.name}.`);
    }
    if (previous) return;

    // retain declaration identity while visiting shared dependencies once
    visited.set(schema.name, schema);
    active.add(schema);
    for (const dependency of schema.dependencies ?? []) {
        visitSchema(dependency, visited, active, ordered);
    }
    active.delete(schema);
    ordered.push(schema);
}
