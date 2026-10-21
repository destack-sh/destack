import { color, font } from "@destack/theme/tokens.stylex";
import { media } from "@destack/style/media.stylex";
import { present } from "@destack/schema";
import * as style from "@destack/style";
import { createEffect, createSignal, For } from "@destack/view";

import { palette } from "../../palette.stylex";
import { AppGlyph, AppHeader, AppRail, appStyles, appText } from "../figure/app";
import type { Connector } from "../command/chat";
import { type Item, type Moment, Scene } from "../command/scene";
import {
    askers,
    depthOf,
    durationOf,
    type Event,
    INES_HOLD,
    type Lane,
    ROOT_SPAN,
    runOf,
    type Service,
    type Span,
    stampOf,
    Trace,
    TRACE_LENGTH,
} from "./trace";
import { typeScale } from "../../layout/type.stylex";

/** A browser narrower than a phone, where the map stacks its cards in a grid. */
const NARROW = "@container (max-width: 479px)";

/** The milliseconds the trace draws across, the scrubber's whole width. */
export const RUN_LENGTH = TRACE_LENGTH;

/** The trace's width in milliseconds, as the view measures time. */
const AXIS = TRACE_LENGTH;

/** Your question about the board and the calendar. */
const ask: Item = {
    kind: "ask",
    text: "why is Ines on the calendar twice? the launch board has 9 new cards too",
};

/** Your agent reading what ran after the approval. */
const readRuns: Item = {
    kind: "call",
    server: "destack",
    running: "Reading the runs after 09:00",
    title: "Read launch-askers runs",
    request: 'client.query.run.findMany({ where: { workflow: "launch-askers", after: "09:00" } })',
    response: "9 runs · all set off by tx-91c · approve 400 signups",
    isFailed: false,
    isExpanded: false,
};

/** The live view of the duplicates, built in your space. */
const duplicates: Item = {
    kind: "app",
    title: "Duplicates from tx-91c",
    badge: "Live · 3 apps",
    rows: [
        ["Ines · Orbit Labs", "DST-31 · post · Wed 11:00", "2nd"],
        ["Jonas · Kettle", "DST-32 · post · Thu 14:00", "2nd"],
        ["Priya · Nair Studio", "DST-33 · post · Thu 15:00", "2nd"],
        ["… 6 more askers", "task · post · hold", "2nd"],
    ],
    note: "27 duplicate writes · 9 askers · one transaction",
    isLive: true,
};

/** The same live view once the duplicates are undone. */
const duplicatesUndone: Item = {
    kind: "app",
    title: "Duplicates from tx-91c",
    badge: "Live · 3 apps",
    rows: [
        ["Ines · Orbit Labs", "DST-31 · post · Wed 11:00", "undone"],
        ["Jonas · Kettle", "DST-32 · post · Thu 14:00", "undone"],
        ["Priya · Nair Studio", "DST-33 · post · Thu 15:00", "undone"],
        ["… 6 more askers", "task · post · hold", "undone"],
    ],
    note: "27 writes undone · nothing else touched",
    isLive: true,
};

/** Your agent's fix to the trigger. */
const editTrigger: Item = {
    kind: "call",
    server: "destack",
    running: "Editing launch.ts",
    title: "Edited launch.ts:2",
    request:
        '- .on(signup.on("update"), (signup) => signup.asksForDemo)\n+ .on(signup.on("insert"), (signup) => signup.asksForDemo)',
    response: "on branch fix/askers-insert",
    isFailed: false,
    isExpanded: true,
};

/** Your agent replaying the approval on the branch. */
const replayApproval: Item = {
    kind: "call",
    server: "destack",
    running: "Replaying tx-91c",
    title: "Replayed tx-91c on the branch",
    request: 'client.transaction.replay({ id: "tx-91c", branch: "fix/askers-insert" })',
    response: "400 rows updated · 0 runs · 0 writes",
    isFailed: false,
    isExpanded: false,
};

/** Your agent undoing the nine runs. */
const undoRuns: Item = {
    kind: "call",
    server: "destack",
    running: "Undoing 9 runs",
    title: "Undid the 9 runs from tx-91c",
    request: 'client.run.undo({ trigger: "tx-91c" })',
    response: "9 tasks · 9 posts · 9 holds removed · 0 conflicts",
    isFailed: false,
    isExpanded: false,
};

/** The conversation in your space, in order. */
const items: readonly Item[] = [
    ask,
    readRuns,
    {
        kind: "reply",
        text: "Approving 400 signups updated their rows, and launch-askers runs on every update. It ran again for the 9 askers it had already boarded. Here are the duplicates:",
    },
    duplicates,
    editTrigger,
    {
        kind: "approval",
        text: "wants to replay tx-91c on fix/askers-insert. Nothing is written.",
        outcome: "Allowed · replay on a branch, nothing written",
        grant: "once",
    },
    replayApproval,
    {
        kind: "reply",
        text: "On the branch the approval runs nothing. Ship it and undo the 9 runs? Everything else since 09:00 stays as it is.",
    },
    {
        kind: "approval",
        text: "wants to ship fix/askers-insert and undo the 9 runs from tx-91c: 27 writes.",
        outcome: "Shipped · release 16 · 9 runs undone",
        grant: "once",
    },
    undoRuns,
];

/** The conversation in your space, its steps placed on one clock with the run. */
export const destackedScene = new Scene(
    items.map((item) => ({ item, row: 0, change: "start" as const })),
);

