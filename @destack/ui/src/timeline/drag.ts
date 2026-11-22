import { type Message, t } from "@destack/locale";
import { ScheduleLink } from "@destack/schema";
import { type Accessor, createMemo, type useLocale } from "@destack/view";
import {
    Draggable,
    type DragKind,
    type DragManager,
    type DragOperation,
    DragTarget,
    Droppable,
} from "../drag/index.ts";
import type { Selection } from "../selection/index.ts";
import type { TimelineOptions, TimelineRowPoint } from "./control.ts";
import { ROW_INDENT } from "./places.ts";
import { TimelineItem, type TimelineTree } from "./tree.ts";
import type { TimelineWindow } from "./window.ts";

/** The kind of item a timeline's draggable bars are. */
export const BAR_TYPE = "bar";

/** The type of a row's drag. */
export const ROW_TYPE = "row";

/** The droppable of the track, along which bars move in time. */
export const TRACK = "timeline-track";

/** The droppable of the bars' edges, which a link is drawn onto. */
const EDGES = "timeline-edges";

/** The droppable of the gaps between rows, which a row moves into. */
export const ROWS = "timeline-rows";

/** A part of a bar a drag takes: its body, an edge's handle, or an edge's connector. */
export type TimelineBarPart = "body" | "start" | "end" | "link-start" | "link-end";

/** The attributes of a bar's draggable parts. */
export interface TimelineBarDrags {
    /** The body, which moves the item. */
    readonly body: ReturnType<Draggable["attributes"]>;
    /** The start's handle, which moves the start alone. */
    readonly start: ReturnType<Draggable["attributes"]>;
    /** The end's handle, which moves the end alone. */
    readonly end: ReturnType<Draggable["attributes"]>;
    /** The start's connector, which draws a link from the start. */
    readonly linkStart: ReturnType<Draggable["attributes"]>;
    /** The end's connector, which draws a link from the end. */
    readonly linkEnd: ReturnType<Draggable["attributes"]>;
}

/** The drags of a timeline's bars, links and rows, and the changes they leave. */
export class TimelineDrag {
    /** The drags of the bars, links and rows. */
    readonly manager: DragManager;
    /** The items a drag in progress would change, by key, with every item they move along. */
    readonly #previews: Accessor<ReadonlyMap<string, TimelineItem>>;
    /** The timeline's options. */
    readonly #options: TimelineOptions;
    /** The time shown, which drags move along. */
    readonly #window: TimelineWindow;
    /** The items and rows. */
    readonly #tree: TimelineTree;
    /** The selected items, which a move takes along. */
    readonly #selection: Selection;

