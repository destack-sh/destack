import { and, not, sql } from "drizzle-orm";
import { defineSchema, schema } from "@destack/schema";
import type { Chunk, CopyStage } from "@destack/resource";
import type { DatabaseConnection } from "../database/connection.ts";
import type { Dialect } from "../dialect/dialect.ts";
import type { ColumnDescription } from "../inspect/table.ts";
import type { ChangePage } from "../log/log.ts";
import { LogPosition } from "../log/position.ts";
import { Snapshot } from "../log/snapshot.ts";
import { Condition } from "../query/condition.ts";
import { Key } from "../query/key.ts";
import { Order } from "../query/order.ts";
import { jsonElements } from "../query/statement.ts";
import { decodeRow, encodeColumns, encodeRow, type Row } from "../table/row.ts";
import { Table, TABLE } from "../table/table.ts";
import {
    bigint,
    binary,
    boolean,
    Column,
    integer,
    json,
    numeric,
    real,
    text,
    timestamp,
    type ColumnBuilder,
    type JsonValue,
} from "../table/column.ts";
import { primaryKey } from "../table/constraint.ts";
import { mergeStates } from "../migration/merge.ts";
import type { DatabaseState, TableState } from "../migration/state.ts";

/**
 * The rows one chunk of a table carries.
 *
 * At 0.1 to 2 KB a row, a chunk is up to about 2 MB.
 */
const CHUNK_ROWS = 1000;
/**
 * How long a replication keeps the source's log after its cursor, in milliseconds.
 *
 * A pass over 1 GB at about 100 MB/s fits well within an hour.
 */
const SLOT_MILLISECONDS = 3_600_000;

/** The column builder of each logical kind. */
const COLUMNS: Readonly<Record<ColumnDescription["kind"], (name: string) => ColumnBuilder<any>>> = {
    text: (name) => text(name),
    integer,
    real,
    boolean,
    json: (name) => json(name, schema.json()),
    binary,
    bigint,
    numeric,
    timestamp,
};

/** Where an export continues. */
const ReplicationCursor = defineSchema(
    schema.object({
        /** The position the live tables' rows are read at. */
        position: LogPosition,
        /** The step: live rows and links, their changes, then fenced rows and links. */
        step: schema.number().int().nonnegative(),
        /** The key of the last row a table step read. */
        key: schema.record(schema.string(), schema.json()).optional(),
        /** The log sequence the changes step read up to. */
        sequence: schema.number().int().nonnegative(),
    }),
);
/** Where an export continues. */
type ReplicationCursor = schema.Infer<typeof ReplicationCursor>;

/** The content of one replication chunk. */
const ReplicationChunk = defineSchema(
    schema.discriminatedUnion("kind", [
        schema.object({
            /** A key range of a table's rows. */
            kind: schema.literal("rows"),
            /** The table's SQL name. */
            table: schema.string(),
            /** The key the range follows, absent from the table's start. */
            after: schema.record(schema.string(), schema.json()).optional(),
            /** The last key of the range, absent to the table's end. */
            last: schema.record(schema.string(), schema.json()).optional(),
            /** The range's rows in their JSON form. */
            rows: schema.array(schema.record(schema.string(), schema.json())),
        }),
        schema.object({
            /** A key range of a table's references to its own rows. */
            kind: schema.literal("links"),
            /** The table's SQL name. */
            table: schema.string(),
            /** Each row's key and self-references, in JSON form. */
            links: schema.array(schema.record(schema.string(), schema.json())),
        }),
        schema.object({
            /** Changes of the live tables in commit order. */
            kind: schema.literal("changes"),
            /** Each change's table and row, or the deleted key. */
            changes: schema.array(
                schema.object({
                    /** The table's SQL name. */
                    table: schema.string(),
                    /** Whether the change deleted the row. */
                    isDeleted: schema.boolean(),
                    /** The row after the change, or its deleted key, in JSON form. */
                    row: schema.record(schema.string(), schema.json()),
                }),
            ),
        }),
    ]),
);
/** The content of one replication chunk. */
type ReplicationChunk = schema.Infer<typeof ReplicationChunk>;

/**
 * A replication of a database's tables into another database, exact across dialects.
 *
 * Live tables are read at a snapshot position, then followed through the log.
 * Fenced tables, such as unlogged tables or tables with binary or sensitive columns, are copied whole.
 */
export class Replication {
    /** The fully logged tables, parents first. */
    readonly live: readonly Table[];
    /** The other tables, parents first. */
    readonly fenced: readonly Table[];
    /** The copied tables, by SQL name. */
    readonly #tables: ReadonlyMap<string, Table>;
    /** The self-referencing columns of each table. */
    readonly #links: ReadonlyMap<Table, readonly string[]>;
    /** The export steps, in order. */
    readonly #steps: readonly Step[];

