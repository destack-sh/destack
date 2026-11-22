import type { TelemetryScope } from "@destack/telemetry";
import {
    fill,
    inline,
    isConstant,
    Parameter,
    render,
    sql,
    type DriverStatement,
    type SQL,
    type SQLWrapper,
    type StatementTemplate,
} from "../sql/index.ts";
import { Column, type ColumnValue, type DriverValue } from "../table/column.ts";
import { TABLE, type Table } from "../table/table.ts";
import { Predicate } from "./predicate.ts";
import type { Comparable } from "./condition.ts";
import { DatabaseError } from "../error/error.ts";
import type { Dialect } from "../dialect/dialect.ts";
import type { ColumnCipher } from "../database/cipher.ts";
import { Projection, type Selection } from "./selection.ts";
import type { DatabaseConnection } from "../database/connection.ts";
import { Cache } from "../cache/cache.ts";

/**
 * The most prepared queries per connection, about twice the largest measured: 112 shapes.
 *
 * A forge's open and two publishes read at most 112 distinct shapes on one connection.
 * At 2 to 10 KB of statement text per query this bounds a connection near 2.5 MB.
 */
const PREPARED_QUERIES = 256;

/** A query prepared once per shape: its statement, its projection and each slot's column. */
export class PreparedQuery {
    /** The rendered statement, a placeholder `s<n>` standing in for each compared literal. */
    readonly template: StatementTemplate;
    /** The selection decoding the query's rows. */
    readonly projection: Projection;
    /** The column encoding each slot's literal, absent for a literal bound as it is. */
    readonly slots: readonly (Column | undefined)[];

    /** Hold a prepared query. */
    constructor(
        template: StatementTemplate,
        projection: Projection,
        slots: readonly (Column | undefined)[],
    ) {
        this.template = template;
        this.projection = projection;
        this.slots = slots;
    }

    /** Prepare a query once for a database's dialect and place, decoded through some fields. */
    static of(
        query: SQLWrapper,
        fields: Selection,
        slots: readonly (Column | undefined)[],
        database: DatabaseConnection,
    ): PreparedQuery {
        return new PreparedQuery(
            render(query, database.dialect, database.state),
            new Projection(fields),
            slots,
        );
    }

    /** Read one run's rows with its literals, decoded through the query's fields. */
    async rows<Read extends object>(
        database: DatabaseConnection,
        literals: readonly ColumnValue[],
    ): Promise<Read[]> {
        const { dialect, state } = database;
        const rows = await database.driver.values(this.bind(literals, dialect, state.cipher));

        return this.projection.decode<Read>(rows, dialect, new Set(), state.cipher);
    }

    /** Bind one run's literals to the slots, decoding each from JSON through its column. */
    bind(
        literals: readonly ColumnValue[],
        dialect: Dialect,
        cipher: ColumnCipher | undefined,
    ): DriverStatement {
        const values: Record<string, DriverValue> = {};
        for (const [index, column] of this.slots.entries()) {
            const literal = literals[index];
            const parameter =
                column === undefined
                    ? new Parameter(literal)
                    : new Parameter(column.definition.fromJson(literal), column);
            values[`s${String(index)}`] = parameter.encode(dialect, cipher);
        }

        return fill(this.template, values);
    }

    /** Bind one run's values to the slots, each through its column as written. */
    bindValues(
        written: readonly unknown[],
        dialect: Dialect,
        cipher: ColumnCipher | undefined,
    ): DriverStatement {
        const values: Record<string, DriverValue> = {};
        for (const [index, column] of this.slots.entries()) {
            values[`s${String(index)}`] = new Parameter(written[index], column).encode(
                dialect,
                cipher,
            );
        }

        return fill(this.template, values);
    }
}

/** The prepared queries of one connection, by table definition and shape. */
export class PreparedQueryCache {
    /** The prepared queries by table and shape key. */
    readonly #queries: Cache<PreparedQuery>;
    /** The text of each fragment binding no value, absent for one binding values. */
    readonly #fragments = new WeakMap<SQLWrapper, string | undefined>();
    /** A number for each table definition, so a table defined anew prepares its own queries. */
    readonly #tables = new WeakMap<Table, number>();
    /** The number of the next table definition. */
    #next = 0;

    /** Create the prepared queries of a connection, counted as the prepared query cache. */
    constructor(scope: TelemetryScope) {
        this.#queries = new Cache({ name: "prepared_query", capacity: PREPARED_QUERIES, scope });
    }

    /** Take a table's prepared query of a shape, preparing it the first time. */
    take(table: Table, shape: string, prepare: () => PreparedQuery): PreparedQuery {
        // number the table's definition once
        let number = this.#tables.get(table);
        if (number === undefined) {
            number = this.#next;
            this.#next += 1;
            this.#tables.set(table, number);
        }

        return this.#queries.take(`${String(number)} ${shape}`, prepare);
    }

    /** Read a fragment's text for a shape key once, absent when it binds values. */
    fragment(fragment: SQLWrapper, dialect: Dialect): string | undefined {
        // reuse the fragment's text
        if (this.#fragments.has(fragment)) {
            return this.#fragments.get(fragment);
        }
        const text = isConstant(fragment) ? inline(fragment, dialect) : undefined;
        this.#fragments.set(fragment, text);

        return text;
    }

    /** Take a table's prepared query of a predicate's shape, with the literals its slots bind. */
    matching(
        table: Table,
        predicate: Predicate,
        shape: string,
        prepare: (where: SQL, slots: readonly (Column | undefined)[]) => PreparedQuery,
    ): { readonly prepared: PreparedQuery; readonly literals: readonly Comparable[] } {
        // key the shape, collecting its compared literals in render order
        const literals: Comparable[] = [];
        const keyed = `${shape} where ${Predicate.key(predicate, literals)}`;
        const prepared = this.take(table, keyed, () => {
            // stand a placeholder in for each literal, noting the column encoding it
            const slots: (Column | undefined)[] = [];
            const slot = (field: SQLWrapper, literal: Comparable) => {
                // refuse a literal the key walk collected at another position
                const index = slots.length;
                if (literals[index] !== literal) {
                    throw DatabaseError.of(
                        "QUERY_BROKEN",
                        `prepared query of '${table[TABLE].name}' at '${keyed}' is broken: slot '${String(index)}' renders another literal than its key collects`,
                    );
                }

                // note the column encoding it
                slots.push(Column.is(field) && !Column.isPlain(field) ? field : undefined);

                return sql.placeholder(`s${String(index)}`);
            };
            const where = Predicate.render(predicate, Predicate.bind(table, {}, undefined, slot));

            // refuse a render whose slots differ in number from the key's literals
            if (slots.length !== literals.length) {
                throw DatabaseError.of(
                    "QUERY_BROKEN",
                    `prepared query of '${table[TABLE].name}' at '${keyed}' is broken: it renders '${String(slots.length)}' slots for '${String(literals.length)}' literals`,
                );
            }

            return prepare(where, slots);
        });

        return { prepared, literals };
    }
}
