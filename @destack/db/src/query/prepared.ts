import type { TelemetryScope } from "@destack/telemetry";
import { fill, Parameter, type DriverStatement, type StatementTemplate } from "../sql/index.ts";
import type { Column, ColumnValue, DriverValue } from "../table/column.ts";
import type { Table } from "../table/table.ts";
import type { Dialect } from "../dialect/dialect.ts";
import type { ColumnCipher } from "../database/cipher.ts";
import type { Projection } from "./selection.ts";
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
}

/** The prepared queries of one connection, by table definition and shape. */
export class PreparedQueryCache {
    /** The prepared queries by table and shape key. */
    readonly #queries: Cache<PreparedQuery>;
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
}
