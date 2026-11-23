import { present } from "@destack/schema";
import { Cell } from "./cell.ts";

/** The cells between two ticks of an axis, room for a label such as 120ms. */
const TICK_SPACING = 10;

/** The steps of an axis's ticks within each power of ten, as on a ruler. */
const TICK_STEPS = [1, 2, 5, 10] as const;

/** The blocks filling one to seven eighths of a cell from the left. */
const EIGHTHS = ["▏", "▎", "▍", "▌", "▋", "▊", "▉"] as const;

/** The eighths in a cell. */
const CELL_EIGHTHS = 8;

/** A timeline's time axis drawn in cells: its ruler and its bars. */
export const Axis = {
    /** Draw an axis's tick labels over a rule, at round steps of its length in milliseconds. */
    ruler(length: number, width: number): { readonly labels: string; readonly rule: string } {
        // take the round step giving about one tick per spacing
        const wanted = Math.max(1, Math.floor(width / TICK_SPACING));
        const raw = Math.max(length, 1) / wanted;
        const magnitude = 10 ** Math.floor(Math.log10(raw));
        const step = present(
            TICK_STEPS.map((each) => each * magnitude).find((each) => each >= raw),
            "a tick step",
        );

        // place each tick and its label
        const rule = Array.from({ length: width }, () => "─");
        let labels = "";
        for (let tick = 0; tick <= length; tick += step) {
            const position = Math.min(
                width - 1,
                Math.round((tick / Math.max(length, 1)) * (width - 1)),
            );
            rule[position] = "┬";

            // start each label at its tick, the last one ending at the rule's end
            const label = tick === 0 ? "0" : Cell.duration(tick);
            const start = Math.min(position, width - label.length);
            if (labels.length === 0 || start > labels.length) {
                labels = `${labels.padEnd(start)}${label}`;
            }
        }

        return { labels, rule: rule.join("") };
    },

    /** Draw a bar from one fraction of a width to another, in whole cells and a last eighth. */
    bar(
        from: number,
        to: number,
        width: number,
    ): { readonly offset: number; readonly width: number; readonly text: string } {
        // start at the cell the bar begins in, at least one eighth long
        const offset = Math.min(width - 1, Math.floor(from * width));
        const eighths = Math.max(1, Math.round((to * width - offset) * CELL_EIGHTHS));
        const cells = Math.min(width - offset, Math.ceil(eighths / CELL_EIGHTHS));

        // fill the whole cells, then the last cell's eighths
        const full = Math.min(cells, Math.floor(eighths / CELL_EIGHTHS));
        const rest = eighths % CELL_EIGHTHS;
        const partial = full < cells && rest > 0 ? (EIGHTHS[rest - 1] ?? "") : "";

        return { offset, width: cells, text: `${"█".repeat(full)}${partial}` };
    },
};
