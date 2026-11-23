import type { BoardGroup, BoardLayout, ObjectType } from "@destack/object";
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
        const groups = frame.groups(rows, layout, type).map((group) => ({
            key: Frame.key(group.choice.value),
            heading: `${group.choice.title} ${frame.terminal.style("dim", String(group.rows.length))}`,
            rows: group.rows,
        }));

        return frame.grouped(groups, type, [layout.by], [], rows);
    },

    /** Draw a board's columns side by side, a heading over each column's cards. */
    draw(
        frame: Frame,
        groups: readonly BoardGroup[],
        type: ObjectType,
        width: number,
        by: string,
    ): string {
        // write each column's heading, then each card's title over its key fields
        const drawn = groups.map((group) => [
            Cell.fit(
                {
                    ...Cell.plain(`${group.choice.title} ${group.rows.length}`),
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
    records(
        frame: Frame,
        rows: readonly JsonObject[],
        layout: BoardLayout,
        type: ObjectType,
    ): string[][] {
        return frame
            .groups(rows, layout, type)
            .flatMap((group) =>
                group.rows.map((row) => [
                    Frame.key(group.choice.value ?? ""),
                    frame.titleText(type, row),
                ]),
            );
    },
};
