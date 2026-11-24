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
import type { SelectQuery } from "./select.ts";
import type { Column, DriverValue } from "../table/column.ts";
import { TABLE, type Table } from "../table/table.ts";
import type { Row } from "../table/row.ts";
import type { Dialect } from "../dialect/dialect.ts";
import { DatabaseError } from "../error/error.ts";

/** A statement rendered once per dialect and place, run with named values. */
export class Statement<Built extends SQLWrapper = SQLWrapper> {
    /** Build the statement from named values on a connection, which gives its dialect. */
    readonly #build: (value: (name: string) => SQLWrapper, connection: DatabaseConnection) => Built;
    /** The rendered statement and its built query of each dialect and place. */
    readonly #rendered = new Map<
        string,
        { readonly template: StatementTemplate; readonly built: Built }
    >();

    /** Create the statement. */
    constructor(
        build: (value: (name: string) => SQLWrapper, connection: DatabaseConnection) => Built,
    ) {
        this.#build = build;
    }

    /** Read every row of a select with the values, decoded through its selection. */
    async select<Result extends object>(
        this: Statement<SQLWrapper & Pick<SelectQuery<Result>, "decode">>,
        database: DatabaseConnection,
        values: Readonly<Record<string, DriverValue>> = {},
    ): Promise<Result[]> {
        const { template, built } = this.#render(database);
        const rows = await database.driver.values(fill(template, values));

        return built.decode(rows, database.dialect, database.state.cipher);
    }

    /** Read every row by column name with the values. */
    all(
        database: DatabaseConnection,
        values: Readonly<Record<string, DriverValue>> = {},
    ): Promise<Record<string, unknown>[]> {
        return database.driver.all(fill(this.#render(database).template, values));
    }

    /** Read every row as an array of values with the values. */
    values(
        database: DatabaseConnection,
        values: Readonly<Record<string, DriverValue>> = {},
    ): Promise<unknown[][]> {
        return database.driver.values(fill(this.#render(database).template, values));
    }

    /** Run the statement for its effect with the values, as a write returning no rows. */
    run(
        database: DatabaseConnection,
        values: Readonly<Record<string, DriverValue>> = {},
    ): Promise<void> {
        return database.driver.execute(fill(this.#render(database).template, values));
    }

    /** Render the statement once per dialect and place: SQLite namespace or PostgreSQL schema. */
    #render(database: DatabaseConnection): {
        readonly template: StatementTemplate;
        readonly built: Built;
    } {
        // reuse the rendering of the database's dialect and place
        const { dialect } = database;
        const { namespace, schema } = database.state;
        const key = `${dialect}:${namespace ?? ""}:${schema ?? ""}`;
        const known = this.#rendered.get(key);
        if (known !== undefined) {
            return known;
        }
        const built = this.#build((name) => sql.placeholder(name), database);
        const rendered = { template: render(built, dialect, { namespace, schema }), built };
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

/** Read the digests a blob column references: its own, or each of those selected beside it. */
export function columnDigests(column: Column): { readonly digest: SQL; readonly from: SQL } {
    // read the digest of a scalar column
    const options = column.definition.blob;
    const path = options?.onDelete === "cascade" ? undefined : options?.path;
    if (column.definition.kind !== "json") {
        return { digest: sql`${column}`, from: sql.empty() };
    }
    // read each digest of a list or a map
    else if (path === undefined) {
        return {
            digest: sql.raw("element.value"),
            from: sql`, ${textValues(column, "element")}`,
        };
    }

    // read the digest at the path, or each of a list there
    const steps = path.split(".").map((name) => `."${name}"`);
    const key = sql.raw(`'$${steps.join("")}'`);
    const query = sql.raw(`'lax $${steps.join("")}[*]'`);

    return {
        digest: sql.raw("element.value"),
        from: dialectSQL({
            sqlite: sql`, json_each(CASE json_type(${column}, ${key})
                WHEN 'array' THEN json_extract(${column}, ${key})
                ELSE json_array(json_extract(${column}, ${key})) END) AS element`,
            postgresql: sql`, LATERAL (SELECT found #>> '{}' AS value
                FROM jsonb_path_query(${column}::jsonb, ${query}) AS found) AS element`,
        }),
    };
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
    // read a JSON column's element
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
            `column '${column.definition.name}' is invalid: it is binary, and SQLite cannot read bytes from JSON`,
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