/** The conversation through the vendors' connectors: the cause found, the cleanup by hand. */
export const stackedScene = new Scene(
    (
        [
            ask,
            {
                kind: "call",
                server: "zapier",
                running: "Reading Zap history",
                title: "Read Zap history",
                request: 'zap_runs({ zap: "Launch askers", since: "09:00" })',
                response: "9 runs at 09:00 · trigger Updated Record in Airtable",
                isFailed: false,
                isExpanded: false,
            },
            {
                kind: "call",
                server: "airtable",
                running: "Reading Signups",
                title: "Read Signups",
                request: 'list_records({ table: "Signups", modifiedAfter: "09:00" })',
                response: "400 records modified at 09:00 by Florian",
                isFailed: false,
                isExpanded: false,
            },
            {
                kind: "reply",
                text: "Approving 400 signups in Airtable modified their records, and the Zap triggers on Updated Record, so it ran again for 9 askers.",
            },
            {
                kind: "call",
                server: "trello",
                running: "Searching the board",
                title: "Searched Launch board",
                request: 'search_cards({ board: "Launch", query: "demo" })',
                response: "18 cards · 9 created twice at 09:00",
                isFailed: false,
                isExpanded: false,
            },
            {
                kind: "reply",
                text: "I can delete the 9 cards and ask someone to remove the Slack posts, but Zapier can't undo a run, and Google Calendar already emailed Ines a second invite.",
            },
        ] as const
    ).map((item) => ({ item, row: 0, change: "start" as const })),
);

/** The connectors the stacked agent has on, by mark and colour. */
export const stackedConnectors: readonly Connector[] = [
    ["zapier", "#ff4f00"],
    ["airtable", "#18bfff"],
    ["trello", "#0052cc"],
    ["slack", "#4a154b"],
    ["googlecalendar", "#4285f4"],
];

/** Return what the chat in your space shows at a beat, the live view current once undone. */
export function destackedChatAt(beat: number | undefined): Moment {
    // swap the live view's rows in place once the runs are undone
    const moment = destackedScene.at(beat);
    const isUndone = beat === undefined || beat >= cueOf(undoRuns).start + 4;

    return isUndone
        ? {
              ...moment,
              items: moment.items.map((item) => (item === duplicates ? duplicatesUndone : item)),
          }
        : moment;
}

/** The stretch of the trace the waterfall shows: from and to, in milliseconds. */
type Range = readonly [from: number, to: number];

/** The whole trace. */
const FULL: Range = [0, AXIS];

/** The transaction and Ines's run, wide enough to read each write. */
const FOCUS: Range = [0, 240];

/** The view's state at one moment of the conversation. */
export type RunMoment = {
    lane: Lane;
    time: number;
    selected: number;
    range: Range;
    isHeld: boolean;
};

/** Draw your space's run as one of its apps: the map, the timeline, and the selected span. */
export function RunScreen(properties: {
    moment: RunMoment;
    onScrub: (share: number) => void;
    onTouch: () => void;
}) {
    // keep what the reader chose over what the autoplay shows
    const [chosen, setChosen] = createSignal<number | undefined>(undefined);
    const [zoom, setZoom] = createSignal<Range | undefined>(undefined);
    const [opened, setOpened] = createSignal<ReadonlySet<number> | undefined>(undefined);

    // read the lane's trace, and a selection the lane has
    const trace = () => traces[properties.moment.lane];
    const selected = () => {
        const id = chosen() ?? properties.moment.selected;

        return trace().spans.some((span) => span.id === id) ? id : ROOT_SPAN;
    };
    const span = () => trace().span(selected());
    const range = () => zoom() ?? properties.moment.range;
    const open = (): ReadonlySet<number> => {
        const run = runOf(span());

        return opened() ?? new Set(run === undefined ? [] : [run]);
    };

    // hold the story still whenever the reader takes over
    const select = (id: number) => {
        properties.onTouch();
        setChosen(id);
    };
    const toggle = (id: number) => {
        properties.onTouch();
        setOpened(() => {
            const next = new Set(open());
            if (!next.delete(id)) {
                next.add(id);
            }

            return next;
        });
    };
    const show = (next: Range) => {
        properties.onTouch();
        setZoom(next);
    };

    return (
        <div {...style.attrs(appStyles.app, styles.app)}>
            <AppRail open="telemetry" apps={rail} morph={undefined} />
            <article {...style.attrs(styles.page)}>
                <AppHeader title="tx-91c">
                    <span {...style.attrs(appStyles.action, styles.wide)}>Thu 09:00 · Florian</span>
                    <span {...style.attrs(appStyles.primary)}>
                        {primaryOf(properties.moment.lane)}
                    </span>
                </AppHeader>
                <div {...style.attrs(styles.stats)}>
                    <Stat
                        name="Runs"
                        value={pick(properties.moment.lane, "9", "0", "9")}
                        isDuplicate={properties.moment.lane === "original"}
                    />
                    <Stat
                        name="Duplicate writes"
                        value={pick(properties.moment.lane, "27", "0", "0")}
                        isDuplicate={properties.moment.lane === "original"}
                    />
                    <Stat name="Rows updated" value="400" isDuplicate={false} />
                    <Stat
                        name="Trigger"
                        value={properties.moment.lane === "replay" ? "insert" : "update"}
                        isDuplicate={properties.moment.lane === "original"}
                    />
                </div>
                <SpaceMap
                    lane={properties.moment.lane}
                    service={span().service}
                    onSelect={(part) => select(trace().first(part).id)}
                />
                <Timeline
                    trace={trace()}
                    time={properties.moment.time}
                    range={range()}
                    selected={selected()}
                    open={open()}
                    service={span().service}
                    isZoomed={range() !== FULL}
                    onScrub={properties.onScrub}
                    onRange={show}
                    onSelect={select}
                    onToggle={toggle}
                />
            </article>
            <SpanBar span={span()} line={trace().lineOf(span())} />
        </div>
    );
}

