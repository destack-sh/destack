import { is, Placeholder, type Query, SQL, sql } from "drizzle-orm";
import { and, eq, inArray, or } from "./predicate.ts";
import type { DatabaseDriver, NativeDatabase } from "../database/driver.ts";
import type { SchemaCompiler } from "../dialect/compiler.ts";
import type { DrizzleDatabase, DrizzleMutation } from "../dialect/drizzle.ts";
import type { Column } from "../table/column.ts";
import { type Insert, type Select, TABLE, type Table } from "../table/table.ts";
import { selectFields, type SelectionResult } from "./selection.ts";
import type { PreparedQuery } from "./select.ts";
import { assertNever } from "../error/error.ts";
import type { SelectedFields, SQLiteTable } from "drizzle-orm/sqlite-core";

/** Values supplied to an insert or update. */
export type MutationRow<Definition extends Table> = {
    [Property in keyof Insert<Definition>]: Insert<Definition>[Property] | SQL | Placeholder;
};

/** Fields returned by a mutation. */
export interface ReturningSelection {
    /** A changed column or expression. */
    readonly [property: string]: Column | SQL | SQL.Aliased;
}

/** A typed insertion, update, or deletion with explicit returned rows. */
export class MutationQuery<
    Definition extends Table,
    Result = void,
    Operation extends "insert" | "update" | "delete" = "insert" | "update" | "delete",
