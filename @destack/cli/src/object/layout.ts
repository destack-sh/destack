import {
    type CalendarDay,
    ObjectViewState,
    type ViewLine,
    BoardLayout,
    CalendarLayout,
    type Layout,
    type ObjectType,
    Presentation,
    TimelineLayout,
    type TimeWindow,
    TreeLayout,
} from "@destack/object";
import { Locale } from "@destack/locale";
import { type JsonObject, type JsonValue, present } from "@destack/schema";
import type { Terminal } from "../output/index.ts";

/** The margin a layout starts at on a terminal, beside the gutter's words. */
const MARGIN = "  ";

/** The microseconds in a millisecond, for durations below one. */
const MICROSECONDS_PER_MILLISECOND = 1000;

/** The width a layout draws at when the terminal reports none: 80 columns, the VT100's. */
const DEFAULT_COLUMNS = 80;

/** The narrowest board column, in characters, below which a board stacks its columns. */
const BOARD_COLUMN = 24;

/** The widest label a timeline gives its rows, in characters. */
const TIMELINE_LABEL = 40;

/** The days a calendar week shows. */
const WEEK_DAYS = 7;

/** The fewest cells a timeline's bars take, so a narrow terminal still shows their order. */
const MINIMUM_BAR = 10;

/** The mark after a failed bar, visible without color. */
const FAILED = "✗";

/** The cells between two ticks of a timeline's axis, room for a label such as 120ms. */
const TICK_SPACING = 10;

/** The blocks filling one to seven eighths of a cell from the left. */
const EIGHTHS = ["▏", "▎", "▍", "▌", "▋", "▊", "▉"] as const;

/** The steps of a timeline's ticks within each power of ten, as on a ruler. */
const TICK_STEPS = [1, 2, 5, 10] as const;

/** The eighths in a cell. */
const CELL_EIGHTHS = 8;

/** The lines a pager draws around a page's objects: a gap, the footer and the key hint. */
const PAGER_CHROME = 3;

/** The mark a selected line starts with, in place of its margin. */
const SELECTED = "• ";

/** The terminal a layout draws on: its styles, whether it is one, and its width. */
type Surface = Pick<Terminal, "style" | "tone" | "isTerminal" | "timeZone" | "locale" | "columns">;

/** One drawn line of a tree or a timeline: its view line, its text and the object it draws. */
export interface LayoutLine extends ViewLine {
    /** The line as drawn. */
    readonly text: string;
    /** The object the line draws, whose fields its details list. */
    readonly row: JsonObject;
}

/** A tree or a timeline drawn for a terminal: its header, one line per object and its footer. */
export interface LayoutPage {
    /** The lines above the objects, such as a timeline's axis. */
    readonly header: readonly string[];
    /** One line per object, in depth-first order. */
    readonly lines: readonly LayoutLine[];
    /** The line below the objects, such as a timeline's summary, empty for none. */
    readonly footer: string;
}

