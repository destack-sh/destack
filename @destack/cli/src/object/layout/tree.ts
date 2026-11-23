import type { ObjectType } from "@destack/object";
import { TreeLayout } from "@destack/object";
import type { JsonObject } from "@destack/schema";
import { Frame } from "../frame.ts";
import { Page } from "../page.ts";

/** A tree drawn for a terminal: rows nested under their parents behind guides. */
export const TreePage = {
    /** Draw a tree as a page: each row's line behind its guides. */
    of(frame: Frame, rows: readonly JsonObject[], layout: TreeLayout, type: ObjectType): Page {
        const placed = TreeLayout.nest(layout, rows).map(({ row, isLast }) => ({
            row,
            depth: isLast.length,
            guide: TreePage.guide(isLast),
        }));

        return new Page(
            [],
            frame.lines(placed, type, [layout.id, layout.parent], layout.id),
            frame.counted(rows, type),
        );
    },

    /** Write the guides before a tree row: a line or a gap per ancestor level, then its branch. */
    guide(isLast: readonly boolean[]): string {
        // draw nothing before a root
        if (isLast.length === 0) {
            return "";
        }

        // continue each ancestor's line past a sibling below it, then branch to the row
        const lines = isLast
            .slice(0, -1)
            .map((last) => (last ? "   " : "│  "))
            .join("");

        return `${lines}${isLast.at(-1) === true ? "└─ " : "├─ "}`;
    },

    /** Write each row's depth, key and title for a program. */
    records(
        frame: Frame,
        rows: readonly JsonObject[],
        layout: TreeLayout,
        type: ObjectType,
    ): string[][] {
        return TreeLayout.nest(layout, rows).map(({ row, isLast }) => [
            String(isLast.length),
            Frame.key(row[layout.id]),
            frame.titleText(type, row),
        ]);
    },
};
