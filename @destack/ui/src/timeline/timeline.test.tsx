import { nth, type ScheduleLink, type TickUnit, type TimeScale } from "@destack/schema";
import { expect, onTestFinished, test } from "@destack/test";
import { space } from "@destack/theme/tokens.stylex";
import { Locale, Localization, Shortcut } from "@destack/locale";
import { Errored, flush, type JSX, LocaleContext } from "@destack/view";
import { render, stubPopovers } from "@destack/view/test";
import { layOutElements } from "../virtualizer/test/layout.ts";
import {
    Timeline,
    TimelineToolbar,
    TimelineAxis,
    TimelineBar,
    TimelineBody,
    TimelineCell,
    type TimelineControl,
    TimelineColumn,
    TimelineGrid,
    TimelineHeader,
    TimelineLinkLayer,
    TimelineNow,
    type TimelineProperties,
    TimelineRange,
    type TimelineItem,
    TimelineMarker,
    TimelineMinimap,
    type TimelineRowMove,
    TimelineRow,
    TimelineRowLabel,
    TimelineRowList,
    TimelineToday,
    TimelineZoom,
    useTimeline,
} from "./index.ts";

/** The words ICU versions join a date and its time with in a default format: "at" or a comma. */
const DATE_TIME_JOINER = / at |, /u;
/** Render a timeline with every part, as an app lays one out. */
function Plan(properties: TimelineProperties): JSX.Element {
    return (
        <Timeline {...properties}>
            <TimelineHeader />
            <TimelineBody>
                <TimelineGrid />
                <TimelineNow />
                <TimelineLinkLayer />
                <TimelineRowList />
            </TimelineBody>
            <TimelineMinimap />
        </Timeline>
    );
}

/** The width every element measures to, so the track fits eight marks. */
const WIDTH = 800;

/** The height the rows' scroller measures to, room for ten rows. */
const ROWS_HEIGHT = 320;

/** A request's trace: two children, one of them with a failed child, on the critical path. */
const TRACE: readonly TimelineItem[] = [
    { key: "request", label: "GET /launch", start: 0, end: 1000 },
    { key: "query", label: "SELECT signups", start: 50, end: 400, parent: "request" },
    { key: "render", label: "render page", start: 400, end: 950, parent: "request" },
    {
        key: "template",
        label: "render template",
        start: 420,
        end: 900,
        parent: "render",
    },
];

/** The pixels a level of depth indents by in the stubbed layout. */
const INDENT = 16;
/** Lay out the timeline at the test's size, its bars' cards stubbed, until the test ends. */
function measureElements(): void {
    // size the virtualizer's scroller, give every element the track's bounds and an indent its own, and show bars' cards
    layOutElements({ client: ROWS_HEIGHT, scroll: ROWS_HEIGHT });
    stubPopovers();
    const measure = Object.getOwnPropertyDescriptor(Element.prototype, "getBoundingClientRect");
    Element.prototype.getBoundingClientRect = function (this: Element) {
        const isIndent = this instanceof HTMLElement && this.style.width === space[4];

        return new DOMRect(0, 0, isIndent ? INDENT : WIDTH, ROWS_HEIGHT);
    };

    // restore the bounds once the test finishes
    onTestFinished(() => {
        if (measure !== undefined) {
            Object.defineProperty(Element.prototype, "getBoundingClientRect", measure);
        }
    });
}

/** Press and release a key on the focused element. */
function press(container: HTMLElement, key: string, modifiers: KeyboardEventInit = {}): void {
    // settle what came before, then each key event as the browser's own task
    for (const type of ["keydown", "keyup"]) {
        flush();
        (container.ownerDocument.activeElement ?? container).dispatchEvent(
            new KeyboardEvent(type, { ...modifiers, key, bubbles: true, cancelable: true }),
        );
    }
    flush();
}

/** Read the visible rows as their depth and label. */
function rowsOf(container: HTMLElement): string[] {
    return [...container.querySelectorAll<HTMLElement>("[data-slot=timeline-row]")].map(
        (row) =>
            `${row.getAttribute("aria-level") ?? ""} ${row.querySelector("[data-slot=timeline-bar]")?.textContent ?? ""}`,
    );
}

test("nest a trace's spans under their parents, folding and unfolding with the arrow keys", () => {
    // focus the request, step into it, fold the render span and unfold it again
    measureElements();
    const { container } = render(() => <Plan items={TRACE} axis="offset" />);
    const before = rowsOf(container);
    container.querySelector<HTMLElement>("[data-slot=timeline-row]")?.focus();
    press(container, "ArrowDown");
    press(container, "ArrowDown");
    press(container, "ArrowLeft");
    const folded = rowsOf(container);
    press(container, "ArrowRight");

    expect({ before, folded, unfolded: rowsOf(container) }).toEqual({
        before: ["1 GET /launch", "2 SELECT signups", "2 render page", "3 render template"],
        folded: ["1 GET /launch", "2 SELECT signups", "2 render page"],
        unfolded: ["1 GET /launch", "2 SELECT signups", "2 render page", "3 render template"],
    });
});

test("draw an uncolored parent's bar as a summary, and a colored parent's in its color", () => {
    // render the trace with the render span colored
    measureElements();
    const items = TRACE.map((item) =>
        item.key === "render" ? { ...item, color: "green" as const } : item,
    );
    const { container } = render(() => <Plan items={items} axis="offset" />);
    flush();
    const bars = [...container.querySelectorAll<HTMLElement>("[data-slot=timeline-bar]")];

    expect(bars.map((bar) => [bar.textContent, bar.dataset["summary"]])).toEqual([
        ["GET /launch", ""],
        ["SELECT signups", undefined],
        ["render page", undefined],
        ["render template", undefined],
    ]);
});

test("mark a trace's axis in offsets from its start, and fill a bar the caller composes solid", () => {
    // compose the rows, filling the failed template's bar solid red
    measureElements();
    const { container } = render(() => (
        <Timeline items={TRACE} axis="offset">
            <TimelineHeader />
            <TimelineBody>
                <TimelineRowList>
                    {(node) => (
                        <TimelineRow>
                            <TimelineRowLabel />
                            <TimelineBar
                                {...(node().item.key === "template"
                                    ? { color: "red", variant: "solid" }
                                    : {})}
                            />
                        </TimelineRow>
                    )}
                </TimelineRowList>
            </TimelineBody>
        </Timeline>
    ));
    const marks = [...container.querySelectorAll("[data-slot=timeline-axis-mark]")].map(
        (mark) => mark.textContent,
    );
    const bars = [...container.querySelectorAll<HTMLElement>("[data-slot=timeline-bar]")].map(
        (bar) => `${bar.textContent ?? ""} ${bar.dataset["variant"] ?? ""}`,
    );

    const parts = [...(container.querySelector("[data-slot=timeline]")?.children ?? [])]
        .map((part) => part.getAttribute("data-slot"))
        .filter((slot) => slot?.startsWith("timeline") === true);

    expect({ marks, bars, parts }).toEqual({
        marks: ["0ms", "200ms", "400ms", "600ms", "800ms", "1,000ms"],
        bars: [
            "GET /launch tint",
            "SELECT signups tint",
            "render page tint",
            "render template solid",
        ],
        parts: ["timeline-header", "timeline-body"],
    });
});

test("move a plan's task from its focused row by the axis's snap with the keys, and open it with Enter", () => {
    // a ten-day plan in UTC, its first row focused, its task lifted with Space, stepped right and dropped
    measureElements();
    const day = 86_400_000;
    const plan: readonly TimelineItem[] = [
        { key: "design", label: "Design", start: 0, end: 3 * day },
        { key: "build", label: "Build", start: 3 * day, end: 10 * day },
    ];
    const changes: TimelineItem[] = [];
    const opened: string[] = [];
    const { container } = render(() => (
        <Plan
            items={plan}
            timeZone="UTC"
            onItemsChange={(items) => changes.push(...items)}
            onAction={(key) => opened.push(key)}
        />
    ));
    container.querySelector<HTMLElement>("[role=row][tabindex='0']")?.focus();
    for (const key of [" ", "ArrowRight", " ", "Enter"]) {
        press(container, key);
    }

    // the plan's 10.4 days over 25 marks of 32 pixels step by whole days
    expect({
        moved: changes.map((item) => [item.key, ...hoursOf(item)]),
        opened,
    }).toEqual({ moved: [["design", 24, 96]], opened: ["design"] });
});