/** The space's apps on the rail, Observability open. */
const rail: readonly (readonly [icon: string, label: string])[] = [
    ["telemetry", "Observability"],
    ["pages", "Pages"],
    ["tasks", "Tasks"],
    ["chat", "Chat"],
    ["table", "Tables"],
    ["calendar", "Calendar"],
];

/** Return the run's state at a beat of the conversation, held where the reader holds it. */
export function runMomentAt(beat: number, held: number | undefined): RunMoment {
    // hold the autoplay's lane and selection at the reader's moment
    const moment = autoplayAt(beat);

    return held === undefined ? moment : { ...moment, time: held, isHeld: true };
}

/** Return what the autoplay shows at a beat: the runs, the zoom to Ines, the replay, the undo. */
function autoplayAt(beat: number): RunMoment {
    // sweep the nine runs while your agent reads them
    const read = cueOf(readRuns);
    const replayed = cueOf(replayApproval);
    const undone = cueOf(undoRuns);
    const at = { selected: ROOT_SPAN, range: FULL, isHeld: false };
    if (beat < read.start) {
        return { ...at, lane: "original", time: 0 };
    } else if (beat < read.end) {
        return { ...at, lane: "original", time: sweep(0, AXIS, beat, read) };
    }
    // zoom to Ines's run while your agent explains and fixes it
    else if (beat < replayed.start) {
        return { ...at, lane: "original", time: AXIS, selected: INES_HOLD, range: FOCUS };
    }
    // sweep the replay, then show the runs undone
    else if (beat < replayed.end) {
        return { ...at, lane: "replay", time: sweep(0, AXIS, beat, replayed) };
    } else if (beat < undone.start) {
        return { ...at, lane: "replay", time: AXIS };
    }

    return { ...at, lane: "undone", time: AXIS, selected: INES_HOLD };
}

/** Return a moment of the trace as a beat crosses a stretch of the clock. */
function sweep(
    from: number,
    to: number,
    beat: number,
    cue: { start: number; end: number },
): number {
    return from + ((to - from) * (beat - cue.start)) / (cue.end - cue.start);
}

/** Return where an item of the conversation sits on the clock, stretched over the step after it. */
function cueOf(item: Item): { start: number; end: number } {
    // find the item's step and the one after it
    const index = destackedScene.cues.findIndex((cue) => cue.step.item === item);
    const cue = present(destackedScene.cues[index], "scene cue");
    const next = destackedScene.cues[index + 1] ?? cue;

    return { start: cue.start, end: next.start + next.beats };
}

/** Draw one stat of the transaction: its value over its name. */
function Stat(properties: { name: string; value: string; isDuplicate: boolean }) {
    return (
        <span {...style.attrs(styles.stat)}>
            <b {...style.attrs(styles.statValue, properties.isDuplicate && styles.duplicate)}>
                {properties.value}
            </b>
            <span {...style.attrs(appText.small, styles.statName)}>{properties.name}</span>
        </span>
    );
}

/** A card of the map: its app, glyph, name, status in each lane, and place on the map. */
type Card = readonly [
    part: Service,
    glyph: string,
    name: string,
    original: string,
    replay: string,
    undone: string,
    x: number,
    y: number,
];

/** The map's width in its own units. */
const MAP_WIDTH = 600;

/** The map's height in its own units. */
const MAP_HEIGHT = 140;

/** The width of a card. */
const CARD_WIDTH = 158;

/** The height of a card. */
const CARD_HEIGHT = 42;

/** The cards of the map: the table, the workflow it triggers, and the three apps it writes to. */
const cards: readonly Card[] = [
    ["tables", "table", "Signups", "400 approved", "400 approved", "400 approved", 0, 49],
    ["workflows", "sync", "launch-askers", "9 runs", "0 runs", "9 runs undone", 212, 49],
    ["tasks", "tasks", "Launch board", "9 duplicate cards", "no change", "9 cards removed", 440, 0],
    ["chat", "chat", "#launch", "9 duplicate posts", "no change", "9 posts removed", 440, 49],
    ["calendar", "calendar", "Demos", "Ines booked twice", "no change", "9 holds removed", 440, 98],
];

/** A link of the map: from one app to another. */
type Wire = readonly [from: Service, to: Service];

/** The links of the map, the trigger first. */
const wires: readonly Wire[] = [
    ["tables", "workflows"],
    ["workflows", "tasks"],
    ["workflows", "chat"],
    ["workflows", "calendar"],
];

/** Draw the map of the apps the approval touched: cards on a fixed grid, links drawn beneath. */
function SpaceMap(properties: { lane: Lane; service: Service; onSelect: (part: Service) => void }) {
    return (
        <section data-component="SpaceMap" {...style.attrs(styles.map)}>
            <svg
                viewBox={`0 0 ${MAP_WIDTH} ${MAP_HEIGHT}`}
                preserveAspectRatio="none"
                {...style.attrs(styles.wires)}
            >
                <For each={wires}>
                    {(wire) => (
                        <WireLine wire={wire} isDuplicate={properties.lane === "original"} />
                    )}
                </For>
            </svg>
            <For each={wires.slice(0, 1)}>
                {(wire) => (
                    <WireLabel
                        wire={wire}
                        text={properties.lane === "replay" ? "insert" : "update"}
                    />
                )}
            </For>
            <For each={cards}>
                {(card) => (
                    <CardBox
                        card={card}
                        status={pick(properties.lane, card[3], card[4], card[5])}
                        badge={badgeOfCard(card[0], properties.lane)}
                        isDuplicate={properties.lane === "original" && card[0] !== "tables"}
                        isSelected={card[0] === properties.service}
                        onSelect={() => properties.onSelect(card[0])}
                    />
                )}
            </For>
        </section>
    );
}