/** Rows of a type drawn by a layout other than a table. */
export const LayoutRender = {
    /** Draw rows by a layout, for a person on a terminal or as records for a program. */
    render(
        rows: readonly JsonObject[],
        layout: Exclude<Layout, { kind: "table" }>,
        type: ObjectType,
        surface: Surface,
        window?: TimeWindow,
    ): string {
        // tell a person there are none, and a program nothing
        if (rows.length === 0) {
            return surface.isTerminal
                ? `${MARGIN}${surface.style("dim", `no ${type.plural}`)}`
                : "";
        }

        // draw the layout's kind
        switch (layout.kind) {
            case "tree":
                return LayoutRender.tree(rows, layout, type, surface);
            case "timeline":
                return LayoutRender.timeline(rows, layout, type, surface);
            case "board":
                return LayoutRender.board(rows, layout, type, surface);
            case "calendar":
                return LayoutRender.calendar(rows, layout, type, surface, window);
            case "gallery":
                return LayoutRender.gallery(rows, type, surface);
        }
    },

    /** Draw rows nested under their parents, with guides on a terminal and depths for a program. */
    tree(
        rows: readonly JsonObject[],
        layout: TreeLayout,
        type: ObjectType,
        surface: Surface,
    ): string {
        // write each row's depth, key and title for a program
        if (!surface.isTerminal) {
            return LayoutRender.place(rows, layout)
                .map(({ row, depth }) => [depth, text(row[layout.id]), title(type, row)].join("\t"))
                .join("\n");
        }

        return LayoutRender.joined(LayoutRender.page(rows, layout, type, surface));
    },

    /** Order rows depth first under their parents, each with its depth and the guides before it. */
    place(
        rows: readonly JsonObject[],
        layout: TreeLayout,
    ): { readonly row: JsonObject; readonly depth: number; readonly guide: string }[] {
        return TreeLayout.nest(layout, rows).map(({ row, isLast }) => ({
            row,
            depth: isLast.length,
            guide: guideOf(isLast),
        }));
    },

    /** Draw one bar per row on a shared time axis, nested under parents as a waterfall. */
    timeline(
        rows: readonly JsonObject[],
        layout: TimelineLayout,
        type: ObjectType,
        surface: Surface,
    ): string {
        // write each timed row's start, end, depth and title for a program
        if (!surface.isTerminal) {
            const placed = placedOf(rows, type);
            const depths = new Map(placed.map((each) => [each.row, each.depth]));
            const axis = TimelineLayout.axis(
                layout,
                placed.map((each) => each.row),
            );

            return (axis === undefined ? [] : axis.entries)
                .map((each) =>
                    [
                        new Date(each.start).toISOString(),
                        new Date(each.end).toISOString(),
                        present(depths.get(each.row), "the depth of an entry"),
                        title(type, each.row),
                    ].join("\t"),
                )
                .join("\n");
        }

        return LayoutRender.joined(LayoutRender.page(rows, layout, type, surface));
    },

    /** Draw a tree or a timeline for a terminal as a header, one line per object and a footer. */
    page(
        rows: readonly JsonObject[],
        layout: TreeLayout | TimelineLayout,
        type: ObjectType,
        surface: Surface,
    ): LayoutPage {
        // draw each row's title after its guides, its key dimmed
        if (layout.kind === "tree") {
            const lines = LayoutRender.place(rows, layout).map(({ row, depth, guide }) => {
                const key = text(row[layout.id]);

                return {
                    key,
                    depth,
                    text: `${MARGIN}${surface.style("dim", guide)}${title(type, row)}  ${surface.style("dim", key)}`,
                    row,
                };
            });

            return { header: [], lines, footer: "" };
        }

        // place the timed rows on one axis
        const tree = type.layouts.find((each) => each.kind === "tree");
        const placed = placedOf(rows, type);
        const axis = TimelineLayout.axis(
            layout,
            placed.map((each) => each.row),
        );
        if (axis === undefined) {
            const none = surface.style("dim", `no ${type.plural} with a start and an end`);

            return { header: [], lines: [], footer: `${MARGIN}${none}` };
        }

        // pair each entry with its place in the tree
        const guides = new Map(placed.map((each) => [each.row, each]));
        const spans = axis.entries.map((entry) => ({
            ...present(guides.get(entry.row), "the guide of an entry"),
            ...entry,
        }));
        const first = axis.window.start;
        const last = axis.window.end;

        // fit the labels, the bars and the durations into the terminal's width
        const columns = surface.columns ?? DEFAULT_COLUMNS;
        const drawn = spans.map((each) => ({
            ...each,
            label: `${each.guide}${title(type, each.row)}`,
            duration: milliseconds(each.end - each.start),
            isFailed: TimelineLayout.isFailed(layout, each.row),
        }));
        const labelWidth = Math.min(
            TIMELINE_LABEL,
            Math.max(...drawn.map((each) => each.label.length)),
        );
        const durationWidth = Math.max(...drawn.map((each) => each.duration.length));
        const markWidth = drawn.some((each) => each.isFailed) ? FAILED.length + 1 : 0;
        const barWidth = Math.max(
            MINIMUM_BAR,
            columns - MARGIN.length - labelWidth - durationWidth - markWidth - 4,
        );
        const indent = `${MARGIN}${" ".repeat(labelWidth)}  `;

        // mark the critical path when the type nests its objects
        const critical =
            tree?.kind === "tree"
                ? TimelineLayout.critical(layout, tree, rows)
                : new Set<JsonObject>();

        // draw the axis's ticks over a rule
        const length = last - first;
        const ruler = rulerOf(length, barWidth);
        const header = [ruler.labels, ruler.rule].map(
            (line) => `${indent}${surface.style("dim", line)}`,
        );

        // draw each bar in eighths of a cell, the critical path bright
        const lines = drawn.map((each, index): LayoutLine => {
            // paint by the bar's standing
            const bar = barOf(each.from, each.to, barWidth);
            const isCritical = critical.has(each.row);
            const paint = (written: string) =>
                each.isFailed
                    ? surface.tone("failed", written)
                    : isCritical || critical.size === 0
                      ? surface.tone("accent", written)
                      : surface.style("dim", written);

            // place the label, the bar and the duration
            const label = fit(each.label, labelWidth);
            const shown = each.isFailed || !isCritical ? paint(label) : label;
            const after = " ".repeat(Math.max(0, barWidth - bar.offset - bar.width));
            const duration = surface.style("dim", each.duration.padStart(durationWidth));
            const mark = each.isFailed ? ` ${surface.tone("failed", FAILED)}` : "";

            return {
                key: tree?.kind === "tree" ? text(each.row[tree.id]) : String(index),
                depth: each.depth,
                row: each.row,
                text: `${MARGIN}${shown}  ${" ".repeat(bar.offset)}${paint(bar.text)}${after}  ${duration}${mark}`,
            };
        });

        // sum up the count and length, the critical path and the failures
        const failures = drawn.filter((each) => each.isFailed).length;
        const summary = [
            `${drawn.length} ${type.plural} over ${milliseconds(length)}`,
            ...(critical.size === 0 ? [] : [`${critical.size} on the critical path`]),
            ...(failures === 0 ? [] : [`${failures} failed`]),
        ].join(" · ");

        return { header, lines, footer: `${MARGIN}${surface.style("dim", summary)}` };
    },

    /** Join a page's header, lines and footer into one drawing. */
    joined(page: LayoutPage): string {
        const body = [...page.header, ...page.lines.map((line) => line.text)];
        const footer =
            page.footer === "" ? [] : body.length === 0 ? [page.footer] : ["", page.footer];

        return [...body, ...footer].join("\n");
    },

    /** Count the lines a pager may show of a page on a terminal of a height. */
    room(page: LayoutPage, height: number): number {
        return Math.max(1, height - page.header.length - PAGER_CHROME);
    },

    /** Draw a page's visible part under a pager, the cursor's line inverted. */
    paged(page: LayoutPage, state: ObjectViewState, height: number, surface: Surface): string {
        // take the visible lines from the top that fit
        const shown = ObjectViewState.visible(page.lines, state.folded);
        const cursor = state.cursor ?? shown[0]?.key;
        const fitting = shown.slice(state.top, state.top + LayoutRender.room(page, height));

        // draw each fitting line
        const body = fitting.flatMap((line) => {
            // draw the line, its fold mark and its details
            const folded = state.folded.has(line.key) ? surface.style("dim", " …") : "";
            const marked = state.selected.has(line.key)
                ? `${surface.tone("accent", SELECTED)}${line.text.slice(SELECTED.length)}`
                : line.text;
            const drawn = line.key === cursor ? surface.style("inverse", marked) : marked;
            const opened =
                line.key === state.opened
                    ? Object.entries(line.row).map(
                          ([field, value]) =>
                              `${MARGIN}    ${surface.style("dim", `${field}: ${text(value)}`)}`,
                      )
                    : [];

            return [`${drawn}${folded}`, ...opened];
        });
        const hint = surface.style(
            "dim",
            "↑↓ move  ← fold  → unfold  enter details  space select  q quit",
        );

        return [...page.header, ...body, "", page.footer, `${MARGIN}${hint}`].join("\n");
    },

    /** Draw one column per value of the board's field, side by side when they fit, else stacked. */
    board(
        rows: readonly JsonObject[],
        layout: BoardLayout,
        type: ObjectType,
        surface: Surface,
    ): string {
        // group the rows by their value in the field's order
        const groups = BoardLayout.group(layout, rows, type.fields);

        // write each row's value and title for a program
        if (!surface.isTerminal) {
            return groups
                .flatMap((group) => group.rows.map((row) => `${group.value}\t${title(type, row)}`))
                .join("\n");
        }

        // stack the columns when they do not fit side by side
        const columns = surface.columns ?? DEFAULT_COLUMNS;
        const width = Math.floor((columns - MARGIN.length) / groups.length) - 2;
        const heading = (group: (typeof groups)[number]) =>
            `${surface.style("bold", group.value)} ${surface.style("dim", String(group.rows.length))}`;
        if (width < BOARD_COLUMN) {
            return groups
                .map((group) =>
                    [
                        `${MARGIN}${heading(group)}`,
                        ...group.rows.map((row) => `${MARGIN}  ${title(type, row)}`),
                    ].join("\n"),
                )
                .join("\n\n");
        }

        // draw the columns side by side, a heading over each column's titles
        const height = Math.max(...groups.map((group) => group.rows.length));
        const cells = (group: (typeof groups)[number]) => [
            pad(heading(group), group.value.length + 1 + String(group.rows.length).length, width),
            ...Array.from({ length: height }, (_, index) => {
                const row = group.rows[index];

                return fit(row === undefined ? "" : `▪ ${title(type, row)}`, width);
            }),
        ];
        const grid = groups.map(cells);

        return Array.from(
            { length: height + 1 },
            (_, line) => `${MARGIN}${grid.map((column) => column[line] ?? "").join("  ")}`,
        ).join("\n");
    },

    /** Draw a month or the window as weeks of days, each day with its count and first title. */
    calendar(
        rows: readonly JsonObject[],
        layout: CalendarLayout,
        type: ObjectType,
        surface: Surface,
        window?: TimeWindow,
    ): string {
        // place each row on its day of the window
        const days = CalendarLayout.days(layout, rows, surface.timeZone, window);

        // write each row's day and title for a program
        if (!surface.isTerminal) {
            return days
                .flatMap((day) => day.rows.map((row) => `${day.date}\t${title(type, row)}`))
                .join("\n");
        }

        // tell a person when no row has a date
        const [start] = days;
        const end = days.at(-1);
        if (start === undefined || end === undefined) {
            return `${MARGIN}${surface.style("dim", `no ${type.plural} with ${layout.by}`)}`;
        }

        // head the weeks by the span's months and the weekdays from the locale's first one
        const columns = surface.columns ?? DEFAULT_COLUMNS;
        const width = Math.max(10, Math.floor((columns - MARGIN.length) / WEEK_DAYS) - 1);
        const firstDay = Locale.firstDay(surface.locale);
        const span = new Intl.DateTimeFormat(surface.locale, {
            month: "long",
            year: "numeric",
            timeZone: "UTC",
        });
        const weekday = new Intl.DateTimeFormat(surface.locale, {
            weekday: "short",
            timeZone: "UTC",
        });
        const names = Array.from({ length: WEEK_DAYS }, (_, index) =>
            fit(weekday.format(Date.UTC(2024, 0, firstDay + index)), width),
        );
        const lines = [
            `${MARGIN}${surface.style("bold", span.formatRange(dayOf(start), dayOf(end)))}`,
            `${MARGIN}${surface.style("dim", names.join(" "))}`,
        ];

        // pad the first week to its weekday, then draw each week's dates, counts and first titles
        const offset = (isoWeekday(start) - firstDay + WEEK_DAYS) % WEEK_DAYS;
        const cells = [...Array.from({ length: offset }, () => undefined), ...days];
        for (let week = 0; week < cells.length; week += WEEK_DAYS) {
            const dates: string[] = [];
            const firsts: string[] = [];
            for (const day of cells.slice(week, week + WEEK_DAYS)) {
                const first = day?.rows[0];
                const count =
                    day === undefined || day.rows.length === 0 ? "" : ` (${day.rows.length})`;
                const label = day === undefined ? "" : `${Number(day.date.slice(8))}${count}`;
                dates.push(pad(surface.style("bold", label), label.length, width));
                firsts.push(fit(first === undefined ? "" : title(type, first), width));
            }
            lines.push("", `${MARGIN}${dates.join(" ")}`, `${MARGIN}${firsts.join(" ")}`);
        }

        return lines.join("\n").trimEnd();
    },

    /** Draw one card per row, titled, in as many columns as fit. */
    gallery(rows: readonly JsonObject[], type: ObjectType, surface: Surface): string {
        // write each row's title for a program
        if (!surface.isTerminal) {
            return rows.map((row) => title(type, row)).join("\n");
        }

        // lay the cards out in rows of as many as fit
        const columns = surface.columns ?? DEFAULT_COLUMNS;
        const perLine = Math.max(1, Math.floor((columns - MARGIN.length) / (BOARD_COLUMN + 2)));
        const lines: string[] = [];
        for (let index = 0; index < rows.length; index += perLine) {
            const cards = rows
                .slice(index, index + perLine)
                .map((row) => fit(`▣ ${title(type, row)}`, BOARD_COLUMN));
            lines.push(`${MARGIN}${cards.join("  ")}`);
        }

        return lines.join("\n");
    },
};

