import type { Direction } from "@destack/locale";
import { nth, type ScheduleLink, type TickUnit, type TimeScale } from "@destack/schema";
import type * as style from "@destack/style";
import {
    type Accessor,
    createContext,
    createEffect,
    createMemo,
    createSignal,
    type JSX,
    onCleanup,
    untrack,
    useContext,
    type useEnvironment,
    type useLocale,
} from "@destack/view";
import { Focus } from "../focus/index.ts";
import type { InteractionEvent } from "../interaction/index.ts";
import { type MultipleSelection, Selection } from "../selection/index.ts";
import { Virtualizer } from "../virtualizer/index.ts";
import { TimelineDrag } from "./drag.ts";
import {
    RowsDelegate,
    type TimelineItem,
    type TimelineNode,
    type TimelineRowMove,
    TimelineTree,
} from "./tree.ts";
import { type TimelineAxisMode, TimelineWindow } from "./window.ts";

/** The height of a row in pixels by default, as Jaeger's and Linear's timelines run about. */
const ROW_HEIGHT = 32;

/** The width of the labels' column by default. */
const LABEL_WIDTH = "16rem";

/** The narrowest labels' column in pixels, room for a short label beside its fold. */
export const MIN_LABEL_WIDTH = 120;

/** How a bar is filled: with its color's tint, or solid. */
export type TimelineBarVariant = "tint" | "solid";

/** The items, window and handlers of a timeline. */
export interface TimelineOptions {
    /** The items, parents before their children. */
    readonly items: readonly TimelineItem[];
    /** The dependencies drawn between items. */
    readonly links?: readonly ScheduleLink[];
    /** How the axis reads, dates and times by default. */
    readonly axis?: TimelineAxisMode;
    /** The window shown, which makes the window controlled. */
    readonly window?: TimeScale;
    /** The window shown at first while uncontrolled, every item by default. */
    readonly defaultWindow?: TimeScale;
    /** Handle the person zooming or panning. */
    readonly onWindowChange?: (window: TimeScale) => void;
    /** Handle the person changing items as one change set, which makes the bars movable. */
    readonly onItemsChange?: (items: readonly TimelineItem[]) => void;
    /** Read every item a change moves, such as the tasks a schedule pushes, after Bryntum. */
    readonly propagate?: (changed: readonly TimelineItem[]) => readonly TimelineItem[];
    /** Handle the person moving a row, which makes the rows movable by a grip and keys. */
    readonly onRowMove?: (move: TimelineRowMove) => void;
    /** Handle the person opening an item. */
    readonly onAction?: (key: string) => void;
    /** The selection of items, several replaced by a plain press by default, after Linear. */
    readonly selection?: MultipleSelection;
    /** Report whether the person may draw a link, such as one closing no cycle, any by default. */
    readonly canLink?: (link: ScheduleLink) => boolean;
    /** Handle the person drawing a dependency from one bar's edge to another's. */
    readonly onLinkCreate?: (link: ScheduleLink) => void;
    /** Handle the person removing a dependency, with Delete on it. */
    readonly onLinkDelete?: (link: ScheduleLink) => void;
    /** Handle the person opening a dependency, with Enter or a double click, such as to edit it. */
    readonly onLinkAction?: (link: ScheduleLink) => void;
    /** What a moved edge snaps to: a unit's boundaries, nothing, or the axis's marks by default. */
    readonly snap?: TickUnit | false;
    /** Whether the track leaves out non-working days, so a week of work runs edge to edge. */
    readonly hideNonWorking?: boolean;
    /** The IANA time zone the axis marks local times in, the environment's by default. */
    readonly timeZone?: string;
    /** The height of a row in pixels, 32 by default, fixed once the timeline mounts. */
    readonly rowHeight?: number;
    /** The width of the labels' column at first, 16rem by default, which the person can drag. */
    readonly labelWidth?: string;
    /** Handle the person resizing the labels' column, such as to save its width. */
    readonly onLabelWidthChange?: (width: string) => void;
}