/** Read an item's start and end in hours from the epoch. */
function hoursOf(item: TimelineItem): number[] {
    if (item.start === undefined || item.end === undefined) {
        throw new TypeError(`timeline item '${item.key}' changed without both dates`);
    }

    return [item.start / 3_600_000, item.end / 3_600_000];
}

/** Drag an element by the pointer a distance along the track. */
function drag(element: Element | null, distance: number): void {
    // press, move past the lift distance, then let go
    for (const [type, x] of [
        ["pointerdown", 200],
        ["pointermove", 200 + distance],
        ["pointerup", 200 + distance],
    ] as const) {
        element?.dispatchEvent(
            new PointerEvent(type, {
                bubbles: true,
                cancelable: true,
                clientX: x,
                clientY: 10,
                pointerId: 5,
                width: 1,
                height: 1,
            }),
        );
        flush();
    }
}

test("show a moving bar where it lands, with its dates beside it, before the drop", () => {
    // drag Design a hundred pixels right and hold it there
    measureElements();
    const day = 86_400_000;
    const plan: readonly TimelineItem[] = [
        { key: "design", label: "Design", start: 0, end: 3 * day },
        { key: "build", label: "Build", start: 3 * day, end: 10 * day },
    ];
    const { container } = render(() => (
        <Plan items={plan} timeZone="UTC" onItemsChange={() => undefined} />
    ));
    const bar = container.querySelector<HTMLElement>("[data-slot=timeline-bar]");
    const before = startOf(bar);
    for (const [type, x] of [
        ["pointerdown", 200],
        ["pointermove", 300],
    ] as const) {
        bar?.dispatchEvent(
            new PointerEvent(type, {
                bubbles: true,
                cancelable: true,
                clientX: x,
                clientY: 10,
                pointerId: 5,
            }),
        );
        flush();
    }

    // the bar moves from day 0 to day 1 of the 10.4 days shown, its dates at the marks' precision
    expect({
        starts: [before, startOf(bar)].map((start) => Math.round(start * 10) / 10),
        dates: container.querySelector("[data-slot=timeline-bar-dates]")?.textContent,
    }).toEqual({ starts: [1.9, 11.5], dates: "Jan 2 – 4" });
});

/** Read a bar's start along the track, in percent. */
function startOf(bar: HTMLElement | null): number {
    return Number.parseFloat(bar?.style.getPropertyValue("--x-insetInlineStart") ?? "");
}

test("describe a bar in its hover card by its dates and planned dates, a milestone by its date", () => {
    // a three-day task, a milestone and a task a day behind its plan, each bar focused in turn
    measureElements();
    const day = 86_400_000;
    const plan: readonly TimelineItem[] = [
        { key: "design", label: "Design", start: 0, end: 3 * day },
        { key: "launch", label: "Launch", start: 9 * day, end: 9 * day },
        {
            key: "build",
            label: "Build",
            start: 4 * day,
            end: 9 * day,
            baseline: { start: 3 * day, end: 7 * day },
        },
    ];
    const { container } = render(() => <Plan items={plan} timeZone="UTC" />);
    const cards = [...container.querySelectorAll<HTMLElement>("[data-slot=timeline-bar]")].map(
        (bar) => {
            bar.focus();
            flush();
            const card = document.querySelector("[data-slot=timeline-bar-card][data-popover-open]");
            const text = card?.textContent;
            bar.blur();
            flush();

            return text;
        },
    );

    expect({
        cards,
        baselines: container.querySelectorAll("[data-slot=timeline-bar-baseline]").length,
    }).toEqual({
        cards: [
            "DesignJan 1 – 3 · 3d",
            "LaunchJan 10",
            "BuildJan 5 – 9 · 5dPlanned \u2068Jan 4 – 7\u2069",
        ],
        baselines: 1,
    });
});

/** Read the width of a header's labels' column in pixels, 16rem being 256. */
function labelsOf(header: HTMLElement | null): number {
    const columns = header?.style.getPropertyValue("--x-gridTemplateColumns") ?? "";

    return columns.startsWith("16rem") ? 256 : Number.parseFloat(columns);
}

test("resize the labels' column from its edge by the pointer and the arrow keys, within its bounds", () => {
    // an 800-pixel timeline whose corner measures the labels' column, its edge dragged and stepped
    measureElements();
    const widths: string[] = [];
    const { container } = render(() => (
        <Plan items={PLAN} timeZone="UTC" onLabelWidthChange={(width) => widths.push(width)} />
    ));
    const header = container.querySelector<HTMLElement>("[data-slot=timeline-header]");
    const corner = container.querySelector<HTMLElement>("[data-slot=timeline-corner]");
    if (corner !== null) {
        corner.getBoundingClientRect = () => new DOMRect(0, 0, labelsOf(header), 40);
    }
    const edge = container.querySelector<HTMLElement>("[data-slot=timeline-resize]");
    for (const [type, x] of [
        ["pointerdown", 256],
        ["pointermove", 296],
        ["pointerup", 296],
    ] as const) {
        edge?.dispatchEvent(
            new PointerEvent(type, { bubbles: true, clientX: x, clientY: 10, pointerId: 7 }),
        );
        flush();
    }
    edge?.focus();
    press(container, "ArrowLeft");
    for (let step = 0; step < 20; step += 1) {
        press(container, "ArrowRight");
    }

    // forty pixels wider, a step narrower, then up to half the timeline
    expect({ first: widths.slice(0, 2), last: widths.at(-1) }).toEqual({
        first: ["296px", "280px"],
        last: "400px",
    });
});

test("draw an item with one date as an open bar, and set an undated item's dates by a sweep or Space", () => {
    // a dated design, a build with its start alone, and a copy and a ship without dates
    measureElements();
    const day = 86_400_000;
    const plan: readonly TimelineItem[] = [
        { key: "design", label: "Design", start: 0, end: 3 * day },
        { key: "build", label: "Build", start: 3 * day },
        { key: "copy", label: "Copy" },
        { key: "ship", label: "Ship" },
    ];
    const changes: TimelineItem[] = [];
    const { container } = render(() => (
        <Plan items={plan} timeZone="UTC" onItemsChange={(items) => changes.push(...items)} />
    ));
    const bars = container.querySelectorAll<HTMLElement>("[data-slot=timeline-bar]");
    bars[1]?.focus();
    flush();
    const card = document.querySelector("[data-slot=timeline-bar-card][data-popover-open]");
    const open = (card?.textContent ?? "").replace(DATE_TIME_JOINER, ", ");
    bars[1]?.blur();

    // sweep Copy's track from 100 to 300 pixels, and press Space on Ship's row
    const undated = container.querySelectorAll("[data-slot=timeline-track][data-undated]");
    for (const [type, x] of [
        ["pointerdown", 100],
        ["pointermove", 200],
        ["pointermove", 300],
        ["pointerup", 300],
    ] as const) {
        undated[0]?.dispatchEvent(
            new PointerEvent(type, { bubbles: true, clientX: x, clientY: 80, pointerId: 8 }),
        );
        flush();
    }
    container.querySelector<HTMLElement>("[data-slot=timeline-row][data-key=ship]")?.focus();
    press(container, " ");

    // the three days shown take marks every 12 hours, which the sweep and Space snap to
    expect({
        bars: bars.length,
        undated: undated.length,
        open,
        changes: changes.map((item) => [item.key, ...hoursOf(item)]),
    }).toEqual({
        bars: 2,
        undated: 2,
        open: "BuildFrom \u2068Jan 4, 12:00 AM\u2069",
        changes: [
            ["copy", 12, 24],
            ["ship", 36, 48],
        ],
    });
});

