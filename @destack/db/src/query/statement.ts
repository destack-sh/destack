import { fillPlaceholders, sql, type Query, type SQL, type SQLWrapper } from "drizzle-orm";
import type { DatabaseConnection } from "../database/connection.ts";
import type { SchemaCompiler } from "../dialect/compiler.ts";
import { dialectSQL } from "../dialect/expression.ts";

/**
 * A statement built and rendered once per database, then run with named values.
 *
 * Its text stays the same across runs, so SQLite connections reuse its prepared statement and PostgreSQL its plan.
 */
export class Statement<Row extends Record<string, unknown> = Record<string, unknown>> {
    /** Build the statement, naming each value it takes. */
    readonly #build: (value: (name: string) => SQLWrapper) => SQL;
    /** The rendered text and parameters, by the compiler of the database it rendered for. */
    readonly #rendered = new WeakMap<SchemaCompiler, Query>();

    /** Keep how to build the statement, which runs once per database. */
    constructor(build: (value: (name: string) => SQLWrapper) => SQL) {
        this.#build = build;
    }

    /** Read every row of the statement with its values as driver rows, on a connection or within a transaction. */
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

    /** Render the statement once for a database's compiler. */
    #render(database: DatabaseConnection): Query {
        // reuse the text rendered for the same compiler
        const known = this.#rendered.get(database.compiler);
        if (known !== undefined) {
            return known;
        }

        // compile the logical statement and render it in the connection's dialect
        const compiled = database.compiler.expression(this.#build((name) => sql.placeholder(name)));
        const query = database.driver.render(compiled);
        this.#rendered.set(database.compiler, query);

        return query;
    }
}

/** Select the elements of a JSON array value as rows of one `value` column under a name, whose fields `->>` reads. */
export function jsonElements(array: SQLWrapper, name: string): SQL {
    return dialectSQL({
        sqlite: sql`json_each(${array}) AS ${sql.raw(name)}`,
        postgresql: sql`jsonb_array_elements(${array}::jsonb) AS ${sql.raw(name)}(value)`,
    });
}