    /** Order table states parents first and split them into live and fenced tables. */
    constructor(states: readonly TableState[]) {
        // build each table without tree indexes
        const derived = new Set(
            states.flatMap((state) =>
                state.tree === undefined ? [] : [state.tree.ancestors, state.tree.revision],
            ),
        );
        const copied = states.filter((state) => !derived.has(state.table.name));
        const tables = copied.map((state) => Replication.#describedTable(state));
        this.#tables = new Map(tables.map((table) => [table[TABLE].sqlName, table]));

        // read each table's references and self-references
        const references = new Map(
            copied.map((state, position) => {
                const parents = state.table.constraints.flatMap((constraint) =>
                    constraint.kind === "foreignKey" ? [constraint.table] : [],
                );

                return [tables[position]!, parents.flatMap((name) => this.#tables.get(name) ?? [])];
            }),
        );
        this.#links = new Map(
            copied.flatMap((state, position) => {
                // read the foreign keys to the table itself
                const columns = state.table.constraints.flatMap((constraint) =>
                    constraint.kind === "foreignKey" && constraint.table === state.table.name
                        ? constraint.columns
                        : [],
                );

                // refuse a required self-reference and non-text keys and links
                const required = columns.find(
                    (name) => !state.table.columns.find((column) => column.name === name)!.nullable,
                );
                if (required !== undefined) {
                    throw new TypeError(
                        `table ${state.table.name} references its own rows through required column ${required}`,
                    );
                }
                const textual = (name: string) =>
                    state.table.columns.find((column) => column.name === name)!.kind === "text";
                const keyColumns = state.table.constraints.flatMap((constraint) =>
                    constraint.kind === "primaryKey" ? constraint.columns : [],
                );
                if (columns.length > 0 && ![...columns, ...keyColumns].every(textual)) {
                    throw new TypeError(
                        `table ${state.table.name} references its own rows through columns other than text`,
                    );
                }

                return columns.length === 0 ? [] : [[tables[position]!, columns] as const];
            }),
        );

        // order parents first
        const ordered = parentsFirst(tables, references);

        // keep tables live while they and their parents log every column
        const live = new Set<Table>();
        for (const table of ordered) {
            const isLogged =
                table[TABLE].retention !== "none" &&
                Object.keys(table[TABLE].logged).length === table[TABLE].entries.length;
            const parents = references.get(table)!.filter((parent) => parent !== table);
            if (isLogged && parents.every((parent) => live.has(parent))) {
                live.add(table);
            }
        }
        this.live = ordered.filter((table) => live.has(table));
        this.fenced = ordered.filter((table) => !live.has(table));

        // read each table's rows then links, with the changes between live and fenced tables
        const steps = (tables: readonly Table[], isLive: boolean): Step[] =>
            tables.flatMap((table) => [
                { kind: "rows", table, isLive } as const,
                ...(this.#links.has(table) ? [{ kind: "links", table, isLive } as const] : []),
            ]);
        this.#steps = [
            ...steps(this.live, true),
            { kind: "changes" },
            ...steps(this.fenced, false),
        ];
    }

    /** The copied tables. */
    get tables(): readonly Table[] {
        return [...this.live, ...this.fenced];
    }

    /** Copy the tables of a database's desired states. */
    static of(desired: readonly DatabaseState[], dialect: Dialect): Replication {
        const tables = desired.map((state) => state.tables[dialect]);

        return new Replication(mergeStates(tables).declared);
    }

    /** Build a table from its state. */
    static #describedTable(state: TableState): Table {
        // build each column by kind, marking unlogged columns sensitive
        const description = state.table;
        const logged = new Set(state.log?.columns ?? []);
        const columns = Object.fromEntries(
            description.columns.map((column) => {
                const built = COLUMNS[column.kind](column.name);
                const definition = {
                    ...built.definition,
                    nullable: column.nullable,
                    ...(state.log !== undefined &&
                    column.kind !== "binary" &&
                    !logged.has(column.name)
                        ? { classification: "sensitive" as const }
                        : {}),
                    ...(column.generated === undefined
                        ? {}
                        : {
                              generated: {
                                  expression: sql.raw(column.generated.expression),
                                  mode: column.generated.mode ?? "virtual",
                              },
                          }),
                };

                return [column.name, new Column(description.name, definition)];
            }),
        );

        // key and log the table
        const key = description.constraints.find(
            (
                constraint,
            ): constraint is Extract<typeof constraint, { kind: "primaryKey" | "unique" }> =>
                constraint.kind === "primaryKey",
        );

        return new Table(
            { package: state.package, name: description.name, sqlName: description.name },
            columns,
            {
                constraints: () =>
                    key === undefined
                        ? []
                        : [
                              primaryKey({
                                  columns: key.columns.map((name) => columns[name]!) as [
                                      Column,
                                      ...Column[],
                                  ],
                              }),
                          ],
                retention: state.log?.retention ?? "none",
                moved: {},
                convert: {},
                aggregates: [],
                dependents: [],
            },
        );
    }

    /**
     * Read a source's content after a cursor as chunks until the stage ends.
     *
     * The live stage reads live rows at a snapshot and their changes.
     * The fenced stage reads the later changes, then the fenced rows.
     */
    async *export(
        database: DatabaseConnection,
        name: string,
        stage: CopyStage,
        after: string | undefined,
        signal: AbortSignal,
    ): AsyncGenerator<Chunk> {
        // start at the current position, or again after a new epoch
        let cursor: ReplicationCursor | undefined =
            after === undefined ? undefined : ReplicationCursor.parse(JSON.parse(after));
        const position = await database.log.position();
        if (cursor === undefined || cursor.position.epoch !== position.epoch) {
            cursor = { position, step: 0, sequence: position.sequence };
        }
        const changes = this.#steps.findIndex((step) => step.kind === "changes");
        const end = stage === "live" ? changes : this.#steps.length - 1;

        // read each step, advancing the replication's slot meanwhile
        while (cursor.step <= end && !signal.aborted) {
            await database.log.advance(name, cursor.sequence, Date.now() + SLOT_MILLISECONDS);
            const step: Step = this.#steps[cursor.step]!;

            // read the changes until caught up
            if (step.kind === "changes") {
                const page: ChangePage = await database.log.read({
                    tables: this.live,
                    after: cursor.sequence,
                });
                if (page.changes.length > 0) {
                    const body: ReplicationChunk = {
                        kind: "changes",
                        changes: page.changes.map((change) => ({
                            table: change.table[TABLE].sqlName,
                            isDeleted: change.operation === "delete",
                            row: encodeRow(change.table, change.after ?? change.key),
                        })),
                    };
                    cursor = { ...cursor, sequence: page.sequence };
                    yield { cursor: JSON.stringify(cursor), body };
                } else {
                    cursor = { ...cursor, step: cursor.step + 1, sequence: page.sequence };
                }
                continue;
            }

            // read a range of a table's rows
            const table: Table = step.table;
            const from = cursor.key === undefined ? undefined : decodeRow(table, cursor.key);
            const rows: Row[] = step.isLive
                ? await new Snapshot(database, cursor.position).ordered(table, {
                      where: Condition.all(),
                      order: Order.complete([], table),
                      ...(from === undefined ? {} : { after: from }),
                      count: CHUNK_ROWS,
                  })
                : await this.#rows(database, table, from, CHUNK_ROWS);
            const last: Record<string, JsonValue> | undefined =
                rows.length < CHUNK_ROWS ? undefined : keyOf(table, rows.at(-1)!);

            // send the rows without self-references, or the self-references alone
            const links = this.#links.get(table) ?? [];
            const body: ReplicationChunk =
                step.kind === "rows"
                    ? {
                          kind: "rows",
                          table: table[TABLE].sqlName,
                          ...(cursor.key === undefined ? {} : { after: cursor.key }),
                          ...(last === undefined ? {} : { last }),
                          rows: rows.map((row) =>
                              encodeRow(table, {
                                  ...row,
                                  ...Object.fromEntries(links.map((column) => [column, null])),
                              }),
                          ),
                      }
                    : {
                          kind: "links",
                          table: table[TABLE].sqlName,
                          links: rows.map((row) => ({
                              ...keyOf(table, row),
                              ...Object.fromEntries(
                                  links.map((column) => [
                                      column,
                                      encodeRow(table, row)[column] ?? null,
                                  ]),
                              ),
                          })),
                      };
            cursor =
                last === undefined
                    ? {
                          position: cursor.position,
                          step: cursor.step + 1,
                          sequence: cursor.sequence,
                      }
                    : { ...cursor, key: last };
            yield { cursor: JSON.stringify(cursor), body };
        }
    }

    /** Write one chunk into a target in one transaction. */
    async import(database: DatabaseConnection, chunk: Chunk): Promise<void> {
        const body = ReplicationChunk.parse(chunk.body);
        await database.transaction(
            async (transaction) => {
                // replace the target's rows in the range
                if (body.kind === "rows") {
                    const table = this.#table(body.table);
                    const rows = body.rows.map((row) => decodeRow(table, row));
                    const kept = new Set(rows.map((row) => Key.name(table, row)));
                    const held = await this.#range(transaction, table, body.after, body.last);
                    await transaction.remove(
                        table,
                        held.filter((row) => !kept.has(Key.name(table, row))),
                    );
                    await transaction.upsert(table, rows);
                }
                // set the range's self-references
                else if (body.kind === "links") {
                    const table = this.#table(body.table);
                    const links = this.#links.get(table)!;
                    const key = table[TABLE].key;
                    const field = (name: string) => sql`linked.value ->> ${name}`;
                    await transaction.execute(sql`
                        UPDATE ${table}
                        SET ${sql.join(
                            links.map((name) => sql`${sql.identifier(name)} = ${field(name)}`),
                            sql`, `,
                        )}
                        FROM ${jsonElements(sql`${JSON.stringify(body.links)}`, "linked")}
                        WHERE ${sql.join(
                            key.map(
                                (name) => sql`${table}.${sql.identifier(name)} = ${field(name)}`,
                            ),
                            sql` AND `,
                        )}
                    `);
                }
                // apply each row's last change, removals first
                else {
                    const last = new Map<string, { table: Table; row: Row; isDeleted: boolean }>();
                    for (const change of body.changes) {
                        const table = this.#table(change.table);
                        const row = decodeRow(table, change.row);
                        last.set(`${change.table}/${Key.name(table, row)}`, {
                            table,
                            row,
                            isDeleted: change.isDeleted,
                        });
                    }
                    for (const table of this.live) {
                        const changed = [...last.values()].filter((entry) => entry.table === table);
                        await transaction.remove(
                            table,
                            changed.filter((entry) => entry.isDeleted).map((entry) => entry.row),
                        );
                        await transaction.upsert(
                            table,
                            changed.filter((entry) => !entry.isDeleted).map((entry) => entry.row),
                        );
                    }
                }
            },
            { constraints: "deferred" },
        );
    }

    /** Read up to a count of a table's rows in key order after a row. */
    async #rows(
        database: DatabaseConnection,
        table: Table,
        after: Row | undefined,
        count: number,
    ): Promise<Row[]> {
        const order = Order.complete([], table);

        return database
            .select()
            .from(table)
            .where(after === undefined ? undefined : Order.after(order, table, after))
            .orderBy(...Order.render(order, table))
            .limit(count);
    }

