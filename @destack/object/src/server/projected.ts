import {
    and,
    dialectSQL,
    eq,
    inArray,
    Key,
    or,
    sql,
    TABLE,
    type DatabaseConnection,
    type Row,
    type SQL,
    type Table,
} from "@destack/db";
import { present } from "@destack/schema";
import type { Projector } from "@destack/sync";
import type { ObjectType } from "../object/object.ts";

import { projected, type ProjectedDefinition, SourceReference } from "../trait/projected.ts";
/** The projector writing a type's objects into one scope from its source's rows. */
export class ScopeProjector implements Projector {
    /** The source table whose rows the objects project. */
    readonly source: Table;
    /** The projecting type. */
    readonly #object: ObjectType;
    /** The scope the objects live in. */
    readonly #into: string;
    /** The source field naming each object's source row. */
    readonly #field: string;
    /** The kept fields, each with the source field it copies. */
    readonly #kept: readonly (readonly [string, string])[];
    /** The own fields a change of the kept fields clears, each set to null. */
    readonly #cleared: Readonly<Record<string, null>>;
    /** Whether a conflicting row's kept fields changed, absent without kept fields. */
    readonly #changed: SQL | undefined;

    /** Prepare the writes of a type's objects into one scope. */
    constructor(object: ObjectType, options: ProjectedDefinition, into: string) {
        // keep the type, the scope and the fields the objects copy and clear
        const definition = object.table[TABLE];
        this.source = projected.sourceOf(options).table;
        this.#object = object;
        this.#into = into;
        this.#field = options.source;
        this.#kept = Object.entries(options.fields ?? {});
        this.#cleared = Object.fromEntries((options.clears ?? []).map((name) => [name, null]));

        // detect a change of the kept fields on conflict
        this.#changed = or(
            ...this.#kept.map(([name]) => {
                const target = definition.column(name);
                const excluded = sql`excluded.${sql.identifier(target.definition.name)}`;

                return dialectSQL({
                    sqlite: sql`${target} IS NOT ${excluded}`,
                    postgresql: sql`${target} IS DISTINCT FROM ${excluded}`,
                });
            }),
        );
    }

    /** Retract the projections of removed rows and project each kept row once. */
    async write(
        transaction: DatabaseConnection,
        scope: string,
        kept: readonly Row[],
        removed: readonly Row[],
    ): Promise<void> {
        // retract the projections of removed rows
        if (removed.length > 0) {
            await transaction.delete(this.#object.table).where(this.#projected(scope, removed));
        }

        // project each kept row once
        const now = Date.now();
        for (const row of kept) {
            await this.#project(transaction, scope, row, now);
        }
    }

    /** Retract the projections of the source scope's rows a snapshot left out. */
    async prune(
        transaction: DatabaseConnection,
        scope: string,
        delivered: ReadonlySet<string>,
    ): Promise<void> {
        // find the projections of the scope's rows the snapshot left out
        const table = this.#object.table;
        const definition = table[TABLE];
        const rows = await transaction
            .select({ id: definition.column("id"), source: definition.column(this.#field) })
            .from(table)
            .where(eq(definition.column("scope"), this.#into));
        const stale = rows.filter((row) => {
            const reference = SourceReference.parse(row.source);

            return (
                reference.scope === scope &&
                !delivered.has(Key.name(this.source, { id: reference.id }))
            );
        });

        // retract them
        if (stale.length > 0) {
            await transaction.delete(table).where(
                inArray(
                    definition.column("id"),
                    stale.map((row) => row.id),
                ),
            );
        }
    }

    /** Project a kept row, keeping its fields current and clearing own state as a new revision when they change. */
    async #project(
        transaction: DatabaseConnection,
        scope: string,
        row: Row,
        now: number,
    ): Promise<void> {
        // insert the projection with the kept fields
        const table = this.#object.table;
        const definition = table[TABLE];
        const copied = Object.fromEntries(this.#kept.map(([name, from]) => [name, row[from]]));
        const insert = transaction.insert(table).values({
            id: this.#object.generateId(),
            scope: this.#into,
            [this.#field]: SourceReference.parse({ scope, id: row["id"] }),
            ...copied,
            createdAt: now,
            createdBy: null,
            updatedAt: now,
            updatedBy: null,
        });

        // keep an existing projection, or update it once its kept fields changed
        await (this.#kept.length === 0
            ? insert.onConflictDoNothing()
            : insert.onConflictDoUpdate({
                  target: [definition.column("scope"), definition.column(this.#field)],
                  set: {
                      ...copied,
                      ...this.#cleared,
                      updatedAt: now,
                      revision: sql`${definition.column("revision")} + 1`,
                  },
                  setWhere: present(this.#changed, "the change of the kept fields"),
              }));
    }

    /** Match the objects in the scope projected from some rows of a source scope. */
    #projected(scope: string, rows: readonly Row[]): SQL | undefined {
        const definition = this.#object.table[TABLE];

        return and(
            eq(definition.column("scope"), this.#into),
            inArray(
                definition.column(this.#field),
                rows.map((row) => SourceReference.parse({ scope, id: row["id"] })),
            ),
        );
    }
}
