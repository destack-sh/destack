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
import type { Column, DriverValue } from "../table/column.ts";
import { TABLE, type Table } from "../table/table.ts";
import type { Row } from "../table/row.ts";
import type { Dialect } from "../dialect/dialect.ts";
import { DatabaseError } from "../error/error.ts";

/** A statement rendered once per dialect and namespace and run with named values. */
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

/** Match a value against the strings of a JSON array. */
export function listed(value: SQLWrapper, array: SQLWrapper): SQL {
    return dialectSQL({
        sqlite: sql`${value} IN (SELECT value FROM json_each(${array}))`,
        postgresql: sql`${value} IN (SELECT jsonb_array_elements_text(${array}::jsonb))`,
    });
}

/** Select a JSON array's elements as rows of one `value` column. */
export function jsonElements(array: SQLWrapper, name: string): SQL {
    return dialectSQL({
        sqlite: sql`json_each(${array}) AS ${sql.identifier(name)}`,
        postgresql: sql`jsonb_array_elements(${array}::jsonb) AS ${sql.identifier(name)}(value)`,
    });
}

/** Select a JSON array's strings as rows of a `value` column, ordered by a `key` column. */
export function textElements(array: SQLWrapper, name: string): SQL {
    return dialectSQL({
        sqlite: sql`json_each(${array}) AS ${sql.identifier(name)}`,
        postgresql: sql`jsonb_array_elements_text(${array}::jsonb) WITH ORDINALITY AS ${sql.identifier(name)}(value, key)`,
    });
}

/** Select the strings of a flat JSON array or object as rows of a `value` column. */
export function textValues(json: SQLWrapper, name: string): SQL {
    return dialectSQL({
        sqlite: sql`json_each(${json}) AS ${sql.identifier(name)}`,
        postgresql: sql`LATERAL (SELECT found #>> '{}' AS value FROM jsonb_path_query(${json}::jsonb, 'strict $.** ? (@.type() == "string")') AS found) AS ${sql.identifier(name)}`,
    });
}

/** Read a JSON element's value at a position as a column's type, the element named `wanted`. */
export function elementValue(column: Column, index: number, dialect: Dialect): SQL {
    // read a JSON column's element as JSON, a JSON null as SQL NULL, and any other or sealed element as its text
    const position = sql.raw(String(index));
    const isJson =
        column.definition.kind === "json" && column.definition.classification !== "sensitive";
    const element = isJson
        ? dialectSQL({
              sqlite: sql`NULLIF(wanted.value -> ${position}, 'null')`,
              postgresql: sql`NULLIF(wanted.value -> ${position}, 'null'::jsonb)`,
          })
        : sql`wanted.value ->> ${position}`;
    const type = column.definition.types.postgresql;
    const isBinary = column.definition.kind === "binary";

    // refuse SQLite bytes, which SQLite cannot decode from their base64 JSON form
    if (dialect === "sqlite" && isBinary) {
        throw DatabaseError.of(
            "QUERY_INVALID",
            `SQLite cannot read bytes from JSON: ${column.definition.name} is binary`,
        );
    }
    // read SQLite values and PostgreSQL text as they are
    else if (dialect === "sqlite" || type === "text") {
        return element;
    }
    // decode PostgreSQL bytes from base64
    else if (isBinary) {
        return sql`decode(${element}, 'base64')`;
    }
    // cast any other PostgreSQL value from its text or JSON
    else {
        return sql`(${element})::${sql.raw(type)}`;
    }
}

/** Write a row's column value in JSON form, null for none. */
export function jsonOf(table: Table, column: string, row: Row): unknown {
    const value = row[column];

    return value === undefined || value === null
        ? null
        : table[TABLE].column(column).definition.toJson(value);
}
