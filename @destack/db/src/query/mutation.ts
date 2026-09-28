import { is, Placeholder, type Query, SQL, sql } from "drizzle-orm";
import type { DatabaseDriver, NativeDatabase } from "../database/driver.ts";
import type { SchemaCompiler } from "../dialect/compiler.ts";
import type { DrizzleDatabase, DrizzleMutation } from "../dialect/drizzle.ts";
import { Column } from "../table/column.ts";
import { type Insert, type Select, TABLE, type Table } from "../table/table.ts";
import { selectFields, type SelectionResult } from "./selection.ts";
import type { PreparedQuery } from "./select.ts";
import { assertNever } from "../error/error.ts";

/** A prepared Drizzle mutation. */
type DrizzlePrepared = { execute(values: Record<string, unknown>): Promise<unknown> };

/** The rendered plain inserts of each native database, by shape. */
const RENDERED = new WeakMap<object, Map<string, DrizzlePrepared>>();

/** The values of an insert or update. */
export type MutationRow<Definition extends Table> = {
    [Property in keyof Insert<Definition>]: Insert<Definition>[Property] | SQL | Placeholder;
};

/** The fields a mutation returns. */
export interface ReturningSelection {
    /** A changed column or expression. */
    readonly [property: string]: Column | SQL | SQL.Aliased;
}

/** A typed insertion, update or deletion. */
export class MutationQuery<
    Definition extends Table,
    Result = void,
    Operation extends "insert" | "update" | "delete" = "insert" | "update" | "delete",
> implements PromiseLike<Result> {
    /** The native database driver. */
    readonly driver: DatabaseDriver;
    /** The table compiler. */
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
    /** The insert conflict handling. */
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

    /** Create the mutation. */
    constructor(
        driver: DatabaseDriver,
        compiler: SchemaCompiler,
        table: Definition,
        operation: Operation,
    ) {
        // keep the driver, compiler, table and operation
        this.driver = driver;
        this.compiler = compiler;
        this.table = table;
        this.operation = operation;
    }

    /** Supply records for insertion. */
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

    /** Update a row that conflicts with a unique key. */
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

    /** Execute the mutation, returning the changed rows when asked. */
    async execute(): Promise<Result> {
        const result = await this.driver.write((native) => {
            const shape = this.#shape();

            return shape === undefined
                ? this.#compile(native)
                : this.#rendered(native, shape).execute(this.records[0] as Record<string, unknown>);
        });

        return (this.fields ? result : undefined) as Result;
    }

    /** Name the shape of a plain single-record insert, absent for other mutations. */
    #shape(): string | undefined {
        // render only plain single-record inserts
        const [record] = this.records;
        const fields = this.fields === undefined ? [] : Object.entries(this.fields);
        const isPlain =
            this.operation === "insert" &&
            this.records.length === 1 &&
            this.conflict === undefined &&
            Object.values(record!).every(
                (value) => !(value instanceof SQL) && !is(value, Placeholder),
            ) &&
            fields.every(([, field]) => field instanceof Column);

        return isPlain
            ? `${this.table[TABLE].sqlName}:${defined(record!).join(",")}:${fields.map(([name, field]) => `${name}=${(field as Column).definition.name}`).join(",")}`
            : undefined;
    }

    /** Take the rendered statement of a shape, rendering it the first time. */
    #rendered(native: NativeDatabase, shape: string): DrizzlePrepared {
        // reuse the statement of the shape
        const statements = RENDERED.get(native.database) ?? new Map<string, DrizzlePrepared>();
        RENDERED.set(native.database, statements);
        const known = statements.get(shape);
        if (known !== undefined) {
            return known;
        }

        // render with a placeholder per value
        const placeholders = Object.fromEntries(
            defined(this.records[0]!).map((property) => [property, sql.placeholder(property)]),
        );
        const rendered = new MutationQuery(this.driver, this.compiler, this.table, "insert").values(
            placeholders as MutationRow<Definition>,
        ).#withFields(this.fields).#compile(native).prepare() as DrizzlePrepared;
        statements.set(shape, rendered);

        return rendered;
    }

    /** Copy the returned fields onto another query. */
    #withFields(fields: ReturningSelection | undefined): this {
        this.fields = fields;

        return this;
    }

    /** Render the SQL and parameters. */
    toSQL(): Query {
        return this.#compile(this.driver.native).toSQL();
    }

    /** Compile a reusable mutation. */
    prepare(): PreparedQuery<Result> {
        const query = this.#compile(this.driver.native).prepare();
        const hasReturning = this.fields !== undefined;

        return {
            execute: async (parameters) => {
                // reject use after the enclosing transaction
                this.driver.transaction?.assertActive();

                // compile on the native database the write runs on
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

    /** Translate expressions in a record. */
    #values(record: object): Record<string, unknown> {
        return Object.fromEntries(
            Object.entries(record).map(([property, value]) => [
                property,
                value instanceof SQL ? this.compiler.expression(value) : value,
            ]),
        );
    }

    /** Build the native mutation. */
    #compile(native: NativeDatabase, isReturning = true): DrizzleMutation {
        // reject use after the enclosing transaction
        this.driver.transaction?.assertActive();

        // translate the values
        const values = (record: object): Record<string, unknown> => this.#values(record);
        const predicate = this.predicate && this.compiler.expression(this.predicate);
        const fields = isReturning && this.fields && selectFields(this.fields, this.compiler);
        const conflict = this.conflict;

        // build the native operation
        const database = native.database as unknown as DrizzleDatabase;
        const table = this.compiler.table(this.table);
        let query: DrizzleMutation;

        // insert records
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

    /** Await execution. */
    then<Fulfilled = Result, Rejected = never>(
        fulfilled?: ((value: Result) => Fulfilled | PromiseLike<Fulfilled>) | null,
        rejected?: ((reason: unknown) => Rejected | PromiseLike<Rejected>) | null,
    ): PromiseLike<Fulfilled | Rejected> {
        return this.execute().then(fulfilled, rejected);
    }
}

/** Normalize a conflict key. */
function columnList(columns: Column | readonly Column[]): readonly Column[] {
    return Array.isArray(columns) ? columns : [columns as Column];
}

/** List a record's defined properties. */
function defined(record: object): string[] {
    return Object.entries(record).flatMap(([property, value]) =>
        value === undefined ? [] : [property],
    );
}
