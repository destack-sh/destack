import {
    Alias,
    fill,
    isSQLWrapper,
    Parameter,
    Placeholder,
    render,
    SQL,
    sql,
    type SQLWrapper,
} from "../sql/index.ts";
import type { DatabaseDriver } from "../database/driver.ts";
import { Column, type ColumnValue } from "../table/column.ts";
import { type Insert, type Select, TABLE, type Table } from "../table/table.ts";
import { Projection, type SelectionResult } from "./selection.ts";
import { DatabaseError } from "../error/error.ts";

/** The values of an insert or update: application values, fragments or placeholders, undefined taking the default. */
export type InsertValue<Definition extends Table> = {
    [Property in keyof Insert<Definition>]:
        | Insert<Definition>[Property]
        | SQL
        | Placeholder
        | undefined;
};

/** The values one inserted record or one update writes, by property. */
type WriteRecord = Readonly<Record<string, ColumnValue | SQL | Placeholder | undefined>>;

/** The fields a mutation returns. */
export interface ReturningSelection {
    /** A changed column or expression. */
    readonly [property: string]: Column | SQL | Alias;
}

/** How an insert treats rows that conflict with a unique key. */
type Conflict<Definition extends Table> =
    | {
          /** Skip the conflicting rows. */
          readonly action: "nothing";
          /** The unique key, any key when absent. */
          readonly target: readonly Column[] | undefined;
          /** The predicate of a partial unique key. */
          readonly targetWhere: SQLWrapper | undefined;
      }
    | {
          /** Update the conflicting rows. */
          readonly action: "update";
          /** The unique key. */
          readonly target: readonly Column[];
          /** The predicate of a partial unique key. */
          readonly targetWhere: SQLWrapper | undefined;
          /** The predicate restricting which conflicting rows update. */
          readonly setWhere: SQLWrapper | undefined;
          /** The values written to conflicting rows. */
          readonly set: Partial<InsertValue<Definition>>;
      };

/** What a mutation writes, to which rows, and what it returns. */
interface MutationState<Definition extends Table> {
    /** The driver running the mutation. */
    readonly driver: DatabaseDriver;
    /** The changed table. */
    readonly table: Definition;
    /** The SQL operation. */
    readonly operation: "insert" | "update" | "delete";
    /** The inserted records by property. */
    readonly records: readonly WriteRecord[];
    /** The updated values by property. */
    readonly changes: WriteRecord;
    /** The changed rows of an update or deletion. */
    readonly where: SQLWrapper | undefined;
    /** The insert's conflict handling. */
    readonly conflict: Conflict<Definition> | undefined;
    /** The returned fields, absent to return nothing. */
    readonly returning: ReturningSelection | undefined;
}

/** A typed insertion, update or deletion, a new mutation for each step, awaited for its result. */
export class MutationQuery<
    Definition extends Table,
    Result = void,
    Operation extends "insert" | "update" | "delete" = "insert" | "update" | "delete",
