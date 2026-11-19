import { type ObjectType, TimelineLayout, TreeLayout } from "@destack/object";
import { type JsonObject, present } from "@destack/schema";
import { Axis } from "../axis.ts";
import { Cell } from "../cell.ts";
import { Frame } from "../frame.ts";
import { Page, type PageLine } from "../page.ts";
import { TreePage } from "./tree.ts";

/** The margin a drawing starts at, beside the gutter's words. */
const MARGIN = "  ";

/** The space between two columns. */
const GAP = "  ";

/** The width a drawing takes when the terminal reports none: 80 columns, the VT100's. */
const DEFAULT_COLUMNS = 80;

/** The widest label a timeline gives its rows. */
const TIMELINE_LABEL = 40;

/** The fewest cells a timeline's bars take, so a narrow terminal still shows their order. */
const MINIMUM_BAR = 10;

/** The mark after a failed bar, visible without color. */
const FAILED = "✗";

/** A timeline drawn for a terminal: bars on one time axis, nested by the type's tree. */
export const TimelinePage = {
    /** Draw a timeline page: a tick axis, one bar per timed row, and a summary. */
    of(frame: Frame, rows: readonly JsonObject[], layout: TimelineLayout, type: ObjectType): Page {
        // nest the rows by the type's tree, and place the timed ones on one axis
        const { terminal } = frame;
        const tree = type.layouts.find((each) => each.kind === "tree");
        const placed =
            tree?.kind === "tree"
                ? TreeLayout.nest(tree, rows).map(({ row, isLast }) => ({
                      row,
                      depth: isLast.length,
                      guide: TreePage.guide(isLast),
                  }))
                : rows.map((row) => ({ row, depth: 0, guide: "" }));
        const axis = TimelineLayout.axis(
            layout,
            placed.map((each) => each.row),
        );
        if (axis === undefined) {
            return new Page(
                [],
                [],
                `${MARGIN}${terminal.style("dim", `no ${type.plural} with a start and an end`)}`,
            );
        }

        // pair each entry with its place in the tree, its label and its duration
        const guides = new Map(placed.map((each) => [each.row, each]));
        const drawn = axis.entries.map((entry) => ({
            ...present(guides.get(entry.row), "the guide of an entry"),
            ...entry,
            label: frame.cells(type, entry.row, [Frame.title(type)])[0] ?? Cell.plain(""),
            duration: Cell.duration(entry.end - entry.start),
            isFailed: TimelineLayout.isFailed(layout, entry.row),
        }));

        // fit the labels, the bars, the durations and the failure marks into the width
        const columns = terminal.columns ?? DEFAULT_COLUMNS;
        const labelWidth = Math.min(
            TIMELINE_LABEL,
            Math.max(...drawn.map((each) => Cell.width(each.guide) + Cell.width(each.label.text))),
        );
        const durationWidth = Math.max(...drawn.map((each) => each.duration.length));
        const markWidth = drawn.some((each) => each.isFailed) ? FAILED.length + 1 : 0;
        const barWidth = Math.max(
            MINIMUM_BAR,
            columns - MARGIN.length - labelWidth - durationWidth - markWidth - 2 * GAP.length,
        );
        const indent = `${MARGIN}${" ".repeat(labelWidth)}${GAP}`;

        // mark the critical path when the type nests its objects, and draw the axis over a rule
        const critical =
            tree?.kind === "tree"
                ? TimelineLayout.critical(layout, tree, rows)
                : new Set<JsonObject>();
        const length = axis.window.end - axis.window.start;
        const ruler = Axis.ruler(length, barWidth);
        const header = [ruler.labels, ruler.rule].map(
            (line) => `${indent}${terminal.style("dim", line)}`,
        );

        // draw each bar in eighths of a cell, the critical path bright, a failure red
        const lines = drawn.map((each, index): PageLine => {
            // paint by the bar's standing
            const bar = Axis.bar(each.from, each.to, barWidth);
            const isCritical = critical.has(each.row);
            const paint = (written: string) =>
                each.isFailed
                    ? terminal.tone("failed", written)
                    : isCritical || critical.size === 0
                      ? terminal.tone("accent", written)
                      : terminal.style("dim", written);

            // place the label, the bar, the duration and a failure's mark
            const label = Cell.fit(
                each.isFailed ? { ...each.label, paint } : each.label,
                Math.max(1, labelWidth - Cell.width(each.guide)),
            );
            const after = " ".repeat(Math.max(0, barWidth - bar.offset - bar.width));
            const duration = terminal.style("dim", each.duration.padStart(durationWidth));
            const mark = each.isFailed ? ` ${terminal.tone("failed", FAILED)}` : "";

            return {
                key: tree?.kind === "tree" ? Frame.key(each.row[tree.id]) : String(index),
                depth: each.depth,
                row: each.row,
                text: `${MARGIN}${terminal.style("dim", each.guide)}${label}${GAP}${" ".repeat(bar.offset)}${paint(bar.text)}${after}${GAP}${duration}${mark}`,
            };
        });

        // sum up the count and length, the critical path and the failures
        const failures = drawn.filter((each) => each.isFailed).length;
        const summary = [
            `${drawn.length} ${type.plural} over ${Cell.duration(length)}`,
            ...(critical.size === 0 ? [] : [`${critical.size} on the critical path`]),
            ...(failures === 0 ? [] : [`${failures} failed`]),
        ].join(" · ");

        return new Page(header, lines, `${MARGIN}${terminal.style("dim", summary)}`);
    },

    /** Write each timed row's start, end and title for a program. */
    records(rows: readonly JsonObject[], layout: TimelineLayout, type: ObjectType): string[][] {
        return (TimelineLayout.axis(layout, rows)?.entries ?? []).map((each) => [
            new Date(each.start).toISOString(),
            new Date(each.end).toISOString(),
            Frame.titleText(type, each.row),
        ]);
    },
};
