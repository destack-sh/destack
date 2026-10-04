import {
    and,
    dialectSQL,
    eq,
    inArray,
    Key,
    or,
    sql,
    TABLE,
    uniqueIndex,
    type DatabaseConnection,
    type Row,
    type SQL,
    type Table,
} from "@destack/db";
import { present, schema } from "@destack/schema";
import type { Projector } from "@destack/sync";
import type { ObjectType } from "../object/object.ts";
import type { Trait } from "./trait.ts";

/** How objects project the rows of another object type people receive, one object in each recipient's home, as a read model of them. */
export interface ProjectedDefinition {
    /** The object type whose rows the objects project. */
    readonly from: ObjectType | (() => ObjectType);
    /** The subject field of the source naming each row's recipient. */
    readonly to: string;
    /** The qualified reference field naming each object's source row, unique in its scope. */
    readonly source: string;
    /** The fields kept equal to the source row's, each naming its source field, such as the source's scope. */
    readonly fields?: Readonly<Record<string, string>>;
    /** The object's own fields a change of the kept fields clears, such as its read state. */
    readonly clears?: readonly string[];
    /** The copied types naming the residents of each home and the installations keeping rows for them. */
    readonly residence: Residence;
}

/** The copied types telling a home whose rows it projects: its residents' users with their homes, and the addresses in their scopes. */
export interface Residence {
    /** The users, each with the `home` field naming its home space. */
    readonly user: ObjectType;
    /** The addresses in users' scopes, each with the `source` scope and the `installation` keeping rows for the user. */
    readonly address: ObjectType;
}

/** The qualified reference of a source row, as the source field keeps it. */
const SourceReference = schema.object({
    scope: schema.string().min(1),
    id: schema.string().min(1),
});

/** Objects projected from another type's rows: each its own object with its own state, created with its source and retracted when the source leaves. */
export const projected: Trait<ProjectedDefinition> & {
    /** Build the projector writing a type's objects into one scope from its source's rows. */
    projector(object: ObjectType, options: ProjectedDefinition, into: string): Projector;
    /** Read the source type a projection names. */
    sourceOf(options: ProjectedDefinition): ObjectType;
} = {
    key: "projected",
    isDurable: true,
    options: (definition) => definition.projected,
    columns: () => ({}),
    constraints: (options, table, columns) => [
        uniqueIndex(`${table}_source`).on(
            present(columns["scope"], "the scope column"),
            present(columns[options.source], "the source column"),
        ),
    ],
    methods: () => ({}),
    validate: (options, object) => {
        // require a qualified reference to the source type, and a subject field naming its recipient
        const field = object.fields[options.source];
        const source = projected.sourceOf(options);
        if (field?.type !== "reference" || !field.qualified || !source.same(field.target?.())) {
            throw new TypeError(
                `object ${object.name} projects ${source.name} through no qualified reference field ${options.source}`,
            );
        } else if (source.fields[options.to]?.type !== "subject") {
            throw new TypeError(
                `object ${object.name} projects ${source.name} to no subject field ${options.to}`,
            );
        }

        // require the kept fields on both types, and the cleared fields on the object's own
        for (const [kept, from] of Object.entries(options.fields ?? {})) {
            if (
                object.fields[kept] === undefined ||
                !Object.hasOwn(source.table[TABLE].columns, from)
            ) {
                throw new TypeError(
                    `object ${object.name} keeps no field ${kept} from ${source.name}'s ${from}`,
                );
            }
        }
        const cleared = (options.clears ?? []).find(
            (name) =>
                object.fields[name] === undefined || Object.hasOwn(options.fields ?? {}, name),
        );
        if (cleared !== undefined) {
            throw new TypeError(`object ${object.name} clears no own field ${cleared}`);
        }

        // require the residents' homes and the addresses naming their sources
        const { user, address } = options.residence;
        const missing = [
            ...(user.fields["home"] === undefined ? [`${user.name}.home`] : []),
            ...["source", "installation"].flatMap((name) =>
                address.fields[name] === undefined ? [`${address.name}.${name}`] : [],
            ),
        ];
        if (missing.length > 0) {
            throw new TypeError(
                `object ${object.name} projects for residents without ${missing.join(", ")}`,
            );
        }
    },
    projector(object, options, into) {
        return new ScopeProjector(object, options, into);
    },

    /** Read the source type a projection names. */
    sourceOf(options: ProjectedDefinition): ObjectType {
        return typeof options.from === "function" ? options.from() : options.from;
    },
};

/** The projector writing a type's objects into one scope from its source's rows. */
class ScopeProjector implements Projector {
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