/** A timeline as its parts read it: its window, rows, drags, focus, selection and links. */
export class TimelineControl {
    /** The timeline's options. */
    readonly options: TimelineOptions;
    /** The time shown and how it maps onto the track. */
    readonly window: TimelineWindow;
    /** The items as rows. */
    readonly tree: TimelineTree;
    /** The drags of the bars, links and rows. */
    readonly drag: TimelineDrag;
    /** The focused row, the timeline's one tab stop. */
    readonly focus: Focus<string>;
    /** The selected items, which move together. */
    readonly selection: Selection;
    /** The rows in view. */
    readonly virtualizer: Virtualizer;
    /** The link under the pointer or focused, which its bars show too. */
    readonly highlight: Accessor<ScheduleLink | undefined>;
    /** Set the highlighted link, or none. */
    readonly setHighlight: (link: ScheduleLink | undefined) => void;
    /** The height of a row in pixels. */
    readonly rowHeight: number;
    /** The width of the labels' column. */
    readonly labelWidth: Accessor<string>;
    /** Set the element that scrolls the rows. */
    readonly attachScroller: (element: HTMLElement | undefined) => void;
    /** The top of each row in pixels and the rows' bottom last, each row a lane high per lane. */
    readonly tops: Accessor<readonly number[]>;
    /** The widths of the columns beside the labels, in their headers' order. */
    readonly columns: Accessor<readonly string[]>;
    /** Set the width of the labels' column. */
    readonly #setLabelWidth: (width: string) => void;
    /** The handlers of the bars whose labels may overflow them, by bar element. */
    readonly #clips = new Map<Element, (isClipped: boolean) => void>();
    /** The observer of the bars' sizes, created with the first bar. */
    #clipObserver: ResizeObserver | undefined;
    /** The headers of the columns beside the labels, with their widths, as they mounted. */
    readonly #headers: Accessor<readonly TimelineColumnHeader[]>;
    /** Replace the headers of the columns beside the labels. */
    readonly #setHeaders: (headers: readonly TimelineColumnHeader[]) => void;

    /** Follow a timeline's options in a locale on an environment. */
    constructor(
        options: TimelineOptions,
        locale: ReturnType<typeof useLocale>,
        environment: ReturnType<typeof useEnvironment>,
        ticking: Accessor<number | undefined>,
    ) {
        // nest the items as rows, and show them all until the person zooms
        this.options = options;
        this.tree = new TimelineTree(
            options,
            untrack(() => environment.clock.now()),
        );
        this.window = new TimelineWindow(options, this.tree, locale, environment, ticking);
        this.rowHeight = untrack(() => options.rowHeight ?? ROW_HEIGHT);
        const [labelWidth, setLabelWidth] = createSignal(
            untrack(() => options.labelWidth ?? LABEL_WIDTH),
        );
        this.labelWidth = labelWidth;
        this.#setLabelWidth = setLabelWidth;

        // lay out the columns beside the labels in their headers' order
        const [headers, setHeaders] = createSignal<readonly TimelineColumnHeader[]>([], {
            ownedWrite: true,
        });
        this.#headers = headers;
        this.#setHeaders = setHeaders;
        this.columns = createMemo(() => columnsOf(headers()));

        // stack the rows, each as high as its lanes
        this.tops = createMemo(() => {
            const tops = [0];
            for (const row of this.tree.rows()) {
                const lanes = this.tree.laneCount(row.item.key);
                tops.push(nth(tops, -1) + lanes * this.rowHeight);
            }

            return tops;
        });

        // render only the rows in view, measured again as their lanes change
        const [scroller, setScroller] = createSignal<HTMLElement | undefined>(undefined, {
            ownedWrite: true,
        });
        this.attachScroller = setScroller;
        this.virtualizer = new Virtualizer({
            count: () => this.tree.rows().length,
            estimateSize: (index) =>
                nth(untrack(this.tops), index + 1) - nth(untrack(this.tops), index),
            scrollElement: scroller,
            itemKey: createMemo(() => {
                // key each virtual row by its item, from the rows the count reads too
                const rows = this.tree.rows();

                return (index: number) => nth(rows, index).item.key;
            }),
        });
        createEffect(this.tops, () => this.virtualizer.remeasure());

        // move one focus through the rows, and select several as Finder and Linear do
        this.focus = new Focus({ delegate: new RowsDelegate(this.tree), mode: "roving" });
        this.selection = new Selection(
            untrack(() => options.selection) ?? { multiple: true, behavior: "replace" },
        );
        const [highlight, setHighlight] = createSignal<ScheduleLink | undefined>(undefined);
        this.highlight = highlight;
        this.setHighlight = setHighlight;

        // drag the bars along the track, links between edges and rows between rows
        this.drag = new TimelineDrag(
            {
                options,
                window: this.window,
                tree: this.tree,
                selection: this.selection,
                scroller,
                rowAt: (offset) => this.rowAt(offset),
                gapAt: (offset) => this.gapAt(offset),
            },
            locale,
            environment,
        );
    }