    /** Drag a timeline's bars along its track and its rows between rows, in a locale. */
    constructor(
        context: {
            readonly options: TimelineOptions;
            readonly window: TimelineWindow;
            readonly tree: TimelineTree;
            readonly selection: Selection;
            readonly scroller: Accessor<HTMLElement | undefined>;
            readonly rowAt: (offset: number) => TimelineRowPoint | undefined;
            readonly gapAt: (offset: number) => number;
        },
        manager: DragManager,
        locale: ReturnType<typeof useLocale>,
    ) {
        // hand each drop on as a change
        const { options, window, tree, selection, scroller, rowAt, gapAt } = context;
        this.#options = options;
        this.#window = window;
        this.#tree = tree;
        this.#selection = selection;
        this.manager = manager;

        // follow a drag as its target changes, past pointer moves that keep it
        const aim = createMemo(() => this.manager.operation(), {
            equals: (previous, next) =>
                previous?.item === next?.item &&
                previous?.kind === next?.kind &&
                (previous?.target === undefined || next?.target === undefined
                    ? previous?.target === next?.target
                    : DragTarget.isSame(previous.target, next.target)),
        });

        // preview a move or resize on the track once per target, with every item it moves along
        this.#previews = createMemo(() => {
            // leave every item as it is without a move or resize aiming at a time
            const operation = aim();
            const target = operation?.target;
            if (operation === undefined || operation.kind === "link" || target?.kind !== "time") {
                return new Map<string, TimelineItem>();
            }
            const changed = tree.propagate(this.changes({ ...operation, target }));

            return new Map(changed.map((item) => [item.key, item]));
        });

        // take bars at a snapped time on the track, moved by the pointer's travel or a mark per key
        let trackRows: DOMRect | undefined;
        void new Droppable(
            {
                id: TRACK,
                group: TRACK,
                axis: "inline",
                element: scroller,
                accepts: (draggable, kind) => draggable.type === BAR_TYPE && kind !== "link",
                measure: () => {
                    trackRows = scroller()?.getBoundingClientRect();
                },
                targetAt: (_x, y, operation) => {
                    // move the dragged edge by the travel's share of the window, mirrored right to left
                    const isEnd = operation.kind === "resize-end";
                    const dragged = tree.item(operation.item);
                    const drawn = tree.drawn(dragged);
                    const travel =
                        locale.direction === "rtl" ? -operation.offsetX : operation.offsetX;
                    const moved = window.instant(
                        window.share(isEnd ? drawn.end : drawn.start) + travel / window.measured(),
                    );

                    // move an item drawn on another row onto the row under the pointer
                    const element = scroller();
                    const at = window.snap(moved, isEnd ? "end" : "start");
                    const isMoving = operation.kind === "move" && dragged.row !== undefined;
                    const under =
                        isMoving && trackRows !== undefined && element !== undefined
                            ? rowAt(y - trackRows.top + element.scrollTop)
                            : undefined;
                    const row = under === undefined ? undefined : tree.rows()[under.index];

                    return row === undefined
                        ? { kind: "time", droppable: TRACK, at }
                        : { kind: "time", droppable: TRACK, at, resource: row.item.key };
                },
                step: (target, step) => {
                    // step the dragged edge to the next or previous mark in the text's direction
                    const at = target.kind === "time" ? target.at : window.range().start;
                    const edge = this.manager.operation()?.kind === "resize-end" ? "end" : "start";
                    const direction =
                        step === "inline-end" ? 1 : step === "inline-start" ? -1 : undefined;

                    return direction === undefined
                        ? undefined
                        : { kind: "time", droppable: TRACK, at: window.step(at, direction, edge) };
                },
                enter: (from) => from,
                describe: (target) => {
                    // describe a time as the axis writes it, the track taking no other target
                    if (target.kind !== "time") {
                        throw new TypeError(`timeline time described as a ${target.kind} target`);
                    }

                    return { kind: "label", label: window.instantLabel(target.at) };
                },
                drop: (operation) => this.drop(operation),
            },
            this.manager,
        );

        // take links on the bars' edges: the row under the pointer, and the nearer edge of its bar
        let edgeRows: DOMRect | undefined;
        let edgeTrack: DOMRect | undefined;
        void new Droppable(
            {
                id: EDGES,
                group: EDGES,
                axis: "both",
                element: scroller,
                accepts: (draggable, kind) => draggable.type === BAR_TYPE && kind === "link",
                measure: () => {
                    edgeRows = scroller()?.getBoundingClientRect();
                    edgeTrack = window.track()?.getBoundingClientRect();
                },
                targetAt: (x, y, operation) => {
                    // read the row under the pointer, past the rows scrolled away
                    const element = scroller();
                    if (
                        edgeRows === undefined ||
                        edgeTrack === undefined ||
                        element === undefined
                    ) {
                        return undefined;
                    }
                    const top = y - edgeRows.top + element.scrollTop;
                    const at = rowAt(top);
                    const node = at === undefined ? undefined : tree.rows()[at.index];
                    const lane =
                        at === undefined || node === undefined
                            ? undefined
                            : tree.lanes(node.item.key)[at.lane];
                    if (lane === undefined) {
                        return undefined;
                    }

                    // take the nearest edge of the lane's bars to the pointer, mirrored right to left
                    const along = (x - edgeTrack.left) / edgeTrack.width;
                    const share = locale.direction === "rtl" ? 1 - along : along;
                    const edges = lane.flatMap((item) => {
                        const drawn = tree.drawn(item);

                        return [
                            { key: item.key, edge: "start", at: window.share(drawn.start) },
                            { key: item.key, edge: "end", at: window.share(drawn.end) },
                        ] as const;
                    });
                    const nearest = edges.reduce((best, each) =>
                        Math.abs(share - each.at) < Math.abs(share - best.at) ? each : best,
                    );
                    const target = {
                        kind: "edge",
                        droppable: EDGES,
                        key: nearest.key,
                        edge: nearest.edge,
                    } as const;

                    return this.isLinkable(operation, target) ? target : undefined;
                },
                step: (target, step) => {
                    // step to the bar's other edge, or along the rows past those the caller refuses
                    const operation = this.manager.operation();
                    if (target.kind !== "edge" || operation === undefined) {
                        return undefined;
                    }
                    const bars = tree.bars();
                    const offset = step === "block-start" ? -1 : step === "block-end" ? 1 : 0;
                    const edge =
                        step === "inline-start"
                            ? "start"
                            : step === "inline-end"
                              ? "end"
                              : target.edge;

                    // take the first linkable edge in the step's direction, or the same bar's edge
                    const index = bars.findIndex((bar) => bar.key === target.key);
                    const candidates =
                        offset === 0
                            ? [bars[index]]
                            : offset < 0
                              ? bars.slice(0, index).toReversed()
                              : bars.slice(index + 1);

                    return candidates
                        .map((bar) =>
                            bar === undefined ? undefined : { ...target, key: bar.key, edge },
                        )
                        .find((next) => next !== undefined && this.isLinkable(operation, next));
                },
                enter: (from) => from,
                describe: (target) => {
                    // name an edge by its item and which edge it is
                    if (target.kind !== "edge") {
                        throw new TypeError(`timeline edge described as a ${target.kind} target`);
                    }
                    const label = tree.item(target.key).label;
                    const message = target.edge === "start" ? t`${label}, start` : t`${label}, end`;

                    return { kind: "label", label: locale.render(message) };
                },
                drop: (operation) => this.drop(operation),
            },
            this.manager,
        );

        // take rows in the gaps between rows: the gap nearest the pointer, past the rows scrolled away
        let gapRows: DOMRect | undefined;
        let indent: number | undefined;
        void new Droppable(
            {
                id: ROWS,
                group: ROWS,
                axis: "block",
                element: scroller,
                accepts: (draggable) => draggable.type === ROW_TYPE,
                measure: () => {
                    // measure the rows and a level's indent, which the theme scales
                    const element = scroller();
                    gapRows = element?.getBoundingClientRect();
                    indent = element === undefined ? undefined : indentOf(element);
                },
                targetAt: (_x, y, operation) => {
                    // read the gap nearest the pointer, none inside the moved row's children
                    const element = scroller();
                    if (gapRows === undefined || indent === undefined || element === undefined) {
                        return undefined;
                    }
                    const index = tree.gapOf(
                        operation.item,
                        gapAt(y - gapRows.top + element.scrollTop),
                    );
                    if (index === undefined) {
                        return undefined;
                    }

                    // nest a level per indent the pointer travels along the text
                    const travel =
                        locale.direction === "rtl" ? -operation.offsetX : operation.offsetX;
                    const levels = Math.round(travel / indent);
                    const from = rowGapOf(operation.from).depth;
                    const depth = this.#depthIn(operation.item, index, from + levels);

                    return { kind: "list", droppable: ROWS, index, depth };
                },
                step: (target, step) => {
                    // nest a level deeper or shallower along the text, within the gap's depths
                    const operation = this.manager.operation();
                    const gap = rowGapOf(target);
                    if (operation === undefined) {
                        return undefined;
                    } else if (step === "inline-start" || step === "inline-end") {
                        const nested = gap.depth + (step === "inline-end" ? 1 : -1);
                        const depth = this.#depthIn(operation.item, gap.index, nested);

                        return depth === gap.depth ? undefined : { ...gap, depth };
                    }

                    // step a gap up or down to the next place, past the moved row's children
                    const index = tree.gapBeside(
                        operation.item,
                        gap.index,
                        step === "block-start" ? -1 : 1,
                    );
                    if (index === undefined) {
                        return undefined;
                    }
                    const depth = this.#depthIn(operation.item, index, gap.depth);

                    return { ...gap, index, depth };
                },
                enter: (from) => from,
                describe: (target) => {
                    // describe a gap by the item it lands under and the row after it
                    const operation = this.manager.operation();
                    const gap = rowGapOf(target);
                    if (operation === undefined) {
                        throw new TypeError("timeline row gap described outside a drag");
                    }
                    const move = tree.place(operation.item, gap.index, gap.depth);
                    const parent =
                        move.parent === undefined ? undefined : tree.item(move.parent).label;
                    const next = move.next === undefined ? undefined : tree.item(move.next).label;

                    return { kind: "label", label: locale.render(gapMessage(parent, next)) };
                },
                drop: (operation) => this.drop(operation),
            },
            this.manager,
        );
    }

    /** Hand on a drop as a change: a link, a row's new place, or the items it moved. */
    drop(operation: DragOperation & { readonly target: DragTarget }): void {
        // draw a link from the edge it left to another item's edge
        const { target } = operation;
        if (operation.kind === "link") {
            this.#link(operation);
        }
        // hand on a moved row as the place it lands in, where it moves
        else if (target.kind === "list") {
            const gap = rowGapOf(target);
            if (this.#tree.isMove(operation.item, gap.index, gap.depth)) {
                this.#options.onRowMove?.(this.#tree.place(operation.item, gap.index, gap.depth));
            }
        }
        // hand on the items as the drag leaves them, with the ones they move along
        else {
            this.#tree.commit(this.changes(operation));
        }
    }

    /** Hand on a link drawn from one item's edge onto another's, the pair of edges its type. */
    #link(operation: DragOperation & { readonly target: DragTarget }): void {
        const { from, target } = operation;
        if (from.kind !== "edge" || target.kind !== "edge") {
            throw new TypeError(`timeline link drawn from a ${from.kind} onto a ${target.kind}`);
        } else if (target.key !== from.key) {
            const type = ScheduleLink.joining(from.edge, target.edge);
            this.#options.onLinkCreate?.({ from: from.key, to: target.key, type, lag: 0 });
        }
    }

    /** Read the items a drag leaves: the dragged one, and the selected ones a move takes along. */
    changes(operation: DragOperation & { readonly target: DragTarget }): TimelineItem[] {
        // move the selection with a selected item, each by the dragged item's travel
        const changed = this.outcome(operation);
        const isGroup = operation.kind === "move" && this.#selection.isSelected(operation.item);
        if (!isGroup) {
            return [changed];
        }
        const travel = this.#travel(operation);
        const others = this.#selection
            .values()
            .filter((key) => key !== operation.item && this.#tree.items().has(key))
            .map((key) => this.#tree.item(key))
            .filter((item) => TimelineItem.isDated(item))
            .map((item) => this.#window.move(item, travel));

        return [changed, ...others];
    }

    /** Read the item a move or resize leaves at its target: its start or end moved, or both. */
    outcome(operation: DragOperation & { readonly target: DragTarget }): TimelineItem {
        // refuse anything but a time on the track
        const { target } = operation;
        if (target.kind !== "time") {
            throw new TypeError(`timeline drop onto a ${target.kind} target`);
        }
        const { at } = target;
        const item = this.#tree.item(operation.item);
        const drawn = this.#tree.drawn(item);

        // move one edge alone from its handle, a step from the other at least
        if (operation.kind === "resize-end") {
            return { ...item, end: Math.max(at, this.#window.step(drawn.start, 1, "end")) };
        } else if (operation.kind === "resize-start") {
            return { ...item, start: Math.min(at, this.#window.step(drawn.end, -1, "start")) };
        }

        // move the dates it has by the travel in the track's time, onto the row it lands on
        const moved = this.#window.move(item, this.#travel(operation));

        return target.resource === undefined ? moved : { ...moved, row: target.resource };
    }

    /** Read how far a move takes the dragged item's drawn start, in the track's time. */
    #travel(operation: DragOperation & { readonly target: DragTarget }): number {
        // refuse anything but a time on the track
        const { target } = operation;
        if (target.kind !== "time") {
            throw new TypeError(`timeline move onto a ${target.kind} target`);
        }
        const drawn = this.#tree.drawn(this.#tree.item(operation.item));

        return this.#window.calendar().between(drawn.start, target.at);
    }

    /** Report whether a link drawn from a drag's edge onto a target edge may join them. */
    isLinkable(operation: DragOperation, target: DragTarget & { readonly kind: "edge" }): boolean {
        // let the dragged item's edges through, and ask the caller about the rest
        const { from } = operation;
        if (from.kind !== "edge") {
            throw new TypeError(`timeline link drawn from a ${from.kind} target`);
        } else if (from.key === target.key) {
            return true;
        }
        const link = {
            from: from.key,
            to: target.key,
            type: ScheduleLink.joining(from.edge, target.edge),
            lag: 0,
        };

        return this.#options.canLink?.(link) ?? true;
    }

    /** Read an item as the drag in progress would leave it, or as it is. */
    preview(key: string): TimelineItem {
        return this.#previews().get(key) ?? this.#tree.item(key);
    }

    /** Read the gap a row drag aims at and its depth, none where the row would stay as it is. */
    rowGap(): { readonly index: number; readonly depth: number } | undefined {
        // show a gap only where the drop moves the row
        const operation = this.manager.operation();
        const target = this.manager.targetIn(ROWS);
        const gap = target === undefined ? undefined : rowGapOf(target);
        const isMove =
            operation !== undefined &&
            gap !== undefined &&
            this.#tree.isMove(operation.item, gap.index, gap.depth);

        return isMove ? gap : undefined;
    }

    /** Fit a depth within the depths a moved row may take in a gap. */
    #depthIn(key: string, index: number, depth: number): number {
        const { min, max } = this.#tree.depths(key, index);

        return Math.min(Math.max(depth, min), max);
    }

    /** Read the id of the draggable of a bar's part: its body, an edge's handle or connector. */
    barId(key: string, part: TimelineBarPart): string {
        return JSON.stringify([key, part]);
    }

    /** Create a bar's draggables: its body, each edge's handle, and each edge's connector. */
    draggables(
        key: string,
        elements: (part: TimelineBarPart) => HTMLElement | undefined,
    ): TimelineBarDrags {
        // add each part, standing at its time on the track or at its edge
        const draggable = (part: TimelineBarPart, kind: DragKind) =>
            new Draggable(
                {
                    id: this.barId(key, part),
                    item: key,
                    type: BAR_TYPE,
                    kind,
                    label: () => this.#tree.item(key).label,
                    element: () => elements(part),
                    home: (): DragTarget =>
                        part === "link-start" || part === "link-end"
                            ? {
                                  kind: "edge",
                                  droppable: EDGES,
                                  key,
                                  edge: part === "link-start" ? "start" : "end",
                              }
                            : {
                                  kind: "time",
                                  droppable: TRACK,
                                  at:
                                      part === "end"
                                          ? this.#tree.drawn(this.#tree.item(key)).end
                                          : this.#tree.drawn(this.#tree.item(key)).start,
                              },
                },
                this.manager,
            ).attributes([" "]);

        return {
            body: draggable("body", "move"),
            start: draggable("start", "resize-start"),
            end: draggable("end", "resize-end"),
            linkStart: draggable("link-start", "link"),
            linkEnd: draggable("link-end", "link"),
        };
    }

    /** Lift a part of an item's bar with the keys: its body, its end, or its end's connector. */
    lift(key: string, part: "body" | "end" | "link-end"): void {
        const kinds = { body: "move", end: "resize-end", "link-end": "link" } as const;
        this.manager.lift(this.barId(key, part), kinds[part], "keyboard");
    }
}

/** Write where a row lands: under an item, before a row, or both, or at the end. */
function gapMessage(parent: string | undefined, next: string | undefined): Message {
    if (parent !== undefined && next !== undefined) {
        return t`Under ${parent}, before ${next}`;
    } else if (parent !== undefined) {
        return t`Last under ${parent}`;
    } else if (next !== undefined) {
        return t`Before ${next}`;
    }
    // land last at the top
    else {
        return t`At the end`;
    }
}

/** Read a row drag's gap and depth, refusing any other target. */
function rowGapOf(
    target: DragTarget,
): DragTarget & { readonly kind: "list"; readonly depth: number } {
    if (target.kind !== "list" || target.depth === undefined) {
        throw new TypeError(`timeline row gap is invalid: a ${target.kind} target without a depth`);
    }

    return { ...target, depth: target.depth };
}

/** Measure the pixels a level of depth indents by, as the theme scales it. */
function indentOf(element: HTMLElement): number {
    // lay out a probe as wide as the indent, then take it away
    const probe = document.createElement("div");
    probe.style.width = ROW_INDENT;
    element.append(probe);
    const width = probe.getBoundingClientRect().width;
    probe.remove();

    // refuse an indent without width, which no travel could nest by
    if (width === 0) {
        throw new RangeError("timeline row indent is invalid: it measures no width");
    }

    return width;
}