> implements PromiseLike<Result> {
    /** The inferred types. */
    declare readonly _: { readonly result: Result; readonly operation: Operation };
    /** What the mutation writes, to which rows, and what it returns. */
    readonly state: MutationState<Definition>;

    /** Create the mutation. */
    constructor(state: MutationState<Definition>) {
        this.state = state;
    }

    /** Start a mutation of a table. */
    static of<Definition extends Table, Operation extends "insert" | "update" | "delete">(
        driver: DatabaseDriver,
        table: Definition,
        operation: Operation,
    ): MutationQuery<Definition, void, Operation> {
        return new MutationQuery({
            driver,
            table,
            operation,
            records: [],
            changes: {},
            where: undefined,
            conflict: undefined,
            returning: undefined,
        });
    }

    /** Supply records for insertion. */
    values(
        this: MutationQuery<Definition, Result, "insert">,
        values: InsertValue<Definition> | readonly InsertValue<Definition>[],
    ): MutationQuery<Definition, Result, "insert"> {
        const records: readonly WriteRecord[] = Array.isArray(values) ? values : [values];

        return new MutationQuery({ ...this.state, records });
    }

    /** Supply properties for an update. */
    set(
        this: MutationQuery<Definition, Result, "update">,
        values: Partial<InsertValue<Definition>>,
    ): MutationQuery<Definition, Result, "update"> {
        return new MutationQuery({ ...this.state, changes: values });
    }

    /** Filter the changed rows of an update or deletion. */
    where(
        this: MutationQuery<Definition, Result, Operation> &
            (Operation extends "insert" ? never : unknown),
        predicate: SQLWrapper | undefined,
    ): MutationQuery<Definition, Result, Operation> {
        return new MutationQuery({ ...this.state, where: predicate });
    }

    /** Skip inserts that conflict with a unique key. */
    onConflictDoNothing(
        this: MutationQuery<Definition, Result, "insert">,
        options: { readonly target?: Column | readonly Column[]; readonly where?: SQLWrapper } = {},
    ): MutationQuery<Definition, Result, "insert"> {
        const target = options.target === undefined ? undefined : columns(options.target);

        return new MutationQuery({
            ...this.state,
            conflict: { action: "nothing", target, targetWhere: options.where },
        });
    }

    /** Update the rows an insert conflicts with on a unique key. */
    onConflictDoUpdate(
        this: MutationQuery<Definition, Result, "insert">,
        options: {
            readonly target: Column | readonly Column[];
            readonly set: Partial<InsertValue<Definition>>;
            readonly targetWhere?: SQLWrapper;
            readonly setWhere?: SQLWrapper;
        },
    ): MutationQuery<Definition, Result, "insert"> {
        return new MutationQuery({
            ...this.state,
            conflict: {
                action: "update",
                target: columns(options.target),
                targetWhere: options.targetWhere,
                setWhere: options.setWhere,
                set: options.set,
            },
        });
    }

    /** Return the changed records. */
    returning(): MutationQuery<Definition, Select<Definition>[], Operation>;
    /** Return selected fields of the changed records. */
    returning<Fields extends ReturningSelection>(
        fields: Fields,
    ): MutationQuery<Definition, SelectionResult<Fields>[], Operation>;
    /**
     * Return fields of the changed records, whose signatures above type them by the fields.
     *
     * @construct the returned rows decode through the selected fields, or every column, which is how the signatures above map the result.
     */
    returning(fields?: ReturningSelection): MutationQuery<Definition, unknown[], Operation> {
        return new MutationQuery({
            ...this.state,
            returning: fields ?? this.state.table[TABLE].columns,
        });
    }

    /** Run the mutation, returning the changed rows when asked. */
    async execute(): Promise<Result>;
    /**
     * Run the mutation, whose signature above types its result by the returned fields.
     *
     * @construct the rows decode through the projection of the returned fields, from which the builder computed Result.
     */
    async execute(): Promise<unknown> {
        // render the mutation for the driver's dialect
        const state = this.state;
        const statement = fill(render(this.#sql(), state.driver.dialect));

        // run without rows, or read and decode the returned rows
        if (state.returning === undefined) {
            await state.driver.execute(statement);

            return undefined;
        }
        const projection = new Projection(unqualified(state.returning));
        const rows = await state.driver.write((session) =>
            session.values(statement.text, statement.parameters),
        );

        return projection.decode(rows, state.driver.dialect, new Set());
    }

    /** Run the mutation and return the first changed row. */
    async get<Row>(this: MutationQuery<Definition, Row[], Operation>): Promise<Row | undefined> {
        const [row] = await this.execute();

        return row;
    }

    /** Await the result. */
    then<Fulfilled = Result, Rejected = never>(
        fulfilled?: ((value: Result) => Fulfilled | PromiseLike<Fulfilled>) | null,
        rejected?: ((reason: unknown) => Rejected | PromiseLike<Rejected>) | null,
    ): PromiseLike<Fulfilled | Rejected> {
        return this.execute().then(fulfilled, rejected);
    }

    /** Write the mutation as SQL. */
    #sql(): SQL {
        // write the statement in parts
        const state = this.state;
        const table = state.table;
        const parts: SQL[] = [];

        // insert every written column, defaulting the columns a record leaves out and refusing properties of no column
        if (state.operation === "insert") {
            parts.push(insertSQL(table, state.records));
        }
        // update the matching rows
        else if (state.operation === "update") {
            parts.push(sql`UPDATE ${table} SET ${assignments(table, state.changes)}`);
        }
        // delete the matching rows
        else {
            parts.push(sql`DELETE FROM ${table}`);
        }
        if (state.where !== undefined) {
            parts.push(sql` WHERE ${state.where}`);
        }

        // skip or update conflicting rows
        if (state.conflict !== undefined) {
            parts.push(conflictSQL(table, state.conflict));
        }

        // return the changed rows' fields
        if (state.returning !== undefined) {
            parts.push(sql` RETURNING ${new Projection(unqualified(state.returning)).sql()}`);
        }

        return sql.join(parts);
    }
}

/** Write an insert of every column some record sets. */
function insertSQL(table: Table, records: readonly WriteRecord[]): SQL {
    // require records of known columns
    if (records.length === 0) {
        throw new DatabaseError("INVALID_QUERY", `insert into ${table[TABLE].name} has no records`);
    }
    for (const property of new Set(records.flatMap((record) => Object.keys(record)))) {
        if (!Object.hasOwn(table[TABLE].columns, property)) {
            throw new DatabaseError(
                "INVALID_QUERY",
                `${table[TABLE].name} has no column ${property}`,
            );
        }
    }

    // write the columns some record sets
    const written = table[TABLE].entries.filter(
        ([property, column]) =>
            column.definition.generated === undefined &&
            records.some((record) => record[property] !== undefined),
    );
    const rows = records.map(
        (record) =>
            sql`(${sql.join(
                written.map(([property, column]) => valueOf(column, record[property])),
                sql.raw(", "),
            )})`,
    );

    return sql`INSERT INTO ${table} (${sql.join(
        written.map(([, column]) => sql.identifier(column.definition.name)),
        sql.raw(", "),
    )}) VALUES ${sql.join(rows, sql.raw(", "))}`;
}

/** Write an insert's clause skipping or updating the rows that conflict with a unique key. */
function conflictSQL<Definition extends Table>(table: Table, conflict: Conflict<Definition>): SQL {
    const target =
        conflict.target === undefined
            ? sql.empty()
            : sql` (${sql.join(
                  conflict.target.map((column) => sql.identifier(column.definition.name)),
                  sql.raw(", "),
              )})${conflict.targetWhere === undefined ? sql.empty() : sql` WHERE ${conflict.targetWhere}`}`;

    return conflict.action === "nothing"
        ? sql` ON CONFLICT${target} DO NOTHING`
        : sql` ON CONFLICT${target} DO UPDATE SET ${assignments(table, conflict.set)}${
              conflict.setWhere === undefined ? sql.empty() : sql` WHERE ${conflict.setWhere}`
          }`;
}

/** Write `column = value` for each set property. */
function assignments(table: Table, changes: WriteRecord): SQL {
    const set = Object.entries(changes).flatMap(([property, value]) => {
        const column = table[TABLE].columns[property];
        if (column === undefined) {
            throw new DatabaseError(
                "INVALID_QUERY",
                `${table[TABLE].name} has no column ${property}`,
            );
        }

        return value === undefined
            ? []
            : [sql`${sql.identifier(column.definition.name)} = ${valueOf(column, value)}`];
    });
    if (set.length === 0) {
        throw new DatabaseError("INVALID_QUERY", `update of ${table[TABLE].name} sets no column`);
    }

    return sql.join(set, sql.raw(", "));
}

/** Write a value: fragments and placeholders as they are, a missing one as the column's default, others bound through the column. */
function valueOf(column: Column, value: unknown): SQLWrapper {
    // keep fragments and placeholders
    if (isSQLWrapper(value)) {
        return value;
    } else if (value instanceof Placeholder) {
        return new SQL([value]);
    }

    // default a value an insert leaves out, and bind the others
    const fallback = column.definition.default;
    if (value !== undefined || fallback === undefined) {
        return new SQL([new Parameter(value ?? null, column)]);
    }

    return fallback instanceof SQL ? fallback : new SQL([new Parameter(fallback, column)]);
}

/** Write returned columns without their table, as SQLite's RETURNING requires. */
function unqualified(fields: ReturningSelection): ReturningSelection {
    return Object.fromEntries(
        Object.entries(fields).map(([property, field]) => [
            property,
            field instanceof Column
                ? sql.identifier(field.definition.name).mapWith(field.definition)
                : field,
        ]),
    );
}

/** Read a conflict key as a list. */
function columns(target: Column | readonly Column[]): readonly Column[] {
    return target instanceof Column ? [target] : target;
}