    /** Read the keys in a key range. */
    async #range(
        database: DatabaseConnection,
        table: Table,
        after: Record<string, JsonValue> | undefined,
        last: Record<string, JsonValue> | undefined,
    ): Promise<Row[]> {
        // select the key columns in the range
        const order = Order.complete([], table);
        const columns = table[TABLE].columns;
        const key = Object.fromEntries(table[TABLE].key.map((name) => [name, columns[name]!]));

        return database
            .select(key)
            .from(table)
            .where(
                and(
                    after === undefined
                        ? undefined
                        : Order.after(order, table, decodeRow(table, after)),
                    last === undefined
                        ? undefined
                        : not(Order.after(order, table, decodeRow(table, last))),
                ),
            );
    }

    /** Find a copied table by SQL name. */
    #table(name: string): Table {
        const table = this.#tables.get(name);
        if (table === undefined) {
            throw new TypeError(`copied chunk names ${name}, which the database does not hold`);
        }

        return table;
    }
}

/** Write a row's key in JSON form. */
function keyOf(table: Table, row: Row): Record<string, JsonValue> {
    const key = table[TABLE].entries.filter(([property]) => table[TABLE].key.includes(property));

    return encodeColumns(key, row, []);
}

/** Order tables parents first and refuse cycles. */
function parentsFirst(
    tables: readonly Table[],
    references: ReadonlyMap<Table, readonly Table[]>,
): Table[] {
    // walk the tables depth first
    const ordered: Table[] = [];
    const visiting: Table[] = [];
    const pending = [...tables].reverse().map((table) => ({ table, isExpanded: false }));
    while (pending.length > 0) {
        const { table, isExpanded } = pending.pop()!;

        // place a table once its parents are placed
        if (isExpanded) {
            visiting.splice(visiting.indexOf(table), 1);
            ordered.push(table);
        }
        // refuse a cycle through a table still waiting for its parents
        else if (visiting.includes(table)) {
            throw new TypeError(`tables referencing ${table[TABLE].sqlName} form a cycle`);
        }
        // place an unplaced table's parents first
        else if (!ordered.includes(table)) {
            visiting.push(table);
            pending.push({ table, isExpanded: true });
            for (const parent of references.get(table)!) {
                if (parent !== table && !ordered.includes(parent)) {
                    pending.push({ table: parent, isExpanded: false });
                }
            }
        }
    }

    return ordered;
}

/** One export step. */
type Step =
    | {
          /** A table's rows, or its references to rows of the same table. */
          readonly kind: "rows" | "links";
          /** The table. */
          readonly table: Table;
          /** Whether the table is live or fenced. */
          readonly isLive: boolean;
      }
    | {
          /** The live tables' changes. */
          readonly kind: "changes";
      };