test("pan the window by dragging empty track, following the pointer in either direction of text", () => {
    // drag the empty track of Build's row eighty pixels left, a tenth of the track
    measureElements();
    const day = 86_400_000;
    const plan: readonly TimelineItem[] = [
        { key: "design", label: "Design", start: 0, end: 3 * day },
        { key: "build", label: "Build", start: 3 * day, end: 10 * day },
    ];
    const panned = (tag: string) => {
        const { container } = render(() => (
            <LocaleContext value={Localization.of(Locale.parse(tag), [])}>
                <Plan items={plan} timeZone="UTC" />
            </LocaleContext>
        ));
        const track = container.querySelectorAll("[data-slot=timeline-track]")[1];
        const bar = () => container.querySelector<HTMLElement>("[data-slot=timeline-bar]");
        const before = startOf(bar());
        for (const [type, x] of [
            ["pointerdown", 400],
            ["pointermove", 360],
            ["pointermove", 320],
            ["pointerup", 320],
        ] as const) {
            track?.dispatchEvent(
                new PointerEvent(type, { bubbles: true, clientX: x, clientY: 30, pointerId: 6 }),
            );
            flush();
        }

        return Math.round(startOf(bar()) - before);
    };

    // the bars move with the pointer: a tenth left in English, a tenth right in Hebrew
    expect({ english: panned("en-GB"), hebrew: panned("he-IL") }).toEqual({
        english: -10,
        hebrew: 10,
    });
});

test("show a chip at the edge a bar lies past, which brings the bar back into view", () => {
    // pan Design's three days out past the start by dragging the empty track most of its width
    measureElements();
    const day = 86_400_000;
    const plan: readonly TimelineItem[] = [
        { key: "design", label: "Design", start: 0, end: 3 * day },
        { key: "build", label: "Build", start: 3 * day, end: 10 * day },
    ];
    const { container } = render(() => <Plan items={plan} timeZone="UTC" />);
    const track = container.querySelectorAll("[data-slot=timeline-track]")[1];
    for (const [type, x] of [
        ["pointerdown", 700],
        ["pointermove", 500],
        ["pointermove", 300],
        ["pointerup", 300],
        ["click", 300],
    ] as const) {
        track?.dispatchEvent(
            new PointerEvent(type, { bubbles: true, clientX: x, clientY: 30, pointerId: 6 }),
        );
        flush();
    }
    const chips = () =>
        [...container.querySelectorAll("[data-slot=timeline-bar-outside]")].map((chip) => [
            chip.getAttribute("data-side"),
            chip.textContent,
        ]);
    const before = chips();
    container.querySelector<HTMLElement>("[data-slot=timeline-bar-outside]")?.click();
    flush();

    expect({ before, after: chips() }).toEqual({ before: [["start", "Design"]], after: [] });
});

test("move a bar by the pointer's travel, mirrored right to left, and stretch its end by its handle", () => {
    // a three-day task of a ten-day plan, dragged a hundred pixels right in English and in Hebrew
    measureElements();
    const day = 86_400_000;
    const plan: readonly TimelineItem[] = [
        { key: "design", label: "Design", start: 0, end: 3 * day },
        { key: "build", label: "Build", start: 3 * day, end: 10 * day },
    ];
    const changesIn = (tag: string, slot: string) => {
        const changes: TimelineItem[] = [];
        const { container } = render(() => (
            <LocaleContext value={Localization.of(Locale.parse(tag), [])}>
                <Plan
                    items={plan}
                    timeZone="UTC"
                    onItemsChange={(items) => changes.push(...items)}
                />
            </LocaleContext>
        ));
        drag(container.querySelector(`[data-slot=${slot}]`), 100);

        return changes.map(hoursOf);
    };

    // a hundred of eight hundred pixels over the 10.4 days shown is 31.2 hours, snapped to a day
    expect({
        moved: changesIn("en-GB", "timeline-bar"),
        mirrored: changesIn("he-IL", "timeline-bar"),
        stretched: changesIn("en-GB", "timeline-bar-handle][data-edge=end"),
    }).toEqual({
        moved: [[24, 96]],
        mirrored: [[-24, 48]],
        stretched: [[0, 96]],
    });
});

/** The milliseconds of a day. */
const DAY_MS = 86_400_000;

/** A ten-day plan: a design task, then a build task, a launch milestone, and a review half done. */
const PLAN: readonly TimelineItem[] = [
    { key: "design", label: "Design", start: 0, end: 3 * DAY_MS },
    { key: "build", label: "Build", start: 3 * DAY_MS, end: 10 * DAY_MS, progress: 0.5 },
    { key: "launch", label: "Launch", start: 10 * DAY_MS, end: 10 * DAY_MS },
];

test("draw a link from a bar's end onto another bar's edge by keys and by pointer, its type from the edges", () => {
    // draw from Design's end onto Build's start with L, the arrows and Space
    measureElements();
    const links: ScheduleLink[] = [];
    const { container } = render(() => (
        <Plan items={PLAN} timeZone="UTC" onLinkCreate={(link) => links.push(link)} />
    ));
    container.querySelector<HTMLElement>("[role=row][tabindex='0']")?.focus();
    for (const key of ["l", "ArrowDown", "ArrowLeft", " "]) {
        press(container, key);
    }

    // drag Build's start connector onto Launch's row, nearer its end, which is its start too
    const connector = container.querySelectorAll(
        "[data-slot=timeline-bar-connector][data-edge=start]",
    )[1];
    for (const [type, x, y] of [
        ["pointerdown", 200, 48],
        ["pointermove", 780, 80],
        ["pointerup", 780, 80],
    ] as const) {
        connector?.dispatchEvent(
            new PointerEvent(type, {
                bubbles: true,
                cancelable: true,
                clientX: x,
                clientY: y,
                pointerId: 6,
                width: 1,
                height: 1,
            }),
        );
        flush();
    }

    expect(links).toEqual([
        { from: "design", to: "build", type: "finish-to-start", lag: 0 },
        { from: "build", to: "launch", type: "start-to-start", lag: 0 },
    ]);
});

test("select rows with X and Shift and the arrows, move the selected bars together, and clear with Escape", () => {
    // select Design and Build with X, move the selection a mark right from Build, then extend and clear
    measureElements();
    const changes: TimelineItem[] = [];
    const { container } = render(() => (
        <Plan items={PLAN} timeZone="UTC" onItemsChange={(items) => changes.push(...items)} />
    ));
    container.querySelector<HTMLElement>("[role=row][tabindex='0']")?.focus();
    for (const key of ["x", "ArrowDown", "x", " ", "ArrowRight", " "]) {
        press(container, key);
    }
    const selected = () =>
        [...container.querySelectorAll("[data-slot=timeline-row][aria-selected=true]")].map((row) =>
            row.getAttribute("data-key"),
        );
    const moved = selected();
    press(container, "Escape");
    const cleared = selected();
    press(container, "ArrowDown", { shiftKey: true });
    const extended = selected();

    expect({
        changes: changes.map((item) => [item.key, ...hoursOf(item)]),
        moved,
        cleared,
        extended,
    }).toEqual({
        changes: [
            ["build", 96, 264],
            ["design", 24, 96],
        ],
        moved: ["design", "build"],
        cleared: [],
        extended: ["build", "launch"],
    });
});

test("move rows by Mod+Shift and the arrows and by their grips, into another parent, never into themselves", () => {
    // a launch phase with two tasks, a review, and a ship; keys then a grip move rows about
    measureElements();
    const day = 86_400_000;
    const plan: readonly TimelineItem[] = [
        { key: "phase", label: "Phase", start: 0, end: 6 * day },
        { key: "design", label: "Design", start: 0, end: 3 * day, parent: "phase" },
        { key: "build", label: "Build", start: 3 * day, end: 6 * day, parent: "phase" },
        { key: "review", label: "Review", start: 6 * day, end: 7 * day },
        { key: "ship", label: "Ship", start: 7 * day, end: 7 * day },
    ];
    const moves: TimelineRowMove[] = [];
    const { container } = render(() => (
        <Plan items={plan} timeZone="UTC" onRowMove={(move) => moves.push(move)} />
    ));
    const row = (key: string) =>
        container.querySelector<HTMLElement>(`[data-slot=timeline-row][data-key=${key}]`);

    // move Review up past Build, into the phase, then Phase down past Review and Ship
    row("review")?.focus();
    const mod = Shortcut.isCommand(navigator.userAgent) ? { metaKey: true } : { ctrlKey: true };
    press(container, "ArrowUp", { shiftKey: true, ...mod });
    row("phase")?.focus();
    press(container, "ArrowDown", { shiftKey: true, ...mod });

    // drag Ship's grip from its row up into the gap between Phase and Design
    const grip = row("ship")?.querySelector("[data-slot=timeline-row-grip]");
    for (const [type, y] of [
        ["pointerdown", 4.5 * 32],
        ["pointermove", 3 * 32],
        ["pointermove", 1 * 32],
        ["pointerup", 1 * 32],
    ] as const) {
        grip?.dispatchEvent(
            new PointerEvent(type, {
                bubbles: true,
                cancelable: true,
                clientX: 20,
                clientY: y,
                pointerId: 9,
            }),
        );
        flush();
    }

    // the review lands between design and build, the phase after it, and ship first in the phase
    expect(moves).toEqual([
        { key: "review", parent: "phase", previous: "design", next: "build" },
        { key: "phase", previous: "review", next: "ship" },
        { key: "ship", parent: "phase", next: "design" },
    ]);
});

