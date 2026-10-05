import {
    dialectSQL,
    fill,
    render,
    sql,
    type StatementTemplate,
    type SQL,
    type SQLWrapper,
} from "../sql/index.ts";
import type { DatabaseConnection } from "../database/connection.ts";
import type { DriverValue } from "../table/column.ts";

/**
 * A statement rendered once per dialect and namespace and run with named values.
 *
 * Its constant text lets SQLite reuse its prepared statement and PostgreSQL its plan.
 */
export class Statement {
    /** Build the statement from named values. */
    readonly #build: (value: (name: string) => SQLWrapper) => SQL;
    /** The rendered statement of each dialect and namespace. */
    readonly #rendered = new Map<string, StatementTemplate>();

    /** Create the statement. */
    constructor(build: (value: (name: string) => SQLWrapper) => SQL) {
        this.#build = build;
    }

    /** Read every row by column name with the values. */
    all(
        database: DatabaseConnection,
        values: Readonly<Record<string, DriverValue>> = {},
    ): Promise<Record<string, unknown>[]> {
        return database.driver.all(fill(this.#render(database), values));
    }

    /** Read every row as an array of values with the values. */
    values(
        database: DatabaseConnection,
        values: Readonly<Record<string, DriverValue>> = {},
    ): Promise<unknown[][]> {
        return database.driver.values(fill(this.#render(database), values));
    }

    /** Render the statement once per dialect and namespace. */
    #render(database: DatabaseConnection): StatementTemplate {
        // reuse the rendering of the database's dialect and namespace
        const { dialect } = database;
        const { namespace } = database.state;
        const key = namespace === undefined ? dialect : `${dialect}:${namespace}`;
        const known = this.#rendered.get(key);
        if (known !== undefined) {
            return known;
        }
        const rendered = render(
            this.#build((name) => sql.placeholder(name)),
            dialect,
            namespace,
        );
        this.#rendered.set(key, rendered);

        return rendered;
    }
}

/** Select a JSON array's elements as rows of one `value` column. */
export function jsonElements(array: SQLWrapper, name: string): SQL {
    return dialectSQL({
        sqlite: sql`json_each(${array}) AS ${sql.identifier(name)}`,
        postgresql: sql`jsonb_array_elements(${array}::jsonb) AS ${sql.identifier(name)}(value)`,
    });
}