/** Draw a link from one card's right edge to the next card's left edge. */
function WireLine(properties: { wire: Wire; isDuplicate: boolean }) {
    // curve from the source's right edge to the target's left edge
    const from = cardOf(properties.wire[0]);
    const to = cardOf(properties.wire[1]);
    const startX = from[6] + CARD_WIDTH;
    const startY = from[7] + CARD_HEIGHT / 2;
    const endX = to[6];
    const endY = to[7] + CARD_HEIGHT / 2;
    const middle = (startX + endX) / 2;

    return (
        <path
            d={`M${startX},${startY} C${middle},${startY} ${middle},${endY} ${endX},${endY}`}
            {...style.attrs(styles.wire, properties.isDuplicate && styles.wireDuplicate)}
        />
    );
}

/** Draw a link's label where it meets its target. */
function WireLabel(properties: { wire: Wire; text: string }) {
    // sit the label halfway along the link, level with its target
    const from = cardOf(properties.wire[0]);
    const to = cardOf(properties.wire[1]);
    const x = (from[6] + CARD_WIDTH + to[6]) / 2;
    const y = to[7] + CARD_HEIGHT / 2 - 9;

    return (
        <span
            style={{ left: `${(x / MAP_WIDTH) * 100}%`, top: `${(y / MAP_HEIGHT) * 100}%` }}
            {...style.attrs(styles.wireLabel)}
        >
            {properties.text}
        </span>
    );
}

/** Draw a card of the map: its app's glyph and name, and its status. */
function CardBox(properties: {
    card: Card;
    status: string;
    badge: string;
    isDuplicate: boolean;
    isSelected: boolean;
    onSelect: () => void;
}) {
    return (
        <button
            type="button"
            onClick={() => properties.onSelect()}
            {...style.attrs(
                styles.card,
                styles.cardAt(
                    `${(properties.card[6] / MAP_WIDTH) * 100}%`,
                    `${(properties.card[7] / MAP_HEIGHT) * 100}%`,
                    `${(CARD_WIDTH / MAP_WIDTH) * 100}%`,
                    `${(CARD_HEIGHT / MAP_HEIGHT) * 100}%`,
                ),
                properties.isSelected && styles.cardSelected,
            )}
        >
            <span {...style.attrs(styles.cardName)}>
                <AppGlyph name={properties.card[1]} />
                <span {...style.attrs(styles.cardTitle)}>{properties.card[2]}</span>
                {properties.badge === "" ? undefined : (
                    <span
                        {...style.attrs(
                            styles.badge,
                            properties.isDuplicate && styles.badgeDuplicate,
                        )}
                    >
                        {properties.badge}
                    </span>
                )}
            </span>
            <span
                {...style.attrs(
                    appText.small,
                    styles.cardStatus,
                    properties.isDuplicate && styles.duplicate,
                )}
            >
                {properties.status}
            </span>
        </button>
    );
}

/** Draw the timeline: the runs across the whole trace, the waterfall of the range, the event. */
function Timeline(properties: {
    trace: Trace;
    time: number;
    range: Range;
    selected: number;
    open: ReadonlySet<number>;
    service: Service;
    isZoomed: boolean;
    onScrub: (share: number) => void;
    onRange: (range: Range) => void;
    onSelect: (id: number) => void;
    onToggle: (id: number) => void;
}) {
    // keep the selected span in view as the selection and the open runs change
    let list: HTMLDivElement | undefined;
    createEffect(
        () => [properties.selected, properties.open.size] as const,
        ([id]) => {
            const row = list?.querySelector(`[data-span="${id}"]`);
            if (list !== undefined && row instanceof HTMLElement) {
                list.scrollTop = Math.max(0, row.offsetTop - list.clientHeight / 2);
            }
        },
    );

    // press to move the playhead, drag to zoom the waterfall, double press to fit the trace
    let origin: number | undefined;
    const isDrag = (share: number) => origin !== undefined && Math.abs(share - origin) > 0.01;

    return (
        <section {...style.attrs(styles.timeline)}>
            <span {...style.attrs(styles.timelineHead)}>
                <b {...style.attrs(appText.heading)}>Timeline</b>
                <span {...style.attrs(appText.meta, styles.quiet)}>
                    {properties.trace.runs().length} runs · {properties.trace.spans.length} spans
                </span>
                <button
                    type="button"
                    onClick={() => properties.onRange(FULL)}
                    {...style.attrs(
                        appStyles.property,
                        styles.fit,
                        !properties.isZoomed && styles.fitIdle,
                    )}
                >
                    Fit
                </button>
            </span>
            <div
                onPointerDown={(event) => {
                    event.currentTarget.setPointerCapture(event.pointerId);
                    origin = shareOfPointer(event);
                }}
                onPointerMove={(event) => {
                    const share = shareOfPointer(event);
                    if (event.buttons > 0 && origin !== undefined && isDrag(share)) {
                        properties.onRange([
                            Math.min(origin, share) * AXIS,
                            Math.max(origin, share) * AXIS,
                        ]);
                    }
                }}
                onPointerUp={(event) => {
                    const share = shareOfPointer(event);
                    if (origin !== undefined && !isDrag(share)) {
                        properties.onScrub(share);
                    }
                    origin = undefined;
                }}
                onDblClick={() => properties.onRange(FULL)}
                data-component="Scrubber"
                {...style.attrs(styles.overview)}
            >
                <For
                    each={properties.trace.spans.filter(
                        (span) => span.kind === "transaction" || span.kind === "run",
                    )}
                >
                    {(span) => <Block span={span} />}
                </For>
                <span
                    style={{ left: 0, width: `${(properties.range[0] / AXIS) * 100}%` }}
                    {...style.attrs(styles.shade)}
                />
                <span
                    style={{ left: `${(properties.range[1] / AXIS) * 100}%`, right: 0 }}
                    {...style.attrs(styles.shade)}
                />
                <span
                    style={{ left: `${(properties.time / AXIS) * 100}%` }}
                    {...style.attrs(styles.playhead)}
                />
            </div>
            <div {...style.attrs(styles.waterfall)}>
                <span {...style.attrs(styles.waterHead)}>
                    <span>span</span>
                    <span {...style.attrs(styles.ruler)}>
                        <For each={ticksOf(properties.range, 4)}>
                            {(tick) => (
                                <span
                                    style={{
                                        left: `${shareIn(tick, properties.range[0], properties.range[1]) * 100}%`,
                                    }}
                                    {...style.attrs(styles.mark)}
                                >
                                    {tick} ms
                                </span>
                            )}
                        </For>
                    </span>
                    <span {...style.attrs(styles.end)}>time</span>
                </span>
                <div ref={list} {...style.attrs(styles.rows)}>
                    <For each={properties.trace.rows(properties.open)}>
                        {(span) => (
                            <WaterRow
                                span={span}
                                depth={depthOf(span)}
                                isRun={span.kind === "run"}
                                isOpen={properties.open.has(span.id)}
                                isSelected={span.id === properties.selected}
                                isAhead={span.start > properties.time}
                                from={properties.range[0]}
                                to={properties.range[1]}
                                onSelect={() => properties.onSelect(span.id)}
                                onToggle={() => properties.onToggle(span.id)}
                            />
                        )}
                    </For>
                    {properties.trace.lane === "replay" ? <Untriggered /> : undefined}
                </div>
            </div>
            <Caption event={eventAt(properties.trace, properties.time)} time={properties.time} />
        </section>
    );
}

