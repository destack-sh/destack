import { type OrderBy, TABLE } from "@destack/db";
import { ObjectDetail, type ObjectType, TableLayout } from "@destack/object";
import type { JsonObject, JsonValue } from "@destack/schema";

import type { Terminal } from "../output/index.ts";
import { CliError } from "../error/index.ts";
import { WallTime } from "./time.ts";

/** The margin a person's records, tables and headings start at, beside the gutter's words. */
const MARGIN = "  ";

/** The widest cell a table shows, in characters, cut with an ellipsis beyond. */
const CELL_WIDTH = 48;

/** Rows of a type as a terminal or a program reads them. */
export const Render = {
    /** Pick the columns. */
    columns(type: ObjectType, layout: TableLayout, fields: string | undefined): string[] {
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

        // take the title, the layout's fields beside it, and the identifier last
        const title = type.presentation?.title;
        const shown = TableLayout.columns(layout, ObjectDetail.shown(type));

        return [...(title === undefined ? [] : [title]), ...shown, "id"].filter((column) =>
            Object.hasOwn(type.table[TABLE].columns, column),
        );
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
            return terminal.isTerminal
                ? `${MARGIN}${terminal.style("dim", `no ${type.plural}`)}`
                : "";
        }

        // write each cell under its column's header
        const cells = rows.map((row) =>
            columns.map((column) => cell(type, column, row[column], terminal)),
        );

        // count the rows below a person's table, the identifiers dimmed beside what people read
        const grid = Render.grid(
            columns.map((column) => heading(type, column)),
            cells,
            terminal,
            columns.indexOf("id"),
        );
        const counted = `${rows.length} ${rows.length === 1 ? type.name : type.plural}`;

        return terminal.isTerminal ? `${grid}\n\n${MARGIN}${terminal.style("dim", counted)}` : grid;
    },

    /** Render padded columns under a dim header on a terminal, else tab-separated rows. */
    grid(
        header: readonly string[],
        cells: readonly (readonly string[])[],
        terminal: Pick<Terminal, "style" | "isTerminal" | "timeZone">,
        dimmed = -1,
    ): string {
        // separate each cell with a tab for a program, with no header
        if (!terminal.isTerminal) {
            return cells.map((line) => line.map(flatten).join("\t")).join("\n");
        }

        // pad each column to its widest cell under a dim header, dimming one column's cells
        const widths = header.map((_, index) =>
            Math.max(...[header, ...cells].map((line) => line[index]?.length ?? 0)),
        );
        const pad = (value: string, index: number) =>
            index === header.length - 1 ? value : value.padEnd(widths[index] ?? 0);
        const line = (values: readonly string[]) =>
            values
                .map((value, index) =>
                    index === dimmed ? terminal.style("dim", pad(value, index)) : pad(value, index),
                )
                .join("  ");

        return [terminal.style("dim", header.map(pad).join("  ")), ...cells.map(line)]
            .map((row) => `${MARGIN}${row}`)
            .join("\n");
    },

    /** Render one object as its fields, one a line, padded on a terminal, else tab-separated. */
    object(
        row: JsonObject,
        type: ObjectType,
        terminal: Pick<Terminal, "style" | "heading" | "isTerminal" | "timeZone">,
    ): string {
        // head a person's view with the object's title and identifier, its other fields below
        const title = type.presentation?.title;
        const headed = terminal.isTerminal && title !== undefined && typeof row[title] === "string";
        const fields = Object.entries(row).filter(
            ([field]) => !headed || (field !== title && field !== "id"),
        );
        const width = Math.max(...fields.map(([field]) => field.length));
        const margin = terminal.isTerminal ? MARGIN : "";
        const head = headed
            ? [`${margin}${terminal.heading(text(row[title]), text(row["id"]))}`, ""]
            : [];

        return [
            ...head,
            ...fields.map(([field, value]) => {
                // write structure as JSON, any other value as its cell
                const shown =
                    typeof value === "object" && value !== null
                        ? JSON.stringify(value)
                        : cell(type, field, value, terminal);

                // pad the field's name for a person, and separate it with a tab for a program
                return terminal.isTerminal
                    ? `${margin}${terminal.style("dim", field.padEnd(width))}  ${shown}`
                    : `${field}\t${flatten(shown)}`;
            }),
        ].join("\n");
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
    const name = field?.reference === undefined ? column : field.relationAt(column);

    return name.replaceAll(/[A-Z]/gu, (upper) => ` ${upper}`).toUpperCase();
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