    /** Select or open a pressed item: open on a plain press, select with a modifier. */
    press(key: string, gesture: InteractionEvent): void {
        // select with a modifier, or on any press where nothing opens
        if (this.selection.isSelecting(gesture, this.options.onAction !== undefined)) {
            this.selection.select(key, gesture, this.tree.collection);
        }
        // open the item
        else {
            this.open(key);
        }
    }

    /** Open an item. */
    open(key: string): void {
        this.options.onAction?.(key);
    }

    /** Report whether an item is an end of the highlighted link. */
    isLinked(key: string): boolean {
        const link = this.highlight();

        return link !== undefined && (link.from === key || link.to === key);
    }

    /** Move a row a place up or down among the rows, as Notion moves blocks. */
    stepRow(key: string, direction: 1 | -1): void {
        const move = this.tree.step(key, direction);
        if (move !== undefined) {
            this.options.onRowMove?.(move);
        }
    }

    /** Read the top of a row in pixels, or the rows' bottom past the last. */
    rowTop(index: number): number {
        return nth(this.tops(), index);
    }

    /** Read the row at an offset from the rows' top, and the lane there, none past the rows. */
    rowAt(offset: number): TimelineRowPoint | undefined {
        // find the last row starting at or above the offset
        const tops = this.tops();
        const index = tops.findLastIndex((top) => top <= offset);
        if (index === -1 || index >= tops.length - 1) {
            return undefined;
        }

        return { index, lane: Math.floor((offset - nth(tops, index)) / this.rowHeight) };
    }

    /** Read the gap between rows nearest an offset from the rows' top. */
    gapAt(offset: number): number {
        const tops = this.tops();

        return tops.reduce(
            (best, top, gap) =>
                Math.abs(top - offset) < Math.abs(nth(tops, best) - offset) ? gap : best,
            0,
        );
    }