/** Draw the transaction or a run as a block on the timeline's overview. */
function Block(properties: { span: Span }) {
    return (
        <span
            style={{
                left: `${(properties.span.start / AXIS) * 100}%`,
                width: `${((properties.span.end - properties.span.start) / AXIS) * 100}%`,
            }}
            {...style.attrs(
                styles.block,
                properties.span.kind === "transaction" && styles.blockRoot,
                properties.span.status === "duplicate" && styles.blockDuplicate,
                properties.span.status === "undone" && styles.blockUndone,
            )}
        />
    );
}

/** Draw the row the replay shows where the runs were: the workflow, not triggered. */
function Untriggered() {
    return (
        <span {...style.attrs(styles.row, styles.rowMuted)}>
            <span {...style.attrs(styles.rowName)}>
                <span {...style.attrs(styles.fold)} />
                <AppGlyph name="sync" />
                <span {...style.attrs(styles.rowLabel)}>launch-askers</span>
            </span>
            <span {...style.attrs(styles.untriggered)}>
                not triggered · on insert · 9 runs avoided
            </span>
            <span {...style.attrs(styles.end)}>—</span>
        </span>
    );
}

/** Draw one row of the waterfall: its fold, glyph and name, its bar, and its duration. */
function WaterRow(properties: {
    span: Span;
    depth: number;
    isRun: boolean;
    isOpen: boolean;
    isSelected: boolean;
    isAhead: boolean;
    from: number;
    to: number;
    onSelect: () => void;
    onToggle: () => void;
}) {
    return (
        <span
            data-span={properties.span.id}
            onClick={() => properties.onSelect()}
            {...style.attrs(
                styles.row,
                properties.isSelected && styles.rowSelected,
                properties.isAhead && styles.rowAhead,
            )}
        >
            <span
                style={{ "padding-inline-start": `${properties.depth * 14}px` }}
                {...style.attrs(styles.rowName)}
            >
                <button
                    type="button"
                    onClick={(event) => {
                        event.stopPropagation();
                        properties.onToggle();
                    }}
                    {...style.attrs(styles.fold, !properties.isRun && styles.foldNone)}
                >
                    {properties.isOpen ? "▾" : "▸"}
                </button>
                {properties.span.kind === "run" && properties.span.asker !== undefined ? (
                    <span
                        style={{
                            "background-color": faceTints[properties.span.asker % faceTints.length],
                        }}
                        {...style.attrs(appStyles.avatar)}
                    >
                        {present(askers[properties.span.asker], "asker")[0].slice(0, 1)}
                    </span>
                ) : (
                    <AppGlyph name={glyphOf[properties.span.service]} />
                )}
                <span
                    {...style.attrs(
                        styles.rowLabel,
                        properties.span.status === "duplicate" &&
                            properties.span.kind === "write" &&
                            styles.duplicate,
                        properties.span.status === "undone" && styles.struck,
                    )}
                >
                    {properties.span.name}
                </span>
            </span>
            <span {...style.attrs(styles.track)}>
                <span
                    style={{
                        left: `${Math.max(0, shareIn(properties.span.start, properties.from, properties.to)) * 100}%`,
                        width: `max(3px, ${(Math.min(1, shareIn(properties.span.end, properties.from, properties.to)) - Math.max(0, shareIn(properties.span.start, properties.from, properties.to))) * 100}%)`,
                    }}
                    {...style.attrs(
                        styles.bar,
                        properties.span.kind === "transaction" && styles.barRoot,
                        properties.span.status === "duplicate" &&
                            properties.span.kind === "write" &&
                            styles.barDuplicate,
                        properties.span.status === "undone" && styles.barUndone,
                        properties.isSelected && styles.barSelected,
                        shareIn(properties.span.end, properties.from, properties.to) < 0 &&
                            styles.barOut,
                        shareIn(properties.span.start, properties.from, properties.to) > 1 &&
                            styles.barOut,
                    )}
                />
            </span>
            <span {...style.attrs(styles.end, styles.duration)}>{durationOf(properties.span)}</span>
        </span>
    );
}

