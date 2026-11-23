import type { ObjectType } from "@destack/object";
import type { JsonObject } from "@destack/schema";
import type { Frame } from "./frame.ts";

/** The margin a page starts at, beside the gutter's words. */
const MARGIN = "  ";

/** The lines a pager draws around a page's objects: a gap, the footer and the key hint. */
const PAGER_CHROME = 3;

/** The mark a selected line starts with, in place of its margin. */
const SELECTED = "• ";

/** The marks of an open and a folded line with lines below it. */
const FOLDS = { open: "▾ ", folded: "▸ " } as const;

/** A move on a page: up, down, fold or step out, unfold or step in, details, or select. */
export type PageKey = "up" | "down" | "left" | "right" | "enter" | "select";

/** Where a person stands on a page: the cursor, top line, folds, details and selection. */
export interface PageState {
    /** The key of the line under the cursor, the first line's when absent. */
    readonly cursor: string | undefined;
    /** The index of the first visible line shown. */
    readonly top: number;
    /** The keys of the lines whose lines below are hidden. */
    readonly folded: ReadonlySet<string>;
    /** The key of the object whose details show under it, none when absent. */
    readonly opened: string | undefined;
    /** The keys of the selected objects, which bulk actions apply to. */
    readonly selected: ReadonlySet<string>;
}

/** Where a person stands on a page. */
export const PageState = {
    /** Start at the first line with nothing folded, opened or selected. */
    start(): PageState {
        return {
            cursor: undefined,
            top: 0,
            folded: new Set(),
            opened: undefined,
            selected: new Set(),
        };
    },
};

/** One drawn line of a page: its key, depth and text, and its object, none for a heading. */
export interface PageLine {
    /** The key of the line's object or group, which the cursor, folds and details follow. */
    readonly key: string;
    /** The line's depth below the page's roots, zero for a root. */
    readonly depth: number;
    /** The line as drawn. */
    readonly text: string;
    /** The object the line draws, absent for a group's heading. */
    readonly row: JsonObject | undefined;
}

/** A layout drawn for a terminal: its header, its lines of objects and groups, and its footer. */
export class Page {
    /** The lines above the objects, such as a table's headings or a calendar's month. */
    readonly header: readonly string[];
    /** The lines of objects and group headings, in order. */
    readonly lines: readonly PageLine[];
    /** The line below the objects, such as a count or a timeline's summary. */
    readonly footer: string;

    /** Create a page of a header, lines and a footer. */
    constructor(header: readonly string[], lines: readonly PageLine[], footer: string) {
        this.header = header;
        this.lines = lines;
        this.footer = footer;
    }

    /** Join the header, lines and footer into one drawing. */
    joined(): string {
        const body = [...this.header, ...this.lines.map((line) => line.text)];
        const footer =
            this.footer === "" ? [] : body.length === 0 ? [this.footer] : ["", this.footer];

        return [...body, ...footer].join("\n");
    }

    /** Count the lines a pager shows on a terminal of a height. */
    room(height: number): number {
        return Math.max(1, height - this.header.length - PAGER_CHROME);
    }

    /** List the lines a fold leaves visible: every line but the descendants of folded ones. */
    visible(folded: ReadonlySet<string>): PageLine[] {
        // skip the lines deeper than a folded line until one at its depth or above
        const shown: PageLine[] = [];
        let hiddenBelow: number | undefined;
        for (const line of this.lines) {
            // show a line outside folded subtrees
            const isHidden = hiddenBelow !== undefined && line.depth > hiddenBelow;
            if (!isHidden) {
                hiddenBelow = folded.has(line.key) ? line.depth : undefined;
                shown.push(line);
            }
        }

        return shown;
    }

    /** Answer a key on a terminal of a height: move, fold or unfold, open details, or select. */
    press(state: PageState, key: PageKey, height: number): PageState {
        // find the cursor among the visible lines, the first one when it is gone
        const { lines } = this;
        const shown = this.visible(state.folded);
        const found = shown.findIndex((line) => line.key === state.cursor);
        const index = Math.max(0, found);
        const line = shown[index];
        if (line === undefined) {
            return state;
        }
        const hasChildren = (lines[lines.indexOf(line) + 1]?.depth ?? -1) > line.depth;
        const isFolded = state.folded.has(line.key);

        // move, fold, unfold or open by the key
        let next = index;
        let folded = state.folded;
        let opened = state.opened;
        let selected = state.selected;
        if (key === "up") {
            next = Math.max(0, index - 1);
        } else if (key === "down") {
            next = Math.min(shown.length - 1, index + 1);
        }
        // fold an open subtree, else step out to the parent
        else if (key === "left") {
            if (hasChildren && !isFolded) {
                folded = new Set([...state.folded, line.key]);
            } else {
                const parent = shown.findLastIndex(
                    (each, position) => position < index && each.depth < line.depth,
                );
                next = parent === -1 ? index : parent;
            }
        }
        // unfold a folded subtree, else step into its first child
        else if (key === "right") {
            if (isFolded) {
                folded = new Set([...state.folded].filter((each) => each !== line.key));
            } else if (hasChildren) {
                next = Math.min(shown.length - 1, index + 1);
            }
        }
        // open the details under the cursor, or close them
        else if (key === "enter") {
            opened = state.opened === line.key ? undefined : line.key;
        }
        // select the object under the cursor, or deselect it
        else {
            selected = state.selected.has(line.key)
                ? new Set([...state.selected].filter((each) => each !== line.key))
                : new Set([...state.selected, line.key]);
        }

        // scroll so the cursor stays within the height
        const top = Math.min(next, Math.max(state.top, next - this.room(height) + 1));

        return { cursor: shown[next]?.key, top, folded, opened, selected };
    }

    /** Draw the visible part under a pager: the cursor inverted, folds marked, details opened. */
    paged(state: PageState, height: number, frame: Frame, type: ObjectType): string {
        // take the visible lines from the top that fit, and the lines with lines below them
        const { terminal } = frame;
        const shown = this.visible(state.folded);
        const cursor = state.cursor ?? shown[0]?.key;
        const fitting = shown.slice(state.top, state.top + this.room(height));
        const parents = new Set(
            this.lines.flatMap((line, index) =>
                (this.lines[index + 1]?.depth ?? 0) > line.depth ? [line.key] : [],
            ),
        );

        // draw each fitting line
        const body = fitting.flatMap((line) => {
            // mark a parent open or folded, and a selected line
            const fold = parents.has(line.key)
                ? state.folded.has(line.key)
                    ? FOLDS.folded
                    : FOLDS.open
                : "";
            const selected = state.selected.has(line.key)
                ? `${terminal.tone("accent", SELECTED)}${line.text.slice(SELECTED.length)}`
                : line.text;
            const marked = `${selected.slice(0, MARGIN.length)}${terminal.style("dim", fold)}${selected.slice(MARGIN.length)}`;
            const drawn = line.key === cursor ? terminal.style("inverse", marked) : marked;

            // list the opened object's fields below it
            const opened =
                line.key === state.opened && line.row !== undefined
                    ? frame
                          .record(line.row, type)
                          .split("\n")
                          .map((each) => `${MARGIN}  ${each}`)
                    : [];

            return [drawn, ...opened];
        });
        const hint = terminal.style("dim", "↑↓ move  ←→ fold  enter details  space select  q quit");

        return [...this.header, ...body, "", this.footer, `${MARGIN}${hint}`].join("\n");
    }
}