test("nest a dragged row by the pointer's travel along the text, a level per indent, or by the inline arrows", () => {
    // drag Review's grip twenty pixels along the text in English and in Hebrew, then lift it by keys
    measureElements();
    const items: readonly TimelineItem[] = [
        { key: "launch", label: "Launch", start: 0, end: 6 * DAY_MS },
        { key: "design", label: "Design", parent: "launch", start: 0, end: 3 * DAY_MS },
        { key: "build", label: "Build", parent: "launch", start: 3 * DAY_MS, end: 6 * DAY_MS },
        { key: "review", label: "Review", start: 6 * DAY_MS, end: 7 * DAY_MS },
    ];
    const moves: TimelineRowMove[] = [];
    const nestIn = (tag: string, travel: number) => {
        const { container } = render(() => (
            <LocaleContext value={Localization.of(Locale.parse(tag), [])}>
                <Plan items={items} timeZone="UTC" onRowMove={(move) => moves.push(move)} />
            </LocaleContext>
        ));
        const grip = container.querySelector<HTMLElement>(
            "[data-slot=timeline-row][data-key=review] [data-slot=timeline-row-grip]",
        );
        const depths: (string | null)[] = [];
        for (const [type, x] of [
            ["pointerdown", 20],
            ["pointermove", 20 + travel / 2],
            ["pointermove", 20 + travel],
            ["pointerup", 20 + travel],
        ] as const) {
            grip?.dispatchEvent(
                new PointerEvent(type, {
                    bubbles: true,
                    cancelable: true,
                    clientX: x,
                    clientY: 3.5 * 32,
                    pointerId: 11,
                }),
            );
            flush();
            depths.push(
                container
                    .querySelector("[data-slot=timeline-row-drop]")
                    ?.getAttribute("data-depth") ?? null,
            );
        }

        return { container, grip, depths };
    };
    const english = nestIn("en-GB", 20).depths;
    const hebrew = nestIn("he-IL", -20).depths;

    // lift Review by its grip, nest it one level with the inline end arrow, and drop it
    const { container, grip } = nestIn("en-GB", 0);
    grip?.focus();
    press(container, " ");
    press(container, "ArrowRight");
    press(container, " ");

    // the drop line shows the depth while dragging, and each drop nests Review last under Launch
    expect({ english, hebrew, moves }).toEqual({
        // no line while the row would stay where it stands
        english: [null, null, "1", null],
        hebrew: [null, null, "1", null],
        moves: [
            { key: "review", parent: "launch", previous: "build" },
            { key: "review", parent: "launch", previous: "build" },
            { key: "review", parent: "launch", previous: "build" },
        ],
    });
});

/** Read how far a lens moved and grew between two places, in tenths of a percent. */
function shift(
    from: { readonly left: number; readonly width: number },
    to: { readonly left: number; readonly width: number },
): { readonly left: number; readonly width: number } {
    return {
        left: Math.round((to.left - from.left) * 10) / 10,
        width: Math.round((to.width - from.width) * 10) / 10,
    };
}

test("resize the window by the minimap lens's grips, and pan it by the lens between them", () => {
    // zoom in twice, drag the lens's start grip a hundred pixels back, then the lens forty forward
    measureElements();
    const { container } = render(() => (
        <Timeline items={PLAN} timeZone="UTC">
            <TimelineBody>
                <TimelineGrid />
                <TimelineNow />
                <TimelineLinkLayer />
                <TimelineRowList />
            </TimelineBody>
            <TimelineMinimap />
        </Timeline>
    ));
    container.querySelector<HTMLElement>("[role=row][tabindex='0']")?.focus();
    press(container, "+");
    press(container, "+");
    const lens = () => container.querySelector<HTMLElement>("[data-slot=timeline-minimap-lens]");
    const place = () => ({
        left: percent(lens() ?? undefined, "insetInlineStart"),
        width: percent(lens() ?? undefined, "width"),
    });
    const zoomed = place();
    drag(lens()?.querySelector("[data-edge=start]") ?? null, -100);
    const widened = place();
    drag(lens(), 40);
    const panned = place();

    // a hundred of eight hundred pixels is an eighth of the span, forty a twentieth
    expect({ widened: shift(zoomed, widened), panned: shift(widened, panned) }).toEqual({
        widened: { left: -12.5, width: 12.5 },
        panned: { left: 5, width: 0 },
    });
});

/** Push Build to start as a changed Design ends, as a schedule would. */
function pushBuild(changed: readonly TimelineItem[]): readonly TimelineItem[] {
    const design = changed.find((item) => item.key === "design");
    const build = nth(PLAN, 1);

    return design?.end === undefined
        ? changed
        : [...changed, { ...build, start: design.end, end: design.end + 7 * DAY_MS }];
}

test("hand on a gesture's changes once with what the caller propagates, previewed before the drop", () => {
    // push Build to start as Design ends, then move Design a day later by the keys
    measureElements();
    const sets: (readonly TimelineItem[])[] = [];
    const { container } = render(() => (
        <Plan
            items={PLAN}
            timeZone="UTC"
            propagate={pushBuild}
            onItemsChange={(items) => sets.push(items)}
        />
    ));
    container.querySelector<HTMLElement>("[role=row][tabindex='0']")?.focus();
    press(container, " ");
    press(container, "ArrowRight");
    const build = container.querySelectorAll<HTMLElement>("[data-slot=timeline-bar]")[1] ?? null;
    const preview = Math.round(startOf(build) * 10) / 10;
    press(container, " ");

    // the build shows from day 4 of the 10.4 days before the drop, and lands with design in one set
    expect({
        preview,
        sets: sets.map((set) => set.map((item) => [item.key, ...hoursOf(item)])),
    }).toEqual({
        preview: 40.4,
        sets: [
            [
                ["design", 24, 96],
                ["build", 96, 264],
            ],
        ],
    });
});

test("leave the toolbar's keys to it, leave a pressed preset pressed, and lift nothing on a read-only timeline", () => {
    // press Enter on today, the week preset twice, and Space on a row of a read-only timeline
    measureElements();
    const actions: string[] = [];
    const { container } = render(() => (
        <Timeline items={PLAN} timeZone="UTC" onAction={(key) => actions.push(key)}>
            <TimelineToolbar>
                <TimelineToday />
                <TimelineZoom />
            </TimelineToolbar>
            <TimelineHeader />
            <TimelineBody>
                <TimelineGrid />
                <TimelineNow />
                <TimelineLinkLayer />
                <TimelineRowList />
            </TimelineBody>
        </Timeline>
    ));
    const today = container.querySelector<HTMLElement>("[data-slot=timeline-today]");
    today?.focus();
    press(container, "Enter");
    const week = container.querySelector<HTMLElement>(
        "[data-slot=timeline-zoom] [data-value=week]",
    );
    week?.click();
    flush();
    week?.click();
    flush();
    container.querySelector<HTMLElement>("[role=row][tabindex='0']")?.focus();
    press(container, " ");

    expect({
        actions,
        week: week?.getAttribute("aria-checked"),
        lifted: container.querySelectorAll("[data-dragging]").length,
    }).toEqual({ actions: [], week: "true", lifted: 0 });
});

