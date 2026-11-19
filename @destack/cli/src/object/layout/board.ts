import { BoardLayout, type ObjectType } from "@destack/object";
import type { JsonObject } from "@destack/schema";
import { Cell } from "../cell.ts";
import { Frame } from "../frame.ts";
import type { Page } from "../page.ts";

/** The margin a drawing starts at, beside the gutter's words. */
const MARGIN = "  ";

/** The space between two columns. */
const GAP = "  ";

/** A board drawn for a terminal: one column per value, its cards under it. */
export const BoardPage = {
    /** Draw a board as a page: each column's value as a heading over its objects' lines. */
    of(frame: Frame, rows: readonly JsonObject[], layout: BoardLayout, type: ObjectType): Page {
        const groups = BoardLayout.group(layout, rows, type.fields).map((group) => ({
            key: group.value,
            heading: `${group.value} ${frame.terminal.style("dim", String(group.rows.length))}`,
            rows: group.rows,
        }));

        return frame.grouped(groups, type, [layout.by], [], rows);
    },

    /** Draw a board's columns side by side, a heading over each column's cards. */
    draw(
        frame: Frame,
        groups: readonly { readonly value: string; readonly rows: readonly JsonObject[] }[],
        type: ObjectType,
        width: number,
        by: string,
    ): string {
        // write each column's heading, then each card's title over its key fields
        const drawn = groups.map((group) => [
            Cell.fit(
                {
                    ...Cell.plain(`${group.value} ${group.rows.length}`),
                    paint: (text) => frame.terminal.style("bold", text),
                },
                width,
            ),
            ...group.rows.flatMap((row) => frame.card(row, type, width, [by])),
        ]);
        const height = Math.max(...drawn.map((each) => each.length));

        return Array.from({ length: height }, (_, line) =>
            `${MARGIN}${drawn.map((each) => each[line] ?? " ".repeat(width)).join(GAP)}`.trimEnd(),
        ).join("\n");
    },

    /** Write each row's column value and title for a program. */
    records(rows: readonly JsonObject[], layout: BoardLayout, type: ObjectType): string[][] {
        return BoardLayout.group(layout, rows, type.fields).flatMap((group) =>
            group.rows.map((row) => [group.value, Frame.titleText(type, row)]),
        );
    },
};
