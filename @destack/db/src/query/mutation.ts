import { type Placeholder, type Query, SQL } from "drizzle-orm";
import type {
    SelectedFieldsFlat as SQLiteSelection,
    SQLiteColumn,
    SQLiteTable,
} from "drizzle-orm/sqlite-core";
import type {
    PgColumn,
    PgTable,
    SelectedFieldsFlat as PostgresSelection,
} from "drizzle-orm/pg-core";
import { type DatabaseDriver } from "../database/driver.ts";
import type { SchemaCompiler } from "../dialect/compiler.ts";
import type { Column } from "../table/column.ts";
import { type Insert, type Select, TABLE, type Table } from "../table/table.ts";
import { selectFields, type SelectionResult } from "./selection.ts";
import type { PreparedQuery } from "./select.ts";
import { assertNever } from "../error/error.ts";

/** Values supplied to an insert or update. */
export type MutationValues<Definition extends Table> = {
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
    /** The physical connection. */
    readonly connection: DatabaseDriver;
    /** The materialized schema. */
    readonly schema: SchemaCompiler;
    /** The affected logical table. */
    readonly table: Definition;
    /** The SQL operation. */
    readonly operation: Operation;
    /** The inserted application records. */
    records: readonly MutationValues<Definition>[] = [];
    /** The updated application properties. */
    changes: Partial<MutationValues<Definition>> = {};
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
              readonly set: Partial<MutationValues<Definition>>;
          };

    /** Retain a mutation and its connection. */
    constructor(
        connection: DatabaseDriver,
        schema: SchemaCompiler,
        table: Definition,
        operation: Operation,
    ) {
        this.connection = connection;
        this.schema = schema;
        this.table = table;
        this.operation = operation;
    }

    /** Supply one or more records for insertion. */
    values(
        this: MutationQuery<Definition, Result, "insert">,
        values: MutationValues<Definition> | readonly MutationValues<Definition>[],
    ): MutationQuery<Definition, Result, "insert"> {
        this.records = Array.isArray(values) ? values : [values as MutationValues<Definition>];

        return this;
    }

    /** Supply properties for an update. */
    set(
        this: MutationQuery<Definition, Result, "update">,
        values: Partial<MutationValues<Definition>>,
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
            readonly set: Partial<MutationValues<Definition>>;
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

    /** Execute the mutation. */
    async execute(): Promise<Result> {
        const result = await this.connection.run(() => this.compile());

        return (this.fields ? result : undefined) as Result;
    }

    /** Compile SQL and positional parameters without executing the mutation. */
    toSQL(): Query {
        return this.compile().toSQL();
    }

    /** Compile a reusable mutation with named placeholder values. */
    prepare(): PreparedQuery<Result> {
        const query = this.compile().prepare();
        const returnsRows = this.fields !== undefined;

        return {
            execute: async (parameters) => {
                this.connection.transaction?.assertActive();
                const result = await this.connection.run(() => query.execute(parameters));

                return (returnsRows ? result : undefined) as Result;
            },
        };
    }

    /** Build the native mutation with its parameter encoders and result decoder. */
    private compile() {
        this.connection.transaction?.assertActive();

        // translate expressions while retaining column encoders on the native table
        const values = (record: object): Record<string, unknown> =>
            Object.fromEntries(
                Object.entries(record).map(([property, value]) => [
                    property,
                    value instanceof SQL ? this.schema.expression(value) : value,
                ]),
            );
        const predicate = this.predicate && this.schema.expression(this.predicate);
        const fields = this.fields && selectFields(this.fields, this.schema);
        const conflict = this.conflict;

        // apply SQLite mutations through its native query builder
        if (this.connection.native.dialect === "sqlite") {
            const database = this.connection.native.database;
            const table = this.schema.table(this.table) as SQLiteTable;
            if (this.operation === "insert") {
                let query = database.insert(table).values(this.records.map(values)).$dynamic();
                if (conflict?.action === "nothing") {
                    query = query.onConflictDoNothing({
                        target: conflict.target?.map(
                            (column) => this.schema.column(column) as SQLiteColumn,
                        ),
                        where: conflict.targetWhere && this.schema.expression(conflict.targetWhere),
                    });
                } else if (conflict?.action === "update") {
                    query = query.onConflictDoUpdate({
                        target: conflict.target.map(
                            (column) => this.schema.column(column) as SQLiteColumn,
                        ),
                        set: values(conflict.set),
                        targetWhere:
                            conflict.targetWhere && this.schema.expression(conflict.targetWhere),
                        setWhere: conflict.setWhere && this.schema.expression(conflict.setWhere),
                    });
                }
                if (fields) {
                    return query.returning(fields as SQLiteSelection);
                }
                return query;
            } else if (this.operation === "update") {
                const query = database.update(table).set(values(this.changes)).where(predicate);
                if (fields) {
                    return query.returning(fields as SQLiteSelection);
                }
                return query;
            } else if (this.operation === "delete") {
                const query = database.delete(table).where(predicate);
                if (fields) {
                    return query.returning(fields as SQLiteSelection);
                }
                return query;
            } else {
                return assertNever(this.operation);
            }
        } // apply PostgreSQL mutations through its native query builder
        else if (this.connection.native.dialect === "postgresql") {
            const database = this.connection.native.database;
            const table = this.schema.table(this.table) as PgTable;

            if (this.operation === "insert") {
                let query = database.insert(table).values(this.records.map(values)).$dynamic();
                if (conflict?.action === "nothing") {
                    query = query.onConflictDoNothing({
                        target: conflict.target?.map(
                            (column) => this.schema.column(column) as PgColumn,
                        ),
                        where: conflict.targetWhere && this.schema.expression(conflict.targetWhere),
                    });
                } else if (conflict?.action === "update") {
                    query = query.onConflictDoUpdate({
                        target: conflict.target.map(
                            (column) => this.schema.column(column) as PgColumn,
                        ),
                        set: values(conflict.set),
                        targetWhere:
                            conflict.targetWhere && this.schema.expression(conflict.targetWhere),
                        setWhere: conflict.setWhere && this.schema.expression(conflict.setWhere),
                    });
                }
                if (fields) {
                    return query.returning(fields as PostgresSelection);
                }
                return query;
            } else if (this.operation === "update") {
                const query = database.update(table).set(values(this.changes)).where(predicate);
                if (fields) {
                    return query.returning(fields as PostgresSelection);
                }
                return query;
            } else if (this.operation === "delete") {
                const query = database.delete(table).where(predicate);
                if (fields) {
                    return query.returning(fields as PostgresSelection);
                }
                return query;
            } else {
                return assertNever(this.operation);
            }
        } else {
            return assertNever(this.connection.native);
        }
    }

    /** Execute a mutation that does not return rows. */
    run(): Promise<Result> {
        return this.execute();
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
