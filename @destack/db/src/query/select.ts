import { fill, render, sql, SQL, type SQLWrapper } from "../sql/index.ts";
import { type Select, TABLE, type Table } from "../table/table.ts";
import type { DatabaseDriver } from "../database/driver.ts";
import { Projection, type Selection, type SelectionResult } from "./selection.ts";

/** How a join keeps unmatched rows: inner keeps none, left keeps the source's. */
type JoinKind = "inner" | "left";

/** A joined table and its condition. */
interface Join {
    /** How unmatched rows stay. */
    readonly kind: JoinKind;
    /** The joined table. */
    readonly table: Table;
    /** The join condition. */
    readonly on: SQLWrapper;
}

/** What a selection reads, filters, groups, orders and limits. */
interface SelectState {
    /** The driver running the query. */
    readonly driver: DatabaseDriver;
    /** The source table. */
    readonly source: Table;
    /** The selected fields. */
    readonly fields: Selection;
    /** Whether joins add each joined table's columns to the fields. */
    readonly isAutomatic: boolean;
    /** Whether duplicate rows are removed. */
    readonly isDistinct: boolean;
    /** The joins in evaluation order. */
    readonly joins: readonly Join[];
    /** The row predicate. */
    readonly where: SQLWrapper | undefined;
    /** The grouping expressions. */
    readonly groups: readonly SQLWrapper[];
    /** The group predicate. */
    readonly having: SQLWrapper | undefined;
    /** The ordering expressions. */
    readonly order: readonly SQLWrapper[];
    /** The most rows returned. */
    readonly limit: number | undefined;
    /** The rows skipped. */
    readonly offset: number | undefined;
}

/** A typed selection, a new query for each step, awaited for its rows. */
export class SelectQuery<
    Result,
    Fields extends Selection = Selection,
    NullableTables extends string = never,
    Automatic extends boolean = false,