test("open a single milestone on a day at the least, and refuse items that loop through their parents", () => {
    // a launch alone, and two items each under the other
    measureElements();
    const { container } = render(() => (
        <Plan items={[{ key: "launch", label: "Launch", start: 0, end: 0 }]} timeZone="UTC" />
    ));
    const periods = [...container.querySelectorAll("[data-slot=timeline-axis-period]")].map(
        (period) => period.textContent,
    );
    const looped: readonly TimelineItem[] = [
        { key: "draft", label: "Draft", start: 0, end: DAY_MS, parent: "review" },
        { key: "review", label: "Review", start: 0, end: DAY_MS, parent: "draft" },
    ];

    const refused = render(() => (
        <Errored fallback={(error) => String(error())}>
            <Plan items={looped} timeZone="UTC" />
        </Errored>
    ));

    // the launch spans a day, and the cycle's refusal reaches the nearest error boundary
    expect([periods, refused.container.textContent]).toEqual([
        ["Wed, Dec 31", "Thu, Jan 1"],
        "RangeError: timeline items 'draft', 'review' loop through their parents",
    ]);
});

/** Read the x of the first link's two ends in a locale. */
function linkEndsIn(tag: string): number[] {
    const { container } = render(() => (
        <LocaleContext value={Localization.of(Locale.parse(tag), [])}>
            <Plan
                items={PLAN}
                links={[{ from: "design", to: "build", type: "finish-to-start", lag: 0 }]}
                timeZone="UTC"
            />
        </LocaleContext>
    ));
    const path = container.querySelector("[data-slot=timeline-link] path")?.getAttribute("d") ?? "";
    const numbers = path.match(/-?\d+(\.\d+)?/gu)?.map(Number) ?? [];

    return [Math.round(numbers[0] ?? 0), Math.round(numbers.at(-1) ?? 0)];
}

test("draw links mirrored right to left, from the inline end of the track", () => {
    // draw design's end to build's start, in English and in Hebrew
    measureElements();

    expect({ english: linkEndsIn("en-GB"), hebrew: linkEndsIn("he-IL") }).toEqual({
        english: [246, 246],
        hebrew: [554, 554],
    });
});

test("route a link whose target starts before its source ends along the target row's top", () => {
    // review ends after build starts, so the link turns at the top of build's row
    measureElements();
    const items: readonly TimelineItem[] = [
        { key: "review", label: "Review", start: 3 * DAY_MS, end: 6 * DAY_MS },
        { key: "build", label: "Build", start: 0, end: 2 * DAY_MS },
    ];
    const { container } = render(() => (
        <Plan
            items={items}
            links={[{ from: "review", to: "build", type: "finish-to-start", lag: 0 }]}
            timeZone="UTC"
        />
    ));
    const path = container.querySelector("[data-slot=timeline-link] path")?.getAttribute("d") ?? "";
    const steps = path.match(/[MHV] -?[\d.]+( -?[\d.]+)?/gu) ?? [];

    // out of review's row middle, down to build's top, back past build's start, then down into it
    expect(steps.map((step) => (step.startsWith("V") ? step : step.charAt(0)))).toEqual([
        "M",
        "H",
        "V 32",
        "H",
        "V 48",
        "H",
    ]);
});

test("draw no link the caller refuses, the arrows stepping past its edges to the next row", () => {
    // refuse links into Build, then draw from Design's end down to Launch's start by keys
    measureElements();
    const links: ScheduleLink[] = [];
    const { container } = render(() => (
        <Plan
            items={PLAN}
            timeZone="UTC"
            canLink={(link) => link.to !== "build"}
            onLinkCreate={(link) => links.push(link)}
        />
    ));
    container.querySelector<HTMLElement>("[role=row][tabindex='0']")?.focus();
    for (const key of ["l", "ArrowDown", "ArrowLeft", " "]) {
        press(container, key);
    }

    expect(links).toEqual([{ from: "design", to: "launch", type: "finish-to-start", lag: 0 }]);
});

test("highlight a focused link and its bars, remove it with Delete, and draw each type from its edges", () => {
    // a link of each type, each between its own pair, the finish-to-start one focused and deleted
    measureElements();
    const deleted: ScheduleLink[] = [];
    const items = [...PLAN, { key: "review", label: "Review", start: 3 * DAY_MS, end: 5 * DAY_MS }];
    const links: readonly ScheduleLink[] = [
        { from: "design", to: "build", type: "finish-to-start", lag: 0 },
        { from: "design", to: "review", type: "start-to-start", lag: 0 },
        { from: "build", to: "launch", type: "finish-to-finish", lag: 0 },
        { from: "review", to: "launch", type: "start-to-finish", lag: 0 },
    ];
    const { container } = render(() => (
        <Plan
            items={items}
            links={links}
            timeZone="UTC"
            onLinkDelete={(link) => deleted.push(link)}
        />
    ));
    const paths = [...container.querySelectorAll<SVGElement>("[data-slot=timeline-link]")];
    paths[0]?.focus();
    flush();
    const highlighted = {
        links: container.querySelectorAll("[data-slot=timeline-link][data-highlighted]").length,
        bars: container.querySelectorAll("[data-slot=timeline-bar][data-linked]").length,
    };
    press(container, "Delete");

    // each path starts at the source edge's x and ends at the target edge's
    const ends = paths.map((link) => {
        const path = link.querySelector("path")?.getAttribute("d") ?? "";
        const numbers = path.match(/-?\d+(\.\d+)?/gu)?.map(Number) ?? [];

        return [Math.round(numbers[0] ?? 0), Math.round(numbers.at(-1) ?? 0)];
    });

    expect({
        highlighted,
        deleted,
        labels: paths.map((link) => link.getAttribute("aria-label")),
        ends,
    }).toEqual({
        highlighted: { links: 1, bars: 2 },
        deleted: [{ from: "design", to: "build", type: "finish-to-start", lag: 0 }],
        labels: [
            "\u2068Design\u2069 to \u2068Build\u2069",
            "\u2068Design\u2069 to \u2068Review\u2069",
            "\u2068Build\u2069 to \u2068Launch\u2069",
            "\u2068Review\u2069 to \u2068Launch\u2069",
        ],
        ends: [
            [246, 246],
            [15, 246],
            [785, 793],
            [246, 793],
        ],
    });
});

test("move a bar's start alone by its handle, and draw a milestone as a diamond and progress as a fill", () => {
    // drag Design's start a hundred pixels left: 31.2 hours, snapped to a day
    measureElements();
    const changes: TimelineItem[] = [];
    const { container } = render(() => (
        <Plan items={PLAN} timeZone="UTC" onItemsChange={(items) => changes.push(...items)} />
    ));
    drag(container.querySelector("[data-slot=timeline-bar-handle][data-edge=start]"), -100);
    const bars = [...container.querySelectorAll<HTMLElement>("[data-slot=timeline-bar]")];

    expect({
        changes: changes.map(hoursOf),
        milestones: bars.map((bar) => bar.dataset["milestone"] !== undefined),
        progress: bars.map(
            (bar) =>
                bar
                    .querySelector<HTMLElement>("[data-slot=timeline-bar-progress]")
                    ?.style.getPropertyValue("--x-width") ?? null,
        ),
        startHandles: container.querySelectorAll("[data-slot=timeline-bar-handle][data-edge=start]")
            .length,
    }).toEqual({
        changes: [[-24, 72]],
        milestones: [false, false, true],
        progress: [null, "50%", null],
        startHandles: 2,
    });
});

test("zoom to a week, a month or a quarter per column, bring today to the middle, and shade the weekends", () => {
    // a sprint from Monday 16 November 2026 in UTC, its margins reaching into Sunday 15, with its toolbar
    measureElements();
    const monday = Date.UTC(2026, 10, 16);
    const week: readonly TimelineItem[] = [
        { key: "sprint", label: "Sprint", start: monday, end: monday + 7 * DAY_MS },
    ];
    const { container } = render(() => (
        <Timeline items={week} timeZone="UTC">
            <TimelineToday />
            <TimelineZoom />
            <TimelineHeader>
                <TimelineAxis />
            </TimelineHeader>
            <TimelineBody>
                <TimelineGrid />
                <TimelineNow />
                <TimelineLinkLayer />
                <TimelineRowList />
            </TimelineBody>
        </Timeline>
    ));
    const pressed = () =>
        [...container.querySelectorAll<HTMLElement>("[data-slot=timeline-zoom] button")]
            .filter((button) => button.getAttribute("aria-checked") === "true")
            .map((button) => button.textContent);
    const shaded = container.querySelectorAll("[data-slot=timeline-non-working]").length;
    const before = pressed();
    const buttons = [
        ...container.querySelectorAll<HTMLElement>("[data-slot=timeline-zoom] button"),
    ];
    buttons.find((button) => button.textContent === "Month")?.click();
    flush();
    const month = pressed();
    container.querySelector<HTMLElement>("[data-slot=timeline-today]")?.click();
    flush();

    expect({
        shaded,
        before,
        month,
        isToday: container.querySelector("[data-slot=timeline-now]") !== null,
    }).toEqual({
        // shade each weekend as one span
        shaded: 2,
        before: [],
        month: ["Month"],
        isToday: true,
    });
});

