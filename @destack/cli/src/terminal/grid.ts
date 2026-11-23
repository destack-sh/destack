import type { Terminal } from "./index.ts";
import { Cell } from "./cell.ts";

/** The margin a grid starts at, beside the gutter's words. */
const MARGIN = "  ";

/** The width a terminal is taken to have when it reports none: 80, the VT100's. */
const DEFAULT_COLUMNS = 80;

/** The narrowest a column shrinks to before columns hide. */
const MINIMUM_CELL = 8;

/** The space between two columns. */
const GAP = "  ";

/** The line under a grid's header. */
const RULE = "─";

/** The column standing in for hidden ones, as DuckDB elides them. */
const ELISION = "…";

/** Columns of cells fitted to a terminal's width, under a header ruled off. */
export const Grid = {
    /** Draw cells under a header for a person, else as tab-separated rows for a program. */
    draw(
        header: readonly string[],
        rows: readonly (readonly Cell[])[],
        terminal: Pick<Terminal, "style" | "isTerminal" | "columns">,
    ): string {
        // separate each cell with a tab for a program, with no header
        if (!terminal.isTerminal) {
            return rows.map((line) => line.map((each) => each.text).join("\t")).join("\n");
        }

        const drawn = Grid.lines(header, rows, terminal);

        return [...drawn.header, ...drawn.lines].join("\n");
    },

    /** Draw cells as lines under a header ruled off, the columns fitted to the terminal. */
    lines(
        header: readonly string[],
        rows: readonly (readonly Cell[])[],
        terminal: Pick<Terminal, "style" | "columns">,
    ): { readonly header: readonly string[]; readonly lines: readonly string[] } {
        // size each column to its widest text, then fit them to the terminal, the last one shown
        const widths = header.map((name, index) =>
            Math.max(Cell.width(name), ...rows.map((line) => Cell.width(line[index]?.text ?? ""))),
        );
        const available = (terminal.columns ?? DEFAULT_COLUMNS) - MARGIN.length;
        const shown = Grid.fit(widths, available, header.length - 1);
        const isHiding = shown.length < header.length;

        // draw the header, its rule and each row, with an elision column when columns hide
        const line = (cells: readonly string[], elision: string) =>
            `${MARGIN}${[...cells, ...(isHiding ? [elision] : [])].join(GAP)}`.trimEnd();
        const dim = (text: string) => terminal.style("dim", text);
        const headings = shown.map((index) => {
            // align the header with its column's values
            const align = rows[0]?.[index]?.align ?? "start";
            const cell = { ...Cell.plain(header[index] ?? ""), align };

            return terminal.style("bold", Cell.fit(cell, widths[index] ?? 0));
        });

        return {
            header: [
                line(headings, dim(ELISION)),
                line(
                    shown.map((index) => dim(RULE.repeat(widths[index] ?? 0))),
                    dim(RULE),
                ),
            ],
            lines: rows.map((cells) =>
                line(
                    shown.map((index) =>
                        Cell.fit(cells[index] ?? Cell.plain(""), widths[index] ?? 0),
                    ),
                    dim(ELISION),
                ),
            ),
        };
    },

    /** Pick the columns that fit a width: narrow the widest, then hide from the right. */
    fit(widths: number[], available: number, fixed: number): number[] {
        // measure the shown columns with their gaps and any elision column
        let shown = widths.map((_, index) => index);
        const total = () =>
            shown.reduce((sum, index) => sum + (widths[index] ?? 0), 0) +
            GAP.length * (shown.length - 1) +
            (shown.length < widths.length ? GAP.length + ELISION.length : 0);

        // narrow the widest column, else hide the last one besides the first and the last
        while (total() > available) {
            const widest = shown.reduce((best, index) =>
                index !== fixed && (widths[index] ?? 0) > (widths[best] ?? 0) ? index : best,
            );
            const hidden = shown.findLast((index) => index !== 0 && index !== fixed);
            if ((widths[widest] ?? 0) > MINIMUM_CELL) {
                widths[widest] = (widths[widest] ?? 0) - 1;
            } else if (hidden !== undefined) {
                shown = shown.filter((index) => index !== hidden);
            } else {
                break;
            }
        }

        return shown;
    },
};