/** Order rows under their parents when a type offers a tree, each with its depth and guides. */
function placedOf(
    rows: readonly JsonObject[],
    type: ObjectType,
): { readonly row: JsonObject; readonly depth: number; readonly guide: string }[] {
    const tree = type.layouts.find((each) => each.kind === "tree");

    return tree?.kind === "tree"
        ? LayoutRender.place(rows, tree)
        : rows.map((row) => ({ row, depth: 0, guide: "" }));
}

/** Draw a bar from one fraction of a width to another, in whole cells and a last eighth. */
function barOf(
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
}

/** Draw an axis's tick labels over a rule, at round steps of its length. */
function rulerOf(
    length: number,
    width: number,
): { readonly labels: string; readonly rule: string } {
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
        const label = tick === 0 ? "0" : milliseconds(tick);
        const start = Math.min(position, width - label.length);
        if (labels.length === 0 || start > labels.length) {
            labels = `${labels.padEnd(start)}${label}`;
        }
    }

    return { labels, rule: rule.join("") };
}

/** Read a row's title, else its identifier. */
function title(type: ObjectType, row: JsonObject): string {
    return Presentation.title(type.presentation, row);
}

/** Write a value as text: absent as a dash, structure as JSON. */
function text(value: JsonValue | undefined): string {
    // write absence as a dash, structure as JSON and a scalar as it reads
    if (value === undefined || value === null) {
        return "-";
    } else if (typeof value === "object") {
        return JSON.stringify(value);
    }

    return String(value);
}