>
    implements PromiseLike<Result[]>, SQLWrapper
{
    /** The inferred types. */
    declare readonly _: {
        readonly result: Result;
        readonly fields: Fields;
        readonly nullable: NullableTables;
        readonly automatic: Automatic;
    };
    /** What the query reads, filters, groups, orders and limits. */
    readonly state: SelectState;

    /** Create the query. */
    constructor(state: SelectState) {
        this.state = state;
    }

    /** Filter rows before grouping. */
    where(
        predicate: SQLWrapper | undefined,
    ): SelectQuery<Result, Fields, NullableTables, Automatic> {
        return new SelectQuery({ ...this.state, where: predicate });
    }

    /** Include matching rows from another table. */
    innerJoin<Joined extends Table>(
        table: Joined,
        on: SQLWrapper,
    ): SelectQuery<
        SelectionResult<JoinFields<Fields, Joined, Automatic>, NullableTables>,
        JoinFields<Fields, Joined, Automatic>,
        NullableTables,
        Automatic
    > {
        return new SelectQuery(this.#join("inner", table, on));
    }

    /** Include matching or null rows from another table. */
    leftJoin<Joined extends Table>(
        table: Joined,
        on: SQLWrapper,
    ): SelectQuery<
        SelectionResult<
            JoinFields<Fields, Joined, Automatic>,
            NullableTables | Joined[typeof TABLE]["name"]
        >,
        JoinFields<Fields, Joined, Automatic>,
        NullableTables | Joined[typeof TABLE]["name"],
        Automatic
    > {
        return new SelectQuery(this.#join("left", table, on));
    }

    /** Group rows by expressions. */
    groupBy(...expressions: SQLWrapper[]): SelectQuery<Result, Fields, NullableTables, Automatic> {
        return new SelectQuery({ ...this.state, groups: expressions });
    }

    /** Filter grouped rows. */
    having(
        predicate: SQLWrapper | undefined,
    ): SelectQuery<Result, Fields, NullableTables, Automatic> {
        return new SelectQuery({ ...this.state, having: predicate });
    }

    /** Order rows by expressions. */
    orderBy(...expressions: SQLWrapper[]): SelectQuery<Result, Fields, NullableTables, Automatic> {
        return new SelectQuery({ ...this.state, order: expressions });
    }

    /** Return at most a number of rows. */
    limit(count: number): SelectQuery<Result, Fields, NullableTables, Automatic> {
        return new SelectQuery({ ...this.state, limit: count });
    }

    /** Skip a number of rows. */
    offset(count: number): SelectQuery<Result, Fields, NullableTables, Automatic> {
        return new SelectQuery({ ...this.state, offset: count });
    }

    /** Read the first row, if any. */
    async get(): Promise<Result | undefined> {
        const [row] = await this.limit(1).execute();

        return row;
    }

    /** Read every row. */
    async execute(): Promise<Result[]>;
    /**
     * Read every row, decoded by the query's selection.
     *
     * @construct the rows decode through the projection of this query's fields and joins, from which the builder computed Result.
     */
    async execute(): Promise<unknown[]> {
        // run the query and decode its rows, nullable joined groups missing as null
        const projection = this.#projection();
        const rows = await this.state.driver.values(
            fill(
                render(
                    this.#sql(projection),
                    this.state.driver.dialect,
                    this.state.driver.state.namespace,
                ),
            ),
        );
        const nullable = new Set(
            this.state.joins
                .filter((join) => join.kind === "left")
                .map((join) => join.table[TABLE].sqlName),
        );

        return projection.decode(rows, this.state.driver.dialect, nullable);
    }

    /** Embed the query as a subquery, such as in `inArray` or `exists`. */
    getSQL(): SQL {
        return this.#sql(this.#projection());
    }

    /** Await the rows. */
    then<Fulfilled = Result[], Rejected = never>(
        fulfilled?: ((value: Result[]) => Fulfilled | PromiseLike<Fulfilled>) | null,
        rejected?: ((reason: unknown) => Rejected | PromiseLike<Rejected>) | null,
    ): PromiseLike<Fulfilled | Rejected> {
        return this.execute().then(fulfilled, rejected);
    }

    /** Add a join, and the joined table's columns to an automatic selection. */
    #join(kind: JoinKind, table: Table, on: SQLWrapper): SelectState {
        const fields = this.state.isAutomatic
            ? { ...this.state.fields, [table[TABLE].name]: table }
            : this.state.fields;

        return { ...this.state, fields, joins: [...this.state.joins, { kind, table, on }] };
    }

    /** Flatten the selected fields, the source's columns for an automatic selection without joins. */
    #projection(): Projection {
        const isBare = this.state.isAutomatic && this.state.joins.length === 0;

        return new Projection(isBare ? this.state.source[TABLE].columns : this.state.fields);
    }

    /** Write the query as SQL. */
    #sql(projection: Projection): SQL {
        // select the fields from the source
        const state = this.state;
        const parts: SQL[] = [
            sql`SELECT ${sql.raw(state.isDistinct ? "DISTINCT " : "")}${projection.sql()} FROM ${from(state.source)}`,
        ];

        // join, filter, group and order
        for (const join of state.joins) {
            parts.push(
                sql` ${sql.raw(join.kind === "inner" ? "INNER JOIN" : "LEFT JOIN")} ${from(join.table)} ON ${join.on}`,
            );
        }
        if (state.where !== undefined) {
            parts.push(sql` WHERE ${state.where}`);
        }
        if (state.groups.length > 0) {
            parts.push(sql` GROUP BY ${sql.join(state.groups, sql.raw(", "))}`);
        }
        if (state.having !== undefined) {
            parts.push(sql` HAVING ${state.having}`);
        }
        if (state.order.length > 0) {
            parts.push(sql` ORDER BY ${sql.join(state.order, sql.raw(", "))}`);
        }

        // limit and skip, SQLite requiring a limit before an offset
        if (state.limit !== undefined) {
            parts.push(sql` LIMIT ${state.limit}`);
        } else if (state.offset !== undefined && state.driver.dialect === "sqlite") {
            parts.push(sql` LIMIT -1`);
        }
        if (state.offset !== undefined) {
            parts.push(sql` OFFSET ${state.offset}`);
        }

        return sql.join(parts);
    }
}

/** A selection awaiting its source table. */
export class SelectBuilder<Fields extends Selection | undefined = undefined> {
    /** The driver running the query. */
    readonly driver: DatabaseDriver;
    /** The explicitly selected fields, absent to select the source's records. */
    readonly fields: Fields;
    /** Whether duplicate rows are removed. */
    readonly isDistinct: boolean;

    /** Create the builder. */
    constructor(driver: DatabaseDriver, fields: Fields, isDistinct: boolean) {
        this.driver = driver;
        this.fields = fields;
        this.isDistinct = isDistinct;
    }

    /** Select rows from a table. */
    from<Source extends Table>(
        table: Source,
    ): SelectQuery<
        Fields extends Selection ? SelectionResult<Fields> : Select<Source>,
        Fields extends Selection ? Fields : Record<Source[typeof TABLE]["name"], Source>,
        never,
        Fields extends Selection ? false : true
    > {
        return new SelectQuery({
            driver: this.driver,
            source: table,
            fields: this.fields ?? { [table[TABLE].name]: table },
            isAutomatic: this.fields === undefined,
            isDistinct: this.isDistinct,
            joins: [],
            where: undefined,
            groups: [],
            having: undefined,
            order: [],
            limit: undefined,
            offset: undefined,
        });
    }
}

/** Render a table where a query reads it: an alias after the table it renames. */
export function from(table: Table): SQL {
    const source = table[TABLE].source;

    return source === undefined ? sql`${table}` : sql`${source} AS ${table}`;
}

/** Add a joined table to an automatic selection. */
type JoinFields<
    Fields extends Selection,
    Joined extends Table,
    Automatic extends boolean,
> = Automatic extends true ? Fields & Record<Joined[typeof TABLE]["name"], Joined> : Fields;
