import type { ObjectType } from "@destack/object";
import type { JsonObject } from "@destack/schema";
import { Frame } from "../frame.ts";
import { Page } from "../page.ts";

/** The margin a drawing starts at, beside the gutter's words. */
const MARGIN = "  ";

/** The space between two columns. */
const GAP = "  ";

/** The width a drawing takes when the terminal reports none: 80 columns, the VT100's. */
const DEFAULT_COLUMNS = 80;

/** The widest card of a gallery. */
const GALLERY_CARD = 24;

/** A gallery drawn for a terminal: cards in as many columns as fit. */
export const GalleryPage = {
    /** Draw a gallery as a page: one object line per row. */
    of(frame: Frame, rows: readonly JsonObject[], type: ObjectType): Page {
        const placed = rows.map((row) => ({ row, depth: 0, guide: "" }));

        return new Page([], frame.lines(placed, type, [], "id"), frame.counted(rows, type));
    },

    /** Draw one card per row in as many columns as fit. */
    draw(frame: Frame, rows: readonly JsonObject[], type: ObjectType): string {
        // lay the cards out in rows of as many as fit
        const perLine = Math.max(
            1,
            Math.floor(
                ((frame.terminal.columns ?? DEFAULT_COLUMNS) - MARGIN.length) /
                    (GALLERY_CARD + GAP.length),
            ),
        );
        const lines: string[] = [];
        for (let index = 0; index < rows.length; index += perLine) {
            const cards = rows
                .slice(index, index + perLine)
                .map((row) => frame.card(row, type, GALLERY_CARD, []));
            lines.push(
                ...[0, 1].map((part) =>
                    `${MARGIN}${cards.map((card) => card[part] ?? "").join(GAP)}`.trimEnd(),
                ),
                "",
            );
        }

        return lines.join("\n").trimEnd();
    },

    /** Write each row's title for a program. */
    records(rows: readonly JsonObject[], type: ObjectType): string[][] {
        return rows.map((row) => [Frame.titleText(type, row)]);
    },
};