/** The colours of the askers' faces, in turn. */
const faceTints: readonly string[] = [
    palette.teal,
    palette.violet,
    palette.indigo,
    palette.green,
    palette.amber,
];

/** The glyph of each app of your space. */
const glyphOf: Readonly<Record<Service, string>> = {
    tables: "table",
    workflows: "sync",
    tasks: "tasks",
    chat: "chat",
    calendar: "calendar",
};

/** Draw the event at the playhead under the waterfall. */
function Caption(properties: { event: Event; time: number }) {
    return (
        <span {...style.attrs(appText.meta, styles.caption)}>
            <span {...style.attrs(styles.stamp)}>{stampOf(properties.time)}</span>
            <span {...style.attrs(styles.captionText, properties.event[1] && styles.duplicate)}>
                {properties.event[2]}
            </span>
        </span>
    );
}

/** Draw the selected span in the status bar: what ran, how it ended, and the line it ran. */
function SpanBar(properties: {
    span: Span;
    line: readonly [file: string, text: string, value: string];
}) {
    return (
        <footer {...style.attrs(appStyles.statusBar, styles.status)}>
            <span {...style.attrs(appStyles.status)}>
                <span
                    {...style.attrs(
                        styles.dot,
                        properties.span.status === "duplicate" && styles.dotDuplicate,
                        properties.span.status === "undone" && styles.dotUndone,
                    )}
                />
                <b {...style.attrs(styles.spanName)}>{properties.span.name}</b>
            </span>
            <span {...style.attrs(appStyles.status)}>{durationOf(properties.span)}</span>
            <code {...style.attrs(styles.code)}>
                {properties.line[0]} ·{" "}
                <span {...style.attrs(styles.codeText)}>{properties.line[1]}</span>
            </code>
            <span
                {...style.attrs(
                    styles.value,
                    properties.span.status === "duplicate" && styles.duplicate,
                )}
            >
                {properties.line[2]}
            </span>
        </footer>
    );
}

/** The traces of each lane. */
const traces: Readonly<Record<Lane, Trace>> = {
    original: new Trace("original"),
    replay: new Trace("replay"),
    undone: new Trace("undone"),
};

/** Return a value by lane. */
function pick(lane: Lane, original: string, replay: string, undone: string): string {
    return lane === "original" ? original : lane === "replay" ? replay : undone;
}

/** Return the page's primary action in a lane. */
function primaryOf(lane: Lane): string {
    return lane === "original" ? "Undo 9 runs" : lane === "replay" ? "Ship fix" : "Undone";
}

/** Return a card's count badge in a lane: the writes it got, or took back. */
function badgeOfCard(part: Service, lane: Lane): string {
    // count the writes for the three apps the workflow writes to
    if (part === "tables" || lane === "replay") {
        return "";
    } else if (part === "workflows") {
        return lane === "original" ? "9" : "";
    }

    return lane === "original" ? "+9" : "−9";
}

/** Return a card of the map by its app. */
function cardOf(part: Service): Card {
    return present(
        cards.find(([candidate]) => candidate === part),
        `map card ${part}`,
    );
}

/** Return the latest event of a trace at a moment. */
function eventAt(trace: Trace, time: number): Event {
    // take the last event at or before the moment
    const events = trace.events();

    return present(events.findLast(([at]) => at <= time) ?? events[0], "trace event");
}

/** Return where a pointer falls across the element it pressed, from 0 to 1. */
function shareOfPointer(event: PointerEvent): number {
    const target = event.currentTarget;
    const box = target instanceof HTMLElement ? target.getBoundingClientRect() : undefined;

    return box === undefined ? 0 : Math.min(1, Math.max(0, (event.clientX - box.left) / box.width));
}

/** Return where a moment falls across a range, from 0 to 1. */
function shareIn(time: number, from: number, to: number): number {
    return (time - from) / (to - from);
}

/** Return the marks of a ruler across a range, at most a number of them. */
function ticksOf(range: Range, marks: number): readonly number[] {
    // space the marks at the first step that fits
    const width = range[1] - range[0];
    const step =
        [20, 25, 50, 100, 200, 250, 500].find((candidate) => width / candidate <= marks) ?? 500;
    const first = Math.ceil(range[0] / step) * step;

    return Array.from(
        { length: Math.floor((range[1] - first) / step) + 1 },
        (_, index) => first + index * step,
    );
}