test("shade a range down the track, mark a bar late past its deadline, and draw milestones and what is beside a bar", () => {
    // a cycle over the first days, a build due before it ends with a beta along it, and copy due after
    measureElements();
    const items: readonly TimelineItem[] = [
        {
            key: "build",
            label: "Build",
            start: 3 * DAY_MS,
            end: 10 * DAY_MS,
            deadline: 8 * DAY_MS,
            milestones: [{ key: "beta", label: "Beta", at: 5 * DAY_MS }],
        },
        { key: "copy", label: "Copy", start: 0, end: 2 * DAY_MS, deadline: 5 * DAY_MS },
    ];
    const { container } = render(() => (
        <Timeline items={items} timeZone="UTC">
            <TimelineBody>
                <TimelineRange start={DAY_MS} end={4 * DAY_MS} color="purple">
                    Cycle 12
                </TimelineRange>
                <TimelineRowList>
                    {() => (
                        <TimelineRow>
                            <TimelineRowLabel />
                            <TimelineBar before="Started" after="Ada" />
                        </TimelineRow>
                    )}
                </TimelineRowList>
            </TimelineBody>
        </Timeline>
    ));
    const all = (slot: string) => [...container.querySelectorAll(`[data-slot=${slot}]`)];

    expect({
        range: all("timeline-range").map((range) => [
            range.textContent,
            range.getAttribute("data-color"),
        ]),
        late: all("timeline-bar").map((bar) => bar.hasAttribute("data-late")),
        deadlines: all("timeline-bar-deadline").length,
        milestones: all("timeline-bar-milestone").map((milestone) =>
            milestone.getAttribute("aria-label"),
        ),
        beside: [all("timeline-bar-before"), all("timeline-bar-after")].map((each) =>
            each.map((element) => element.textContent),
        ),
    }).toEqual({
        range: [["Cycle 12", "purple"]],
        late: [true, false],
        deadlines: 2,
        milestones: ["Beta"],
        beside: [
            ["Started", "Started"],
            ["Ada", "Ada"],
        ],
    });
});

test("lay a column out beside the labels in the header and every row, the track and its lines after it", () => {
    // a lead column six rem wide, with a marker on the track
    measureElements();
    const { container } = render(() => (
        <Timeline items={PLAN} timeZone="UTC">
            <TimelineHeader corner="Task">
                <TimelineColumn width="6rem">Lead</TimelineColumn>
                <TimelineAxis />
            </TimelineHeader>
            <TimelineBody>
                <TimelineMarker at={DAY_MS}>Kickoff</TimelineMarker>
                <TimelineRowList>
                    {(node) => (
                        <TimelineRow>
                            <TimelineRowLabel />
                            <TimelineCell>
                                {node().item.key === "build" ? "Ada" : "Grace"}
                            </TimelineCell>
                            <TimelineBar />
                        </TimelineRow>
                    )}
                </TimelineRowList>
            </TimelineBody>
        </Timeline>
    ));
    const template = (selector: string) =>
        container
            .querySelector<HTMLElement>(selector)
            ?.style.getPropertyValue("--x-gridTemplateColumns");

    expect({
        header: template("[data-slot=timeline-header]"),
        row: template("[data-slot=timeline-row]"),
        column: container.querySelector("[role=columnheader]")?.textContent,
        cells: [...container.querySelectorAll("[data-slot=timeline-cell]")].map(
            (cell) => cell.textContent,
        ),
        marker: container
            .querySelector<HTMLElement>("[data-slot=timeline-marker]")
            ?.style.getPropertyValue("--x-insetInlineStart")
            .startsWith("calc(calc(16rem + 6rem)"),
    }).toEqual({
        header: "16rem 6rem minmax(0, 1fr)",
        row: "16rem 6rem minmax(0, 1fr)",
        column: "Lead",
        cells: ["Grace", "Ada", "Grace"],
        marker: true,
    });
});

test("draw items on the rows they name, packed into lanes where they overlap, each row as high as its lanes", () => {
    // two overlapping tasks and a later one on Ada, and one task on Grace
    measureElements();
    const items: readonly TimelineItem[] = [
        { key: "ada", label: "Ada" },
        { key: "grace", label: "Grace" },
        { key: "design", label: "Design", row: "ada", start: 0, end: 3 * DAY_MS },
        { key: "build", label: "Build", row: "ada", start: 2 * DAY_MS, end: 6 * DAY_MS },
        { key: "launch", label: "Launch", row: "ada", start: 4 * DAY_MS, end: 5 * DAY_MS },
        { key: "copy", label: "Copy", row: "grace", start: DAY_MS, end: 4 * DAY_MS },
    ];
    const { container } = render(() => <Plan items={items} timeZone="UTC" rowHeight={30} />);
    const rows = [...container.querySelectorAll<HTMLElement>("[data-slot=timeline-row]")];

    expect(
        rows.map((row) => ({
            label: row.querySelector("[data-slot=timeline-row-label]")?.textContent,
            height: row.style.getPropertyValue("--x-height"),
            bars: [...row.querySelectorAll("[data-slot=timeline-lane]")].map((lane) => [
                lane.querySelector("[data-slot=timeline-bar]")?.textContent,
                lane.getAttribute("data-lane"),
            ]),
        })),
    ).toEqual([
        {
            label: "Ada",
            height: "60px",
            bars: [
                ["Design", "0"],
                ["Launch", "0"],
                ["Build", "1"],
            ],
        },
        { label: "Grace", height: "30px", bars: [["Copy", "0"]] },
    ]);
});

test("move an item drawn on another row onto the row it is dropped on, and in time", () => {
    // drag Ada's design down onto Grace's row, which starts after Ada's two lanes of 32 pixels
    measureElements();
    const changes: TimelineItem[] = [];
    const items: readonly TimelineItem[] = [
        { key: "ada", label: "Ada" },
        { key: "grace", label: "Grace" },
        { key: "design", label: "Design", row: "ada", start: 0, end: 3 * DAY_MS },
        { key: "build", label: "Build", row: "ada", start: 2 * DAY_MS, end: 6 * DAY_MS },
    ];
    const { container } = render(() => (
        <Plan items={items} timeZone="UTC" onItemsChange={(changed) => changes.push(...changed)} />
    ));
    const bar = container.querySelector("[data-slot=timeline-bar]");
    for (const [type, x, y] of [
        ["pointerdown", 200, 10],
        ["pointermove", 300, 80],
        ["pointerup", 300, 80],
    ] as const) {
        bar?.dispatchEvent(
            new PointerEvent(type, {
                bubbles: true,
                cancelable: true,
                clientX: x,
                clientY: y,
                pointerId: 5,
                width: 1,
                height: 1,
            }),
        );
        flush();
    }

    // the hundred pixels snap to the axis's next mark, a day later
    expect(changes.map((item) => [item.key, item.row, (item.start ?? 0) / 3_600_000])).toEqual([
        ["design", "grace", 24],
    ]);
});

/** Read a percentage a bar is placed by, such as its start or width. */
function percent(bar: HTMLElement | undefined, name: string): number {
    return Number.parseFloat(bar?.style.getPropertyValue(`--x-${name}`) ?? "");
}