    /** Add a column beside the labels until its header unmounts. */
    addColumn(header: TimelineColumnHeader): void {
        this.#setHeaders([...untrack(this.#headers), header]);
        onCleanup(() => this.#setHeaders(untrack(this.#headers).filter((each) => each !== header)));
    }

    /** Place an item without dates in the window's middle, a step long. */
    schedule(key: string): void {
        // snap the window's middle, then end a step later
        const start = this.window.snap(this.window.instant(0.5), "start");
        const end = this.window.step(start, 1, "end");
        this.tree.commit([{ ...this.tree.item(key), start, end }]);
    }

    /** Report whether a bar's label overflows it as the bar resizes, until the bar unmounts. */
    observeClip(element: HTMLElement, onClip: (isClipped: boolean) => void): void {
        // measure every resized bar in one pass after layout, then report each
        this.#clipObserver ??= new ResizeObserver((entries) => {
            const clips = entries.map((entry) => {
                const target = entry.target;

                return [target, target.scrollWidth > target.clientWidth + 1] as const;
            });
            for (const [target, isClipped] of clips) {
                this.#clips.get(target)?.(isClipped);
            }
        });
        this.#clips.set(element, onClip);
        this.#clipObserver.observe(element);
        onCleanup(() => {
            this.#clips.delete(element);
            this.#clipObserver?.unobserve(element);
        });
    }

    /** Read the width the labels and the columns beside them take together. */
    labelArea(): string {
        const widths = [this.labelWidth(), ...this.columns()];

        return widths.length === 1 ? this.labelWidth() : `calc(${widths.join(" + ")})`;
    }

    /** Read the grid columns of the header and each row: labels, columns and the track. */
    template(): string {
        return [this.labelWidth(), ...this.columns(), "minmax(0, 1fr)"].join(" ");
    }

    /** Resize the labels' column to a width in pixels, leaving each column room. */
    resizeLabels(pixels: number, total: number): void {
        const width = `${String(Math.round(Math.min(Math.max(pixels, MIN_LABEL_WIDTH), total / 2)))}px`;
        this.#setLabelWidth(width);
        this.options.onLabelWidthChange?.(width);
    }

    /** Fold or unfold the focused row on its inline arrow, reporting whether it did. */
    fold(event: KeyboardEvent, direction: Direction): boolean {
        const key = this.focus.current();

        return key !== undefined && this.tree.fold(key, event, direction);
    }
}

/** The header of a column beside a timeline's labels, which sets the column's width. */
export interface TimelineColumnHeader {
    /** The header's element, which orders the columns. */
    readonly element: Accessor<HTMLElement | undefined>;
    /** The column's width, such as 6rem. */
    readonly width: Accessor<string>;
}

/** A place among a timeline's rows: a row's index and a lane in it. */
export interface TimelineRowPoint {
    /** The row's index among the visible rows. */
    readonly index: number;
    /** The lane's index in the row. */
    readonly lane: number;
}

/** The nearest timeline, null outside one. */
export const TimelineContext = createContext<TimelineControl | null>(null);

/** A row as its parts read it: its item and its place among the visible rows. */
export interface TimelineRowControl {
    /** The row, which follows its item's changes. */
    readonly node: Accessor<TimelineNode>;
    /** The row's place among the visible rows. */
    readonly index: Accessor<number>;
}

/** The nearest row of a timeline, null outside one. */
export const TimelineRowContext = createContext<TimelineRowControl | null>(null);

/** Read the nearest timeline, refusing a part outside one. */
export function useTimeline(): TimelineControl {
    const control = useContext(TimelineContext);
    if (control === null) {
        throw new TypeError("timeline part is invalid: it renders outside a timeline");
    }

    return control;
}

/** Read the nearest row of a timeline and its place, refusing a part outside one. */
export function useTimelineRow(): TimelineRowControl {
    const row = useContext(TimelineRowContext);
    if (row === null) {
        throw new TypeError("timeline row part is invalid: it renders outside a timeline's row");
    }

    return row;
}

/** The properties of a part of a timeline, the native element's attributes included. */
export interface TimelinePartProperties extends Omit<JSX.HTMLAttributes<HTMLDivElement>, "class"> {
    /** The StyleX styles applied after the part's. */
    readonly xstyle?: style.Styles;
}

/** List the widths of mounted column headers in their document order. */
function columnsOf(headers: readonly TimelineColumnHeader[]): string[] {
    // order the headers as the document does, an unmounted one last
    const ordered = headers.toSorted((left, right) => {
        // sort an unmounted header last, else by document position
        const leftElement = left.element();
        const rightElement = right.element();
        if (leftElement === undefined || rightElement === undefined) {
            return Number(leftElement === undefined) - Number(rightElement === undefined);
        }

        return leftElement.compareDocumentPosition(rightElement) & Node.DOCUMENT_POSITION_FOLLOWING
            ? -1
            : 1;
    });

    return ordered.map((header) => header.width());
}