/** Write the guides before a tree row: a line or a gap per ancestor level, then its branch. */
function guideOf(isLast: readonly boolean[]): string {
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
}

/** Write a duration in milliseconds as people read it: microseconds, milliseconds or seconds. */
function milliseconds(duration: number): string {
    // write under a millisecond in microseconds, under a second in milliseconds
    if (duration < 1) {
        return `${Math.round(duration * MICROSECONDS_PER_MILLISECOND)}µs`;
    } else if (duration < 1000) {
        return `${duration.toFixed(duration < 10 ? 1 : 0)}ms`;
    }

    return `${(duration / 1000).toFixed(2)}s`;
}

/** Cut text to a width with an ellipsis, padding a shorter one. */
function fit(written: string, width: number): string {
    return written.length > width ? `${written.slice(0, width - 1)}…` : written.padEnd(width);
}

/** Pad styled text of a visible length to a width. */
function pad(styled: string, visible: number, width: number): string {
    return `${styled}${" ".repeat(Math.max(0, width - visible))}`;
}

/** Read a calendar day's date as its UTC midnight. */
function dayOf(day: CalendarDay): number {
    return Date.parse(`${day.date}T00:00:00Z`);
}

/** Read a calendar day's ISO weekday, 1 for Monday through 7 for Sunday. */
function isoWeekday(day: CalendarDay): number {
    const weekday = new Date(dayOf(day)).getUTCDay();

    return weekday === 0 ? WEEK_DAYS : weekday;
}