test("leave non-working days out of the track on request, the work on either side meeting", () => {
    // a Friday and a Monday of November 2026, the weekend between them hidden, two working days shown
    measureElements();
    const friday = Date.UTC(2026, 10, 6);
    const saturday = Date.UTC(2026, 10, 7);
    const monday = Date.UTC(2026, 10, 9);
    const items: readonly TimelineItem[] = [
        { key: "close", label: "Close the sprint", start: friday, end: friday + DAY_MS },
        { key: "plan", label: "Plan the next", start: monday, end: monday + DAY_MS },
    ];
    const changes: TimelineItem[] = [];
    const { container } = render(() => (
        <Plan
            items={items}
            timeZone="UTC"
            defaultWindow={{ start: friday, end: monday + DAY_MS }}
            hideNonWorking
            snap="day"
            onItemsChange={(changed) => changes.push(...changed)}
        />
    ));
    const bars = [...container.querySelectorAll<HTMLElement>("[data-slot=timeline-bar]")];
    const places = bars.map((bar) => [percent(bar, "insetInlineStart"), percent(bar, "width")]);

    // drag the Friday a working day later, past the weekend onto Monday
    drag(nth(bars, 0), 400);

    // show a window inside the weekend from the Monday after it
    const weekend = render(() => (
        <Plan
            items={items}
            timeZone="UTC"
            window={{ start: saturday, end: saturday + DAY_MS }}
            hideNonWorking
        />
    ));
    const inside = weekend.container.querySelector<HTMLElement>("[data-slot=timeline-bar]");

    expect({
        places,
        marks: [...container.querySelectorAll("[data-slot=timeline-axis-mark]")].map(
            (mark) => mark.textContent,
        ),
        shaded: container.querySelectorAll("[data-slot=timeline-non-working]").length,
        moved: changes.map((item) => [item.start, item.end]),
        inside: [
            percent(inside ?? undefined, "insetInlineStart"),
            percent(inside ?? undefined, "width"),
        ],
    }).toEqual({
        places: [
            [0, 50],
            [50, 50],
        ],
        // marks every six hours at two working days per track, the weekend's folded into Monday's
        marks: ["12 AM", "6 AM", "12 PM", "6 PM", "12 AM", "6 AM", "12 PM", "6 PM", "12 AM"],
        shaded: 0,
        moved: [[monday, monday + DAY_MS]],
        inside: [-100, 100],
    });
});

test("leave nights out of the track by the owner's working hours", () => {
    // work nine to five on Monday and nine to noon on Tuesday, over a window from Monday to Wednesday
    measureElements();
    const monday = Date.UTC(2026, 10, 9);
    const hour = 3_600_000;
    const items: readonly TimelineItem[] = [
        { key: "work", label: "Work", start: monday + 9 * hour, end: monday + 17 * hour },
        { key: "wrap", label: "Wrap", start: monday + 33 * hour, end: monday + 36 * hour },
    ];
    const { container } = render(() => (
        <Plan
            items={items}
            timeZone="UTC"
            defaultWindow={{ start: monday, end: monday + 2 * DAY_MS }}
            hideNonWorking
            workWeek={{
                1: [{ start: "09:00", end: "17:00" }],
                2: [{ start: "09:00", end: "12:00" }],
            }}
        />
    ));
    const bars = [...container.querySelectorAll<HTMLElement>("[data-slot=timeline-bar]")];

    // eleven working hours fill the track: eight of Monday, then Tuesday's three
    expect(bars.map((bar) => [percent(bar, "insetInlineStart"), percent(bar, "width")])).toEqual([
        [0, (8 / 11) * 100],
        [(8 / 11) * 100, (3 / 11) * 100],
    ]);
});

test("keep marks that fall on hidden days, drawn at the working day after them", () => {
    // twelve weeks of an American plan, whose week marks fall on Sundays, its weekends hidden
    measureElements();
    const monday = Date.UTC(2026, 10, 2);
    const { container } = render(() => (
        <LocaleContext value={Localization.of(Locale.parse("en-US"), [])}>
            <Plan
                items={PLAN}
                timeZone="UTC"
                defaultWindow={{ start: monday, end: monday + 84 * DAY_MS }}
                hideNonWorking
            />
        </LocaleContext>
    ));

    // every Sunday's week mark stays, drawn where Monday starts
    expect(
        [...container.querySelectorAll("[data-slot=timeline-axis-mark]")].map(
            (mark) => mark.textContent,
        ),
    ).toEqual(["8", "15", "22", "29", "6", "13", "20", "27", "3", "10", "17", "24"]);
});

test("count hidden days out of ends, steps, pans and moves, a task keeping its working length", () => {
    // a Friday and a Monday task over a window of two working days, eight hundred pixels wide
    measureElements();
    const friday = Date.UTC(2026, 10, 6);
    const monday = Date.UTC(2026, 10, 9);
    const hour = 3_600_000;
    const items: readonly TimelineItem[] = [
        { key: "close", label: "Close the sprint", start: friday, end: friday + DAY_MS },
        { key: "plan", label: "Plan the next", start: monday, end: monday + DAY_MS },
    ];
    const plan = (snap: TickUnit | false) => {
        const changes: TimelineItem[] = [];
        const windows: TimeScale[] = [];
        const { container } = render(() => (
            <Plan
                items={items}
                timeZone="UTC"
                defaultWindow={{ start: friday, end: monday + DAY_MS }}
                hideNonWorking
                snap={snap}
                onItemsChange={(changed) => changes.push(...changed)}
                onWindowChange={(window) => windows.push(window)}
            />
        ));
        const row = (key: string) =>
            container.querySelector<HTMLElement>(`[data-slot=timeline-row][data-key=${key}]`);

        return { container, changes, windows, row };
    };

    // stretch Friday's end a tenth of a day into Monday, which snaps back to Friday's end and changes nothing
    const ended = plan("day");
    drag(
        ended.row("close")?.querySelector("[data-slot=timeline-bar-handle][data-edge=end]") ?? null,
        40,
    );

    // step Monday's task back an hour with the keys, past the weekend onto Friday's last hour
    const stepped = plan("hour");
    stepped.row("plan")?.focus();
    press(stepped.container, " ");
    press(stepped.container, "ArrowLeft");
    press(stepped.container, " ");

    // pan a quarter of the window, half a working day, and move Friday's task half a day on
    const panned = plan(false);
    panned.row("close")?.focus();
    press(panned.container, "ArrowRight", { altKey: true });
    drag(panned.row("close")?.querySelector("[data-slot=timeline-bar]") ?? null, 200);

    expect({
        ended: ended.changes.map((item) => [item.start, item.end]),
        stepped: stepped.changes.map((item) => [item.start, item.end]),
        panned: panned.windows,
        moved: panned.changes.map((item) => [item.start, item.end]),
    }).toEqual({
        ended: [],
        stepped: [[friday + 23 * hour, monday + 23 * hour]],
        panned: [{ start: friday + 12 * hour, end: monday + DAY_MS + 12 * hour }],
        // the half-day move keeps one working day, ending on Monday's noon
        moved: [[friday + 12 * hour, monday + 12 * hour]],
    });
});

test("snap a moved bar to the axis's marks, to a unit the owner names, or to nothing", () => {
    // a hundred of eight hundred pixels over the 10.4 days shown, which is 31.2 hours
    measureElements();
    const plan: readonly TimelineItem[] = [
        { key: "design", label: "Design", start: 0, end: 3 * DAY_MS },
        { key: "build", label: "Build", start: 3 * DAY_MS, end: 10 * DAY_MS },
    ];
    const movedBy = (snap: TimelineProperties["snap"]) => {
        const changes: TimelineItem[] = [];
        const { container } = render(() => (
            <Plan
                items={plan}
                timeZone="UTC"
                {...(snap === undefined ? {} : { snap })}
                onItemsChange={(items) => changes.push(...items)}
            />
        ));
        drag(container.querySelector("[data-slot=timeline-bar]"), 100);

        return changes.map((item) => Math.round(((item.start ?? 0) / 3_600_000) * 10) / 10);
    };

    expect({ marks: movedBy(undefined), hours: movedBy("hour"), free: movedBy(false) }).toEqual({
        marks: [24],
        hours: [31],
        free: [31.2],
    });
});

test("zoom to the span a sweep across the axis covers, and leave a click on it alone", () => {
    // sweep from a quarter of the way along the axis to half way, then click it
    measureElements();
    const windows: TimeScale[] = [];
    const start = 0;
    const end = 8 * DAY_MS;
    const { container } = render(() => (
        <Plan
            items={PLAN}
            timeZone="UTC"
            defaultWindow={{ start, end }}
            onWindowChange={(window) => windows.push(window)}
        />
    ));
    const axis = container.querySelector("[data-slot=timeline-axis]");
    drag(axis, 200);
    drag(axis, 2);

    expect(windows.map((window) => [window.start / DAY_MS, window.end / DAY_MS])).toEqual([[2, 4]]);
});

