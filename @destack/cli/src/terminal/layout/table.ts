import type { ObjectType, TableLayout } from "@destack/object";
import type { JsonObject } from "@destack/schema";
import { Frame } from "../frame.ts";
import { Grid } from "../grid.ts";
import { Page } from "../page.ts";

/** A table drawn for a terminal: one row a line under headings ruled off. */
export const TablePage = {
    /** Draw a table as a page: its headings as the header, one line per row. */
    of(frame: Frame, rows: readonly JsonObject[], layout: TableLayout, type: ObjectType): Page {
        // draw the columns' headings and each row's cells, one row a line
        const columns = Frame.columns(type, layout);
        const drawn = Grid.lines(
            columns.map((column) => frame.heading(type, column)),
            rows.map((row) => frame.cells(type, row, columns)),
            frame.terminal,
        );
        const lines = rows.map((row, index) => ({
            key: Frame.key(row["id"]),
            depth: 0,
            row,
            text: drawn.lines[index] ?? "",
        }));

        return new Page(drawn.header, lines, frame.counted(rows, type));
    },
};