/** The view styles. */
const styles = style.create({
    app: {
        backgroundColor: color.background,
        color: color.foreground,
        overflow: "hidden",
    },
    page: {
        display: "grid",
        gap: "0.875rem",
        gridTemplateColumns: "minmax(0, 1fr)",
        gridTemplateRows: "auto auto auto minmax(0, 1fr)",
        minHeight: 0,
        minWidth: 0,
        overflow: "hidden",
        paddingBlock: { default: "1.125rem", [NARROW]: "0.875rem" },
        paddingInline: { default: "1.5rem", [NARROW]: "1rem" },
    },
    wide: {
        display: { default: "inline-flex", [media.maxMd]: "none" },
    },
    quiet: {
        color: color.mutedForeground,
    },
    duplicate: {
        color: palette.ember,
    },
    dot: {
        backgroundColor: palette.teal,
        borderRadius: "50%",
        flexShrink: 0,
        height: "0.4375rem",
        width: "0.4375rem",
    },
    dotDuplicate: {
        backgroundColor: palette.ember,
    },
    dotUndone: {
        backgroundColor: color.mutedForeground,
    },

    // map
    map: {
        aspectRatio: { default: `${MAP_WIDTH} / ${MAP_HEIGHT}`, [NARROW]: "auto" },
        columnGap: "0.5rem",
        display: { default: "block", [NARROW]: "grid" },
        gridTemplateColumns: "repeat(2, minmax(0, 1fr))",
        position: "relative",
        rowGap: "0.5rem",
        width: "100%",
    },
    wires: {
        height: "100%",
        inset: 0,
        position: "absolute",
        width: "100%",
        display: { default: "block", [NARROW]: "none" },
    },
    wire: {
        fill: "none",
        stroke: color.border,
        strokeWidth: 1.25,
        vectorEffect: "non-scaling-stroke",
    },
    wireDuplicate: {
        stroke: `color-mix(in srgb, ${palette.ember} 55%, ${color.border})`,
    },
    wireLabel: {
        backgroundColor: color.background,
        color: color.mutedForeground,
        fontFamily: font.code,
        fontSize: typeScale.label,
        paddingInline: "0.25rem",
        position: "absolute",
        transform: "translate(-50%, -50%)",
        whiteSpace: "nowrap",
        display: { default: "inline", [NARROW]: "none" },
    },
    card: {
        alignContent: "center",
        paddingInlineEnd: "2.125rem",
        paddingInlineStart: "0.5rem",
        backgroundColor: color.card,
        borderColor: color.border,
        borderRadius: "8px",
        borderStyle: "solid",
        borderWidth: "1px",
        color: "inherit",
        cursor: "pointer",
        display: "grid",
        fontFamily: "inherit",
        gap: "0.125rem",
        position: { default: "absolute", [NARROW]: "relative" },
        textAlign: "start",
        transitionDuration: "200ms",
        transitionProperty: "border-color, box-shadow",
    },
    cardAt: (left: string, top: string, width: string, height: string) => ({
        height: { default: height, [NARROW]: "2rem" },
        left: { default: left, [NARROW]: "auto" },
        top: { default: top, [NARROW]: "auto" },
        width: { default: width, [NARROW]: "auto" },
    }),
    cardSelected: {
        borderColor: color.foreground,
        boxShadow: `0 0 0 3px color-mix(in srgb, ${color.foreground} 12%, transparent)`,
    },
    cardName: {
        alignItems: "center",
        display: "flex",
        fontSize: typeScale.small,
        fontWeight: 600,
        gap: "0.375rem",
        overflow: "hidden",
        whiteSpace: "nowrap",
    },
    cardStatus: {
        color: color.mutedForeground,
        overflow: "hidden",
        textOverflow: "ellipsis",
        whiteSpace: "nowrap",
        display: { default: "block", [NARROW]: "none" },
    },

    // timeline
    timeline: {
        display: "grid",
        gap: "0.5rem",
        gridTemplateRows: "auto auto minmax(0, 1fr) auto",
        minHeight: 0,
    },
    timelineHead: {
        alignItems: "baseline",
        display: "flex",
        gap: "0.625rem",
    },
    fit: {
        borderWidth: 0,
        cursor: "pointer",
        fontFamily: "inherit",
        marginInlineStart: "auto",
    },
    fitIdle: {
        opacity: 0.5,
    },
    overview: {
        backgroundColor: color.muted,
        borderRadius: "5px",
        cursor: "ew-resize",
        height: "1.125rem",
        position: "relative",
        touchAction: "none",
        userSelect: "none",
    },
    block: {
        backgroundColor: `color-mix(in srgb, ${color.mutedForeground} 35%, transparent)`,
        borderRadius: "3px",
        bottom: "4px",
        position: "absolute",
        top: "4px",
    },
    blockRoot: {
        backgroundColor: color.mutedForeground,
    },
    blockDuplicate: {
        backgroundColor: `color-mix(in srgb, ${palette.ember} 55%, transparent)`,
    },
    blockUndone: {
        opacity: 0.4,
    },
    shade: {
        backgroundColor: `color-mix(in srgb, ${color.background} 55%, transparent)`,
        bottom: 0,
        pointerEvents: "none",
        position: "absolute",
        top: 0,
    },
    playhead: {
        backgroundColor: color.foreground,
        bottom: "-3px",
        position: "absolute",
        top: "-3px",
        transitionDuration: { default: "250ms", [media.motionReduce]: "0ms" },
        transitionProperty: "left",
        transitionTimingFunction: "linear",
        width: "2px",
    },
    ruler: {
        height: "0.875rem",
        position: "relative",
    },
    mark: {
        color: color.mutedForeground,
        fontFamily: font.code,
        position: "absolute",
        whiteSpace: "nowrap",
    },
    bar: {
        backgroundColor: `color-mix(in srgb, ${color.mutedForeground} 30%, transparent)`,
        borderRadius: "3px",
        bottom: "0.4375rem",
        position: "absolute",
        top: "0.4375rem",
    },
    barDuplicate: {
        backgroundColor: `color-mix(in srgb, ${palette.ember} 60%, transparent)`,
    },
    barUndone: {
        opacity: 0.35,
    },
    barSelected: {
        boxShadow: `0 0 0 1.5px ${color.foreground}`,
    },
    caption: {
        alignItems: "baseline",
        display: "flex",
        gap: "0.5rem",
        minWidth: 0,
        whiteSpace: "nowrap",
    },
    stamp: {
        backgroundColor: color.muted,
        borderRadius: "4px",
        color: color.mutedForeground,
        flexShrink: 0,
        fontFamily: font.code,
        paddingInline: "0.375rem",
    },
    captionText: {
        overflow: "hidden",
        textOverflow: "ellipsis",
    },

    // status bar
    status: {
        minWidth: 0,
        overflow: "hidden",
    },
    spanName: {
        color: color.foreground,
        fontFamily: font.code,
    },
    code: {
        flexShrink: 1,
        fontFamily: font.code,
        minWidth: 0,
        overflow: "hidden",
        textOverflow: "ellipsis",
    },
    codeText: {
        color: color.foreground,
    },
    value: {
        flexShrink: 0,
        fontStyle: "italic",
        marginInlineStart: "auto",
    },
    waterfall: {
        borderColor: color.border,
        borderRadius: "8px",
        borderStyle: "solid",
        borderWidth: "1px",
        display: "grid",
        gridTemplateRows: "auto minmax(0, 1fr)",
        minHeight: 0,
        overflow: "hidden",
    },
    waterHead: {
        backgroundColor: color.muted,
        borderBottomColor: color.border,
        borderBottomStyle: "solid",
        borderBottomWidth: "1px",
        color: color.mutedForeground,
        columnGap: "0.75rem",
        display: "grid",
        fontSize: typeScale.label,
        gridTemplateColumns: {
            default: "minmax(0, 0.95fr) minmax(0, 1fr) 3rem",
            [NARROW]: "minmax(0, 1.6fr) minmax(0, 0.8fr) 2.5rem",
        },
        paddingBlock: "0.25rem",
        paddingInline: "0.625rem",
    },
    rows: {
        alignContent: "start",
        position: "relative",
        display: "grid",
        minHeight: 0,
        overflowY: "auto",
    },
    row: {
        alignItems: "center",
        borderBottomColor: color.border,
        borderBottomStyle: "solid",
        borderBottomWidth: "1px",
        columnGap: "0.75rem",
        cursor: "pointer",
        display: "grid",
        fontSize: typeScale.small,
        gridTemplateColumns: {
            default: "minmax(0, 0.95fr) minmax(0, 1fr) 3rem",
            [NARROW]: "minmax(0, 1.6fr) minmax(0, 0.8fr) 2.5rem",
        },
        height: "1.625rem",
        paddingInline: "0.625rem",
        transitionDuration: "150ms",
        transitionProperty: "background-color, opacity",
    },
    rowSelected: {
        backgroundColor: color.muted,
        boxShadow: `inset 2px 0 0 ${color.foreground}`,
    },
    rowAhead: {
        opacity: 0.4,
    },
    rowMuted: {
        color: color.mutedForeground,
        cursor: "default",
    },
    rowName: {
        alignItems: "center",
        display: "flex",
        gap: "0.375rem",
        minWidth: 0,
    },
    fold: {
        backgroundColor: "transparent",
        borderWidth: 0,
        color: color.mutedForeground,
        cursor: "pointer",
        flexShrink: 0,
        fontSize: typeScale.label,
        paddingInline: 0,
        width: "0.625rem",
    },
    foldNone: {
        visibility: "hidden",
    },
    rowLabel: {
        fontFamily: font.code,
        overflow: "hidden",
        textOverflow: "ellipsis",
        whiteSpace: "nowrap",
    },
    struck: {
        color: color.mutedForeground,
        textDecorationLine: "line-through",
    },
    track: {
        alignSelf: "stretch",
        position: "relative",
    },
    barRoot: {
        backgroundColor: color.mutedForeground,
    },
    barOut: {
        visibility: "hidden",
    },
    end: {
        textAlign: "end",
    },
    duration: {
        color: color.mutedForeground,
        fontFamily: font.code,
        fontSize: typeScale.label,
    },
    untriggered: {
        color: color.mutedForeground,
        fontSize: typeScale.label,
        fontStyle: "italic",
        overflow: "hidden",
        textOverflow: "ellipsis",
        whiteSpace: "nowrap",
    },
    stats: {
        display: "grid",
        gap: "0.5rem",
        gridTemplateColumns: {
            default: "repeat(4, minmax(0, 1fr))",
            [NARROW]: "repeat(2, minmax(0, 1fr))",
        },
    },
    stat: {
        backgroundColor: color.card,
        borderColor: color.border,
        borderRadius: "8px",
        borderStyle: "solid",
        borderWidth: "1px",
        display: "grid",
        minWidth: 0,
        paddingBlock: "0.375rem",
        paddingInline: "0.75rem",
    },
    statName: {
        color: color.mutedForeground,
        overflow: "hidden",
        textOverflow: "ellipsis",
        whiteSpace: "nowrap",
    },
    statValue: {
        fontSize: typeScale.body,
        fontWeight: 600,
        lineHeight: 1.25,
    },
    cardTitle: {
        minWidth: 0,
        overflow: "hidden",
        textOverflow: "ellipsis",
    },
    badge: {
        backgroundColor: color.muted,
        position: "absolute",
        right: "0.5rem",
        top: "50%",
        transform: "translateY(-50%)",
        borderRadius: "999px",
        color: color.mutedForeground,
        flexShrink: 0,
        fontFamily: font.code,
        fontSize: typeScale.label,
        fontWeight: 600,
        paddingInline: "0.375rem",
    },
    badgeDuplicate: {
        backgroundColor: palette.ember,
        color: palette.tileGlyph,
    },
});