test("fold every parent and unfold them again, by the tree's verbs and the asterisk, a folded parent drawing its children small", () => {
    // a launch with design and build under it, folded and unfolded by the tree's verbs
    measureElements();
    const items: readonly TimelineItem[] = [
        { key: "launch", label: "Launch" },
        { key: "design", label: "Design", parent: "launch", start: 0, end: 3 * DAY_MS },
        {
            key: "build",
            label: "Build",
            parent: "launch",
            start: 3 * DAY_MS,
            end: 9 * DAY_MS,
            color: "green",
        },
    ];
    let control: TimelineControl | undefined;
    const { container } = render(() => (
        <Timeline items={items} timeZone="UTC">
            <TimelineBody>
                <TimelineRowList />
            </TimelineBody>
            <Probe read={(read) => (control = read)} />
        </Timeline>
    ));
    const state = () => ({
        rows: rowsOf(container).length,
        rollups: container.querySelectorAll("[data-slot=timeline-rollup]").length,
    });
    control?.tree.collapseAll();
    flush();
    const collapsed = state();
    control?.tree.expandAll();
    flush();
    const expanded = state();
    control?.tree.collapseAll();
    flush();
    container.querySelector<HTMLElement>("[data-slot=timeline-row]")?.focus();
    press(container, "*");

    expect({ collapsed, expanded, asterisk: state() }).toEqual({
        collapsed: { rows: 1, rollups: 2 },
        expanded: { rows: 3, rollups: 0 },
        asterisk: { rows: 3, rollups: 0 },
    });
});

/** Hand a test the nearest timeline's control. */
function Probe(properties: { readonly read: (control: TimelineControl) => void }): JSX.Element {
    properties.read(useTimeline());

    return null;
}

test("draw a bar's slack after it, and set how much is done by its grip's arrow keys", () => {
    // a build with two days of slack, half done, its grip stepped right, to the end and to the start
    measureElements();
    const changes: TimelineItem[] = [];
    const items: readonly TimelineItem[] = [
        {
            key: "build",
            label: "Build",
            start: 0,
            end: 4 * DAY_MS,
            slack: 2 * DAY_MS,
            progress: 0.5,
        },
    ];
    const { container } = render(() => (
        <Plan items={items} timeZone="UTC" onItemsChange={(changed) => changes.push(...changed)} />
    ));
    const grip = container.querySelector<HTMLElement>("[data-slot=timeline-bar-progress-grip]");
    grip?.focus();
    press(container, "ArrowRight");
    press(container, "End");
    press(container, "Home");
    const slack = container.querySelector<HTMLElement>("[data-slot=timeline-bar-slack]");

    expect({
        slack: [
            percent(slack ?? undefined, "insetInlineStart"),
            percent(slack ?? undefined, "width"),
        ],
        grip: [
            grip?.getAttribute("role"),
            grip?.getAttribute("aria-valuenow"),
            grip?.getAttribute("aria-valuetext"),
        ],
        done: changes.map((item) => item.progress),
    }).toEqual({
        // the slack runs from the end at 4.08 of 4.16 days shown, two days long
        slack: [98.076_923_076_923_07, 48.076_923_076_923_07],
        grip: ["slider", "50", "50%"],
        done: [0.6, 1, 0],
    });
});

test("drag how much is done along a bar by its grip's pointer, mirrored right to left", () => {
    // a four-day build half done, its grip dragged to three quarters of the track in each direction
    measureElements();
    const items: readonly TimelineItem[] = [
        { key: "build", label: "Build", start: 0, end: 4 * DAY_MS, progress: 0.5 },
    ];
    const doneIn = (tag: string, x: number) => {
        const changes: TimelineItem[] = [];
        const { container } = render(() => (
            <LocaleContext value={Localization.of(Locale.parse(tag), [])}>
                <Plan
                    items={items}
                    timeZone="UTC"
                    onItemsChange={(changed) => changes.push(...changed)}
                />
            </LocaleContext>
        ));
        const grip = container.querySelector("[data-slot=timeline-bar-progress-grip]");
        for (const [type, clientX] of [
            ["pointerdown", 400],
            ["pointermove", (400 + x) / 2],
            ["pointermove", x],
            ["pointerup", x],
        ] as const) {
            grip?.dispatchEvent(
                new PointerEvent(type, {
                    bubbles: true,
                    cancelable: true,
                    clientX,
                    clientY: 10,
                    pointerId: 13,
                }),
            );
            flush();
        }

        return changes.map((item) => item.progress);
    };

    // the track shows 4.16 days from -0.08, so three quarters of it is 76% of the bar
    expect({ english: doneIn("en-GB", 600), hebrew: doneIn("he-IL", 200) }).toEqual({
        english: [0.76],
        hebrew: [0.76],
    });
});

test("zoom around the pointer by a pinch and pan by the wheel, mirrored right to left", () => {
    // pinch at the track's left edge and wheel eighty pixels along, in English and in Hebrew
    measureElements();
    const start = 0;
    const end = 8 * DAY_MS;
    const windowsIn = (tag: string) => {
        const windows: TimeScale[] = [];
        const { container } = render(() => (
            <LocaleContext value={Localization.of(Locale.parse(tag), [])}>
                <Plan
                    items={PLAN}
                    timeZone="UTC"
                    window={{ start, end }}
                    onWindowChange={(window) => windows.push(window)}
                />
            </LocaleContext>
        ));
        const body = container.querySelector("[data-slot=timeline-body]");

        // pinch a trackpad at the left edge, setting what the test DOM's constructor drops
        const pinch = new WheelEvent("wheel", { bubbles: true, cancelable: true, deltaY: -100 });
        Object.defineProperty(pinch, "ctrlKey", { value: true });
        Object.defineProperty(pinch, "clientX", { value: 0 });
        body?.dispatchEvent(pinch);
        flush();

        // wheel eighty pixels along the axis
        body?.dispatchEvent(
            new WheelEvent("wheel", { bubbles: true, cancelable: true, deltaX: 80 }),
        );
        flush();

        return windows.map((window) => [
            Math.round(window.start / 3_600_000),
            Math.round(window.end / 3_600_000),
        ]);
    };

    // the pinch keeps the edge under the pointer, the window's start in English and its end in Hebrew
    expect({ english: windowsIn("en-GB"), hebrew: windowsIn("he-IL") }).toEqual({
        english: [
            [0, 71],
            [19, 211],
        ],
        hebrew: [
            [121, 192],
            [-19, 173],
        ],
    });
});

test("open a link with Enter or a double click, and keep every option off the timeline's element", () => {
    // a link from design to build, opened by its key and by the pointer, with every option set
    measureElements();
    const opened: ScheduleLink[] = [];
    const links: readonly ScheduleLink[] = [
        { from: "design", to: "build", type: "finish-to-start", lag: 0 },
    ];
    const { container } = render(() => (
        <Plan
            items={PLAN}
            links={links}
            timeZone="UTC"
            snap="day"
            hideNonWorking
            workWeek={{ 1: [{ start: "09:00", end: "17:00" }] }}
            rowHeight={32}
            labelWidth="12rem"
            onLinkAction={(link) => opened.push(link)}
            onLinkDelete={() => undefined}
            onLinkCreate={() => undefined}
            canLink={() => true}
            onItemsChange={() => undefined}
            propagate={(changed) => changed}
            onRowMove={() => undefined}
            onAction={() => undefined}
            onWindowChange={() => undefined}
            onLabelWidthChange={() => undefined}
        />
    ));
    const link = container.querySelector<SVGElement>("[data-slot=timeline-link]");
    link?.focus();
    press(container, "Enter");
    link?.dispatchEvent(new MouseEvent("dblclick", { bubbles: true }));
    const root = container.querySelector("[data-slot=timeline]");

    expect({
        opened: opened.map((each) => `${each.from} to ${each.to}`),
        shortcuts: link?.getAttribute("aria-keyshortcuts"),
        attributes: root
            ?.getAttributeNames()
            .filter(
                (name) =>
                    !name.startsWith("data-") &&
                    name !== "class" &&
                    name !== "style" &&
                    name !== "role" &&
                    name !== "aria-label",
            ),
    }).toEqual({
        opened: ["design to build", "design to build"],
        shortcuts: "Delete Backspace Enter",
        attributes: [],
    });
});