> implements PromiseLike<Result> {
    /** The native database, its connection state and its transaction. */
    readonly driver: DatabaseDriver;
    /** The compiler binding logical tables to the native dialect. */
    readonly compiler: SchemaCompiler;
    /** The affected logical table. */
    readonly table: Definition;
    /** The SQL operation. */
    readonly operation: Operation;
    /** The inserted application records. */
    records: readonly MutationRow<Definition>[] = [];
    /** The updated application properties. */
    changes: Partial<MutationRow<Definition>> = {};
    /** The row predicate for updates and deletions. */
    predicate?: SQL;
    /** The fields returned after mutation. */
    fields?: ReturningSelection;
    /** The conflict handling for inserts. */
    conflict?:
        | {
              readonly action: "nothing";
              readonly target?: readonly Column[];
              readonly targetWhere?: SQL;
          }
        | {
              readonly action: "update";
              readonly target: readonly Column[];
              readonly targetWhere?: SQL;
              readonly setWhere?: SQL;
              readonly set: Partial<MutationRow<Definition>>;
          };

    /** Retain a mutation and its connection. */
    constructor(
        driver: DatabaseDriver,
        compiler: SchemaCompiler,
        table: Definition,
        operation: Operation,
    ) {
        // retain the connection, compiler, table and operation
        this.driver = driver;
        this.compiler = compiler;
        this.table = table;
        this.operation = operation;
    }

    /** Supply one or more records for insertion. */
    values(
        this: MutationQuery<Definition, Result, "insert">,
        values: MutationRow<Definition> | readonly MutationRow<Definition>[],
    ): MutationQuery<Definition, Result, "insert"> {
        this.records = Array.isArray(values) ? values : [values as MutationRow<Definition>];

        return this;
    }

    /** Supply properties for an update. */
    set(
        this: MutationQuery<Definition, Result, "update">,
        values: Partial<MutationRow<Definition>>,
    ): MutationQuery<Definition, Result, "update"> {
        this.changes = values;

        return this;
    }

    /** Filter the affected rows. */
    where(
        this: MutationQuery<Definition, Result, Operation> &
            (Operation extends "insert" ? never : unknown),
        predicate: SQL | undefined,
    ): MutationQuery<Definition, Result, Operation> {
        this.predicate = predicate;

        return this;
    }

    /** Ignore inserts that conflict with a unique key. */
    onConflictDoNothing(
        this: MutationQuery<Definition, Result, "insert">,
        options: { readonly target?: Column | readonly Column[]; readonly where?: SQL } = {},
    ): MutationQuery<Definition, Result, "insert"> {
        this.conflict = {
            action: "nothing",
            target: options.target === undefined ? undefined : columnList(options.target),
            targetWhere: options.where,
        };

        return this;
    }

    /** Update a row that conflicts with the selected unique key. */
    onConflictDoUpdate(
        this: MutationQuery<Definition, Result, "insert">,
        options: {
            readonly target: Column | readonly Column[];
            readonly set: Partial<MutationRow<Definition>>;
            readonly targetWhere?: SQL;
            readonly setWhere?: SQL;
        },
    ): MutationQuery<Definition, Result, "insert"> {
        this.conflict = { action: "update", ...options, target: columnList(options.target) };

        return this;
    }

    /** Return the changed application records. */
    returning(): MutationQuery<Definition, Select<Definition>[], Operation>;
    /** Return selected fields from changed records. */
    returning<Fields extends ReturningSelection>(
        fields: Fields,
    ): MutationQuery<Definition, SelectionResult<Fields>[], Operation>;
    returning(
        fields: ReturningSelection = this.table[TABLE].columns,
    ): MutationQuery<Definition, unknown[], Operation> {
        this.fields = fields;

        return this as unknown as MutationQuery<Definition, unknown[], Operation>;
    }

    /** Execute the mutation, reading changed rows back by key on SQLite instead of through RETURNING. */
    async execute(): Promise<Result> {
        const result = await this.driver.write((native) =>
            native.dialect === "sqlite" && this.fields !== undefined && this.#isReadBack()
                ? this.#writeThenRead(native)
                : this.#compile(native),
        );

        return (this.fields ? result : undefined) as Result;
    }

    /** Compile SQL and positional parameters without executing the mutation. */
    toSQL(): Query {
        return this.#compile(this.driver.native).toSQL();
    }

    /** Compile a reusable mutation with named placeholder values. */
    prepare(): PreparedQuery<Result> {
        const query = this.#compile(this.driver.native).prepare();
        const hasReturning = this.fields !== undefined;

        return {
            execute: async (parameters) => {
                // reject use after the enclosing transaction finishes
                this.driver.transaction?.assertActive();

                // compile again for a write that runs on another native database, such as an identified SQLite write
                const result = await this.driver.write((native) =>
                    (native === this.driver.native
                        ? query
                        : this.#compile(native).prepare()
                    ).execute(parameters),
                );

                return (hasReturning ? result : undefined) as Result;
            },
        };
    }

    /**
     * Decide whether SQLite reads changed rows back by key instead of through RETURNING, which Turso charges about 0.15 ms per statement.
     *
     * Updates and deletions read back by rowid; insertions need every record's key as a plain value, and upserts must conflict on the key and update every conflicting row.
     */
    #isReadBack(): boolean {
        // read updates and deletions back by rowid
        if (this.operation !== "insert") {
            return true;
        }

        // read insertions back by key, when every record names its key and an upsert conflicts on it
        const key = this.table[TABLE].key;
        const columns = this.table[TABLE].columns;
        const isKeyed =
            key.length > 0 &&
            this.records.length > 0 &&
            this.records.every((record) =>
                key.every((property) => {
                    const value = (record as Record<string, unknown>)[property];

                    return (
                        value !== undefined && !(value instanceof SQL) && !is(value, Placeholder)
                    );
                }),
            );
        const isKeyConflict =
            this.conflict?.action !== "update" ||
            (this.conflict.setWhere === undefined &&
                this.conflict.target.length === key.length &&
                this.conflict.target.every((column, index) => column === columns[key[index]!]));

        return isKeyed && isKeyConflict;
    }

    /** Write without RETURNING, then read the changed rows back within the write's transaction, which holds the write lock. */
    async #writeThenRead(
        native: Extract<NativeDatabase, { dialect: "sqlite" }>,
    ): Promise<unknown[]> {
        // read through the write's transaction, rows by rowid
        const database = native.database;
        const table = this.compiler.table(this.table) as SQLiteTable;
        const fields = selectFields(this.fields!, this.compiler) as SelectedFields;
        const rowid = sql`rowid`;
        const read = (where: SQL | undefined) =>
            database.select(fields).from(table).where(where) as unknown as Promise<unknown[]>;

        // read insertions back by key, skipping rows a conflict left in place, in insertion order
        if (this.operation === "insert") {
            const key = this.table[TABLE].key;
            const columns = this.table[TABLE].columns;
            const match = this.compiler.expression(
                or(
                    ...this.records.map((record) =>
                        and(
                            ...key.map((property) =>
                                eq(
                                    columns[property]!,
                                    (record as Record<string, unknown>)[property],
                                ),
                            ),
                        ),
                    ),
                )!,
            );
            const rowids = async () =>
                (
                    (await database.select({ rowid }).from(table).where(match)) as {
                        rowid: bigint;
                    }[]
                ).map((row) => row.rowid);
            const existing =
                this.conflict?.action === "nothing" ? new Set(await rowids()) : new Set();
            await this.#compile(native, false);
            const inserted = (await rowids())
                .filter((id) => !existing.has(id))
                .sort((left, right) => (left < right ? -1 : left > right ? 1 : 0));

            return inserted.length === 0 ? [] : read(inArray(rowid, inserted));
        }

        // select the rows the predicate matches, then write and read them by rowid
        const predicate = this.predicate && this.compiler.expression(this.predicate);
        const matched = (await database.select({ rowid }).from(table).where(predicate)) as {
            rowid: bigint;
        }[];
        if (matched.length === 0) {
            return [];
        }
        const selected = inArray(
            rowid,
            matched.map((row) => row.rowid),
        );

        // read deleted rows before they go, and updated rows after they change
        if (this.operation === "delete") {
            const deleted = await read(selected);
            await database.delete(table).where(selected);

            return deleted;
        }
        await database
            .update(table)
            .set(this.#values(this.changes) as never)
            .where(selected);

        return read(selected);
    }

    /** Translate expressions in a record while retaining column encoders on the native table. */
    #values(record: object): Record<string, unknown> {
        return Object.fromEntries(
            Object.entries(record).map(([property, value]) => [
                property,
                value instanceof SQL ? this.compiler.expression(value) : value,
            ]),
        );
    }

    /** Build the native mutation with its parameter encoders and result decoder, returning its fields unless told not to. */
    #compile(native: NativeDatabase, isReturning = true): DrizzleMutation {
        // reject use after the enclosing transaction finishes
        this.driver.transaction?.assertActive();

        // translate expressions while retaining column encoders on the native table
        const values = (record: object): Record<string, unknown> => this.#values(record);
        const predicate = this.predicate && this.compiler.expression(this.predicate);
        const fields = isReturning && this.fields && selectFields(this.fields, this.compiler);
        const conflict = this.conflict;

        // select the native builders once, then apply the common Drizzle operations
        const database = native.database as unknown as DrizzleDatabase;
        const table = this.compiler.table(this.table);
        let query: DrizzleMutation;

        // insert records with their conflict handling
        if (this.operation === "insert") {
            query = database.insert(table).values(this.records.map(values)).$dynamic();
        }
        // update the matching rows
        else if (this.operation === "update") {
            query = database.update(table).set(values(this.changes)).where(predicate);
        }
        // delete the matching rows
        else if (this.operation === "delete") {
            query = database.delete(table).where(predicate);
        }
        // reject other operations
        else {
            return assertNever(this.operation);
        }

        // ignore conflicting rows
        if (conflict?.action === "nothing") {
            query = query.onConflictDoNothing({
                target: conflict.target?.map((column) => this.compiler.column(column)),
                where: conflict.targetWhere && this.compiler.expression(conflict.targetWhere),
            });
        }
        // update conflicting rows
        else if (conflict?.action === "update") {
            query = query.onConflictDoUpdate({
                target: conflict.target.map((column) => this.compiler.column(column)),
                set: values(conflict.set),
                targetWhere: conflict.targetWhere && this.compiler.expression(conflict.targetWhere),
                setWhere: conflict.setWhere && this.compiler.expression(conflict.setWhere),
            });
        }

        return fields ? query.returning(fields) : query;
    }

    /** Execute and return the first changed row. */
    async get(
        this: MutationQuery<Definition, unknown[], Operation>,
    ): Promise<Result extends (infer Row)[] ? Row | undefined : never> {
        const rows = await this.execute();

        return rows[0] as Result extends (infer Row)[] ? Row | undefined : never;
    }

    /** Await mutation execution. */
    then<Fulfilled = Result, Rejected = never>(
        fulfilled?: ((value: Result) => Fulfilled | PromiseLike<Fulfilled>) | null,
        rejected?: ((reason: unknown) => Rejected | PromiseLike<Rejected>) | null,
    ): PromiseLike<Fulfilled | Rejected> {
        return this.execute().then(fulfilled, rejected);
    }
}

/** Normalize a single conflict key or a compound key. */
function columnList(columns: Column | readonly Column[]): readonly Column[] {
    return Array.isArray(columns) ? columns : [columns as Column];
}
