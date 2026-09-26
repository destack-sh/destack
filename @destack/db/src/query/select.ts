import { assertNever } from "../error/error.ts";
import { type Query, SQL, type SQLWrapper, Subquery, WithSubquery } from "drizzle-orm";
import type { SQLiteTable } from "drizzle-orm/sqlite-core";
import type * as sqlite from "drizzle-orm/sqlite-core";
import type { PgTable } from "drizzle-orm/pg-core";
import type * as postgres from "drizzle-orm/pg-core";
import { type Select, TABLE, type Table } from "../table/table.ts";
import { Column } from "../table/column.ts";
import { type DatabaseDriver } from "../database/driver.ts";
import type { SchemaCompiler } from "../dialect/compiler.ts";
import type { DrizzleSelect } from "../dialect/drizzle.ts";
import { selectFields, type Selection, type SelectionResult } from "./selection.ts";
import type { SOURCE } from "./selection.ts";

/** A typed selection from one table and its joins. */
export class SelectQuery<
    Result,
    Fields extends Selection = Selection,
    NullableTables extends string = never,
    Automatic extends boolean = false,
>
    implements PromiseLike<Result[]>, SQLWrapper
{
    /** Drizzle's inferred selection and result types. */
    declare readonly _: { selectedFields: Selection; result: Result[] };
    /** The native database, its connection state and its transaction. */
    readonly driver: DatabaseDriver;
    /** The compiler binding logical tables to the native dialect. */
    readonly compiler: SchemaCompiler;
    /** The query's source table. */
    readonly table: QuerySource;
    /** The selected fields. */
    readonly fields: Fields;
    /** Whether duplicate rows are removed. */
    readonly isDistinct: boolean;
    /** Common table expressions included in this query. */
    readonly withList: readonly WithSubquery[];
    /** Whether joins include every column of each joined table. */
    readonly isAutomatic: Automatic;
    /** The joins in evaluation order. */
    readonly joins: {
        readonly kind: "inner" | "left" | "right" | "full" | "cross";
        readonly table: QuerySource;
        readonly on?: SQL;
    }[] = [];
    /** The row predicate. */
    predicate?: SQL;
    /** The grouping expressions. */
    groups: SQLWrapper[] = [];
    /** The group predicate. */
    groupPredicate?: SQL;
    /** The ordering expressions. */
    order: SQLWrapper[] = [];
    /** The maximum returned row count. */
    count?: number;
    /** The number of rows skipped. */
    skip?: number;

    /** Retain the selected fields and connection. */
    constructor(
        driver: DatabaseDriver,
        compiler: SchemaCompiler,
        table: QuerySource,
        fields: Fields,
        options: {
            readonly isDistinct: boolean;
            readonly isAutomatic: Automatic;
            readonly withList: readonly WithSubquery[];
        },
    ) {
        // retain the query definition
        this.withList = options.withList;
        this.isAutomatic = options.isAutomatic;
        this.driver = driver;
        this.compiler = compiler;
        this.table = table;
        this.fields = fields;
        this.isDistinct = options.isDistinct;
    }

    /** Filter rows before grouping. */
    where(predicate: SQL | undefined): this {
        this.predicate = predicate;

        return this;
    }

    /** Include matching rows from another table. */
    innerJoin<Joined extends QuerySource>(
        table: Joined,
        on: SQL,
    ): SelectQuery<
        SelectionResult<JoinFields<Fields, Joined, Automatic>, NullableTables>,
        JoinFields<Fields, Joined, Automatic>,
        NullableTables,
        Automatic
    > {
        this.#join("inner", table, on);

        return this as unknown as SelectQuery<
            SelectionResult<JoinFields<Fields, Joined, Automatic>, NullableTables>,
            JoinFields<Fields, Joined, Automatic>,
            NullableTables,
            Automatic
        >;
    }

    /** Include matching rows or NULL values from another table. */
    leftJoin<Joined extends QuerySource>(
        table: Joined,
        on: SQL,
    ): SelectQuery<
        SelectionResult<JoinFields<Fields, Joined, Automatic>, NullableTables | SourceName<Joined>>,
        JoinFields<Fields, Joined, Automatic>,
        NullableTables | SourceName<Joined>,
        Automatic
    > {
        this.#join("left", table, on);

        return this as unknown as SelectQuery<
            SelectionResult<
                JoinFields<Fields, Joined, Automatic>,
                NullableTables | SourceName<Joined>
            >,
            JoinFields<Fields, Joined, Automatic>,
            NullableTables | SourceName<Joined>,
            Automatic
        >;
    }

    /** Include every row from the joined table and nullable rows from preceding tables. */
    rightJoin<Joined extends QuerySource>(
        table: Joined,
        on: SQL,
    ): SelectQuery<
        SelectionResult<
            JoinFields<Fields, Joined, Automatic>,
            Exclude<FieldTables<Fields>, SourceName<Joined>>
        >,
        JoinFields<Fields, Joined, Automatic>,
        Exclude<FieldTables<Fields>, SourceName<Joined>>,
        Automatic
    > {
        this.#join("right", table, on);

        return this as unknown as SelectQuery<
            SelectionResult<
                JoinFields<Fields, Joined, Automatic>,
                Exclude<FieldTables<Fields>, SourceName<Joined>>
            >,
            JoinFields<Fields, Joined, Automatic>,
            Exclude<FieldTables<Fields>, SourceName<Joined>>,
            Automatic
        >;
    }

    /** Include unmatched rows from either side of the join. */
    fullJoin<Joined extends QuerySource>(
        table: Joined,
        on: SQL,
    ): SelectQuery<
        SelectionResult<
            JoinFields<Fields, Joined, Automatic>,
            FieldTables<Fields> | SourceName<Joined>
        >,
        JoinFields<Fields, Joined, Automatic>,
        FieldTables<Fields> | SourceName<Joined>,
        Automatic
    > {
        this.#join("full", table, on);

        return this as unknown as SelectQuery<
            SelectionResult<
                JoinFields<Fields, Joined, Automatic>,
                FieldTables<Fields> | SourceName<Joined>
            >,
            JoinFields<Fields, Joined, Automatic>,
            FieldTables<Fields> | SourceName<Joined>,
            Automatic
        >;
    }

    /** Include every combination of rows from both tables. */
    crossJoin<Joined extends QuerySource>(
        table: Joined,
    ): SelectQuery<
        SelectionResult<JoinFields<Fields, Joined, Automatic>, NullableTables>,
        JoinFields<Fields, Joined, Automatic>,
        NullableTables,
        Automatic
    > {
        this.#join("cross", table);

        return this as unknown as SelectQuery<
            SelectionResult<JoinFields<Fields, Joined, Automatic>, NullableTables>,
            JoinFields<Fields, Joined, Automatic>,
            NullableTables,
            Automatic
        >;
    }

    /** Register a joined table and extend an automatic selection. */
    #join(kind: "inner" | "left" | "right" | "full" | "cross", table: QuerySource, on?: SQL): void {
        this.joins.push({ kind, table, on });
        if (this.isAutomatic) {
            Object.assign(this.fields, { [sourceName(table)]: sourceFields(table) });
        }
    }

    /** Group rows by the selected expressions. */
    groupBy(...expressions: SQLWrapper[]): this {
        this.groups = expressions;

        return this;
    }

    /** Filter grouped rows. */
    having(predicate: SQL | undefined): this {
        this.groupPredicate = predicate;

        return this;
    }

    /** Order rows by the selected expressions. */
    orderBy(...expressions: SQLWrapper[]): this {
        this.order = expressions;

        return this;
    }

    /** Limit the returned row count. */
    limit(count: number): this {
        this.count = count;

        return this;
    }

    /** Skip rows before returning results. */
    offset(count: number): this {
        this.skip = count;

        return this;
    }

    /** Execute and return the first row, if any. */
    async get(): Promise<Result | undefined> {
        const rows = await this.driver.run(() => this.#compile(1));

        return rows[0] as Result | undefined;
    }

    /** Execute through the selected dialect's query builder. */
    async execute(): Promise<Result[]> {
        return (await this.driver.run(() => this.#compile())) as Result[];
    }

    /** Compile SQL and positional parameters without executing the query. */
    toSQL(): Query {
        return this.#compile().toSQL();
    }

    /** Embed this selection as a scalar or predicate subquery. */
    getSQL(): SQL {
        return this.#compile().getSQL();
    }

    /** Compile a reusable query with named placeholder values. */
    prepare(): PreparedQuery<Result[]> {
        const query = this.#compile().prepare();

        return {
            execute: async (parameters) => {
                this.driver.transaction?.assertActive();

                return (await this.driver.run(() => query.execute(parameters))) as Result[];
            },
        };
    }

    /** Name a selection for use in FROM or JOIN. */
    as<const Alias extends string>(alias: Alias): SelectedSubquery<Result, Alias> {
        return this.#compile().as(alias) as SelectedSubquery<Result, Alias>;
    }

    /** Expose native selected fields for Drizzle common table expressions. */
    getSelectedFields(): Selection {
        return this.#compile().getSelectedFields();
    }

    /** Build a native query while retaining its result decoder. */
    #compile(maximum?: number): DrizzleSelect {
        // reject use after the enclosing transaction finishes
        this.driver.transaction?.assertActive();

        // materialize table aliases before translating selected columns
        const table = this.table instanceof Subquery ? this.table : this.compiler.table(this.table);
        for (const join of this.joins) {
            if (!(join.table instanceof Subquery)) {
                this.compiler.table(join.table);
            }
        }
        const selected =
            this.isAutomatic && this.joins.length === 0 ? sourceFields(this.table) : this.fields;
        const fields = selectFields(selected, this.compiler);

        // select the native compiler once, then apply the common Drizzle operations
        let query: DrizzleSelect;
        if (this.driver.native.dialect === "sqlite") {
            const database = this.driver.native.database.with(...this.withList);
            const selection = fields as sqlite.SelectedFields;
            query = (this.isDistinct
                ? database.selectDistinct(selection)
                : database.select(selection)
            )
                .from(table as SQLiteTable)
                .$dynamic() as unknown as DrizzleSelect;
        } else if (this.driver.native.dialect === "postgresql") {
            const database = this.driver.native.database.with(...this.withList);
            const selection = fields as postgres.SelectedFields;
            query = (this.isDistinct
                ? database.selectDistinct(selection)
                : database.select(selection)
            )
                .from(table as PgTable)
                .$dynamic() as unknown as DrizzleSelect;
        } else {
            return assertNever(this.driver.native);
        }

        // retain join order and native nullability decoding
        for (const join of this.joins) {
            const table =
                join.table instanceof Subquery ? join.table : this.compiler.table(join.table);
            const on = join.on && this.compiler.expression(join.on);
            switch (join.kind) {
                case "inner":
                    query = query.innerJoin(table, on);
                    break;
                case "left":
                    query = query.leftJoin(table, on);
                    break;
                case "right":
                    query = query.rightJoin(table, on);
                    break;
                case "full":
                    query = query.fullJoin(table, on);
                    break;
                case "cross":
                    query = query.crossJoin(table);
                    break;
                default:
                    assertNever(join.kind);
            }
        }

        // apply filters and grouping before ordering and pagination
        const expression = (value: SQLWrapper): SQLWrapper =>
            value instanceof Column
                ? this.compiler.column(value)
                : value instanceof SQL
                  ? this.compiler.expression(value)
                  : value;
        if (this.predicate) {
            query = query.where(this.compiler.expression(this.predicate));
        }
        if (this.groups.length) {
            query = query.groupBy(...this.groups.map(expression));
        }
        if (this.groupPredicate) {
            query = query.having(this.compiler.expression(this.groupPredicate));
        }
        if (this.order.length) {
            query = query.orderBy(...this.order.map(expression));
        }
        const count = maximum === undefined ? this.count : Math.min(this.count ?? maximum, maximum);
        if (count !== undefined) {
            query = query.limit(count);
        }
        if (this.skip !== undefined) {
            query = query.offset(this.skip);
        }

        return query;
    }

    /** Await query execution. */
    then<Fulfilled = Result[], Rejected = never>(
        fulfilled?: ((value: Result[]) => Fulfilled | PromiseLike<Fulfilled>) | null,
        rejected?: ((reason: unknown) => Rejected | PromiseLike<Rejected>) | null,
    ): PromiseLike<Fulfilled | Rejected> {
        return this.execute().then(fulfilled, rejected);
    }
}

/** A selection awaiting its source table. */
export class SelectBuilder<Fields extends Selection | undefined = undefined> {
    /** The native database, its connection state and its transaction. */
    readonly driver: DatabaseDriver;
    /** The compiler binding logical tables to the native dialect. */
    readonly compiler: SchemaCompiler;
    /** The explicitly selected fields. */
    readonly fields: Fields;
    /** Whether duplicate rows are removed. */
    readonly isDistinct: boolean;
    /** Common table expressions included in this query. */
    readonly withList: readonly WithSubquery[];

    /** Retain a partial query. */
    constructor(
        driver: DatabaseDriver,
        compiler: SchemaCompiler,
        fields: Fields,
        options: {
            readonly isDistinct?: boolean;
            readonly withList?: readonly WithSubquery[];
        } = {},
    ) {
        // retain the query definition
        this.withList = options.withList ?? [];
        this.driver = driver;
        this.compiler = compiler;
        this.fields = fields;
        this.isDistinct = options.isDistinct ?? false;
    }

    /** Select rows from a declared table. */
    from<Definition extends QuerySource>(
        table: Definition,
    ): SelectQuery<
        Fields extends Selection ? SelectionResult<Fields> : SourceResult<Definition>,
        Fields extends Selection
            ? Fields
            : Record<SourceName<Definition>, SourceFields<Definition>>,
        never,
        Fields extends Selection ? false : true
    > {
        const fields = this.fields ?? { [sourceName(table)]: sourceFields(table) };

        return new SelectQuery(
            this.driver,
            this.compiler,
            table,
            fields as Fields extends Selection
                ? Fields
                : Record<SourceName<Definition>, SourceFields<Definition>>,
            {
                isDistinct: this.isDistinct,
                isAutomatic: (this.fields === undefined) as Fields extends Selection ? false : true,
                withList: this.withList,
            },
        );
    }
}

/** Add a joined table to an automatic selection. */
type JoinFields<
    Fields extends Selection,
    Joined extends QuerySource,
    Automatic extends boolean,
> = Automatic extends true ? Fields & Record<SourceName<Joined>, SourceFields<Joined>> : Fields;

/** A compiled query that accepts named placeholder values. */
export interface PreparedQuery<Result> {
    /** Execute the compiled query with a new set of parameters. */
    execute(parameters?: Record<string, unknown>): Promise<Result>;
}

/** Table qualifiers referenced by selected columns and nested records. */
type FieldTables<Fields extends Selection> = {
    [Property in keyof Fields]: Fields[Property] extends Column
        ? Fields[Property]["table"]
        : Fields[Property] extends { readonly [SOURCE]: infer Name extends string }
          ? Name
          : Fields[Property] extends Table
            ? Fields[Property][typeof TABLE]["name"]
            : Fields[Property] extends Selection
              ? FieldTables<Fields[Property]>
              : never;
}[keyof Fields];

/** A declared table or a native Drizzle subquery. */
export type QuerySource = Table | Subquery<string, Selection>;

/** The selected record from a table or subquery. */
type SourceResult<Source extends QuerySource> = Source extends Table
    ? Select<Source>
    : Source extends Subquery<string, infer Fields extends Selection>
      ? SelectionResult<Fields>
      : never;

/** Selected fields qualified by a source name. */
type SourceFields<Source extends QuerySource> = Source extends Table
    ? Source
    : Source extends Subquery<string, infer Fields extends Selection>
      ? Fields
      : never;

/** The SQL qualifier of a table or subquery. */
type SourceName<Source extends QuerySource> = Source extends Table
    ? Source[typeof TABLE]["name"]
    : Source extends Subquery<infer Alias, Selection>
      ? Alias
      : never;

/** A named native query with its inferred result fields. */
export type SelectedSubquery<Result, Alias extends string = string> = Subquery<
    Alias,
    SubqueryFields<Result, Alias>
> &
    SubqueryFields<Result, Alias>;

/** Fields exposed by a named selection. */
type SubqueryFields<Result, Alias extends string> = {
    [Property in keyof Result]: SQL.Aliased<Result[Property]> & {
        readonly [SOURCE]: Alias;
    } & (NonNullable<Result[Property]> extends Record<string, unknown>
            ? SubqueryFields<NonNullable<Result[Property]>, Alias>
            : unknown);
};

/** Read the key a source's fields take in selected rows. */
function sourceName(source: QuerySource): string {
    return source instanceof Subquery ? source._.alias : source[TABLE].name;
}

/** Read a source's selected fields. */
function sourceFields(source: QuerySource): Selection {
    return source instanceof Subquery ? source._.selectedFields : source[TABLE].columns;
}
