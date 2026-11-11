import { type OrderBy, TABLE } from "@destack/db";
import { Field, type ObjectType } from "@destack/object";
import type { JsonObject, JsonValue } from "@destack/schema";

import type { Terminal } from "../output/index.ts";
import { CliError } from "../error/index.ts";
import { WallTime } from "./time.ts";

/** The most columns a table shows without --fields, beside the identifier. */
const DEFAULT_COLUMNS = 6;

/** The widest cell a table shows, in characters, cut with an ellipsis beyond. */
const CELL_WIDTH = 48;

/** The field types a table leaves out without --fields: structured values and long text. */
const HIDDEN_TYPES = new Set(["json", "text"]);

/** Rows of a type as a terminal or a program reads them. */
export const Render = {
    /** Pick the columns. */
    columns(type: ObjectType, fields: string | undefined): string[] {
        // take the named columns, refusing a property the rows lack
        if (fields !== undefined) {
            const named = fields.split(",").filter((field) => field !== "");
            const unknown = named.find((field) => !Object.hasOwn(type.table[TABLE].columns, field));
            if (unknown !== undefined) {
                throw CliError.of(
                    "USAGE_INVALID",
                    `field '${unknown}' is invalid: '${type.plural}' have no such field`,
                );
            }

            return named;
        }

        // take the identifier and the first declared fields that fit a cell
        const referenceColumns = new Set(
            Object.values(type.fields).flatMap((field) => {
                const columns =
                    field.reference === "object" && field.relation !== undefined
                        ? Field.columnsOf(field.relation)
                        : undefined;

                return columns === undefined
                    ? []
                    : [columns.packageId, columns.type, columns.scope];
            }),
        );
        const declared = Object.entries(type.fields)
            .filter(
                ([property, field]) =>
                    !HIDDEN_TYPES.has(field.type) && !referenceColumns.has(property),
            )
            .map(([property]) => property)
            .filter((property) => Object.hasOwn(type.table[TABLE].columns, property));

        return ["id", ...declared.slice(0, DEFAULT_COLUMNS)];
    },

    /** Read --sort, fields separated by commas with a minus for descending, into an order. */
    order(sort: string | undefined): OrderBy | undefined {
        if (sort === undefined) {
            return undefined;
        }

        return Object.fromEntries(
            sort
                .split(",")
                .filter((field) => field !== "")
                .map((field) =>
                    field.startsWith("-") ? [field.slice(1), "desc"] : [field, "asc"],
                ),
        );
    },

    /** Render rows as a table with a header, one row a line. */
    table(
        rows: readonly JsonObject[],
        columns: readonly string[],
        type: ObjectType,
        terminal: Pick<Terminal, "style" | "isTerminal" | "timeZone">,
    ): string {
        // tell a person there are none, and a program nothing
        if (rows.length === 0) {
            return terminal.isTerminal ? `No ${type.plural}.` : "";
        }

        // write each cell under its column's header
        const cells = rows.map((row) =>
            columns.map((column) => cell(type, column, row[column], terminal)),
        );

        return Render.grid(
            columns.map((column) => heading(type, column)),
            cells,
            terminal,
        );
    },

    /** Render padded columns under a bold header on a terminal, else tab-separated rows. */
    grid(
        header: readonly string[],
        cells: readonly (readonly string[])[],
        terminal: Pick<Terminal, "style" | "isTerminal" | "timeZone">,
    ): string {
        // separate each cell with a tab for a program, with no header
        if (!terminal.isTerminal) {
            return cells.map((line) => line.map(flatten).join("\t")).join("\n");
        }

        // pad each column to its widest cell under a bold header
        const widths = header.map((_, index) =>
            Math.max(...[header, ...cells].map((line) => line[index]?.length ?? 0)),
        );
        const line = (values: readonly string[]) =>
            values
                .map((value, index) => value.padEnd(widths[index] ?? 0))
                .join("  ")
                .trimEnd();

        return [terminal.style("bold", line(header)), ...cells.map(line)].join("\n");
    },

    /** Render one object as its fields, one a line, padded on a terminal, else tab-separated. */
    object(
        row: JsonObject,
        type: ObjectType,
        terminal: Pick<Terminal, "style" | "isTerminal" | "timeZone">,
    ): string {
        const width = Math.max(...Object.keys(row).map((field) => field.length));

        return Object.entries(row)
            .map(([field, value]) => {
                // write structure as JSON, any other value as its cell
                const shown =
                    typeof value === "object" && value !== null
                        ? JSON.stringify(value)
                        : cell(type, field, value, terminal);

                // pad the field's name for a person, and separate it with a tab for a program
                return terminal.isTerminal
                    ? `${terminal.style("dim", field.padEnd(width))}  ${shown}`
                    : `${field}\t${flatten(shown)}`;
            })
            .join("\n");
    },
};

/** Write one value as a table cell, cut to the cell width. */
function cell(
    type: ObjectType,
    column: string,
    value: JsonValue | undefined,
    terminal: Pick<Terminal, "isTerminal" | "timeZone">,
): string {
    // write a time as a person's local minute, cut to the cell, and whole as ISO 8601 for a program
    const isTime = type.fields[column]?.type === "time" && typeof value === "number";
    if (isTime && terminal.isTerminal) {
        return WallTime.of(value, terminal.timeZone).format();
    }
    // write a program's time whole
    else if (isTime) {
        return new Date(value).toISOString();
    }
    // write any other value as text, cut for a person
    else {
        return terminal.isTerminal ? cut(text(value)) : text(value);
    }
}

/** Head a column by its field, an object reference by its relation. */
function heading(type: ObjectType, column: string): string {
    const field = type.fields[column];
    const name = field?.reference === "object" ? field.relationAt(column) : column;

    return name.toUpperCase();
}

/** Write one value as text: absent as a dash, structured values as JSON. */
function text(value: JsonValue | undefined): string {
    // write absence as a dash
    if (value === undefined || value === null) {
        return "-";
    }
    // write structure as JSON
    else if (typeof value === "object") {
        return JSON.stringify(value);
    }
    // write a scalar as text
    else {
        return String(value);
    }
}

/** Write a cell on one line, its tabs and line breaks as spaces, so each row stays one record. */
function flatten(written: string): string {
    return written.replaceAll(/[\t\r\n]/gu, " ");
}

/** Cut text longer than a cell with an ellipsis. */
function cut(written: string): string {
    return written.length > CELL_WIDTH ? `${written.slice(0, CELL_WIDTH - 1)}…` : written;
}
