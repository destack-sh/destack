import { fillPlaceholders, sql, type Query, type SQL, type SQLWrapper } from "drizzle-orm";
import type { DatabaseConnection } from "../database/connection.ts";
import type { SchemaCompiler } from "../dialect/compiler.ts";
import { dialectSQL } from "../dialect/expression.ts";

/**
 * A statement rendered once per database and run with named values.
 *
 * Its constant text lets SQLite reuse its prepared statement and PostgreSQL its plan.
 */
export class Statement<Row extends Record<string, unknown> = Record<string, unknown>> {
    /** Build the statement from named values. */
    readonly #build: (value: (name: string) => SQLWrapper) => SQL;
    /** The rendered query of each compiler. */
    readonly #rendered = new WeakMap<SchemaCompiler, Query>();

    /** Create the statement. */
    constructor(build: (value: (name: string) => SQLWrapper) => SQL) {
        this.#build = build;
    }

    /** Read every row with the values. */
    all(
        database: DatabaseConnection,
        values: Readonly<Record<string, unknown>> = {},
    ): Promise<Row[]> {
        const query = this.#render(database);

        return database.driver.all<Row>({
            sql: query.sql,
            params: fillPlaceholders(query.params, values),
        });
    }

    /** Read every row as value arrays. */
    values(
        database: DatabaseConnection,
        values: Readonly<Record<string, unknown>> = {},
    ): Promise<unknown[][]> {
        const query = this.#render(database);

        return database.driver.values({
            sql: query.sql,
            params: fillPlaceholders(query.params, values),
        });
    }

    /** Render the statement once per compiler. */
    #render(database: DatabaseConnection): Query {
        // reuse the rendered query
        const known = this.#rendered.get(database.compiler);
        if (known !== undefined) {
            return known;
        }

        // compile and render the statement
        const compiled = database.compiler.expression(this.#build((name) => sql.placeholder(name)));
        const query = database.driver.render(compiled);
        this.#rendered.set(database.compiler, query);

        return query;
    }
}

/** Select a JSON array's elements as rows of one `value` column. */
export function jsonElements(array: SQLWrapper, name: string): SQL {
    return dialectSQL({
        sqlite: sql`json_each(${array}) AS ${sql.raw(name)}`,
        postgresql: sql`jsonb_array_elements(${array}::jsonb) AS ${sql.raw(name)}(value)`,
    });
}
