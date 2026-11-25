import { size, width } from "@destack/theme/tokens.stylex";
import type { Hue } from "@destack/color";
import type { Direction } from "@destack/locale";
import {
    nth,
    type ScheduleLink,
    type WorkWeek,
    type TickUnit,
    type TimeScale,
} from "@destack/schema";
import type * as style from "@destack/style";
import {
    type Accessor,
    createContext,
    createEffect,
    createMemo,
    createSignal,
    type JSX,
    untrack,
    useContext,
    type useEnvironment,
    type useLocale,
} from "@destack/view";
import type { DragManager } from "../drag/index.ts";
import { Focus } from "../focus/index.ts";
import type { InteractionEvent, Move } from "../interaction/index.ts";
import type { ElementPartProperties } from "../part/index.ts";
import { type MultipleSelection, Selection } from "../selection/index.ts";
import { Length, Virtualizer } from "../virtualizer/index.ts";
import { type TimelineBarDrags, type TimelineBarPart, TimelineDrag } from "./drag.ts";
import {
    RowsDelegate,
    type TimelineBarVariant,
    TimelineItem,
    type TimelineNode,
    type TimelineRowMove,
    TimelineTree,
} from "./tree.ts";
import type { TimelineAxisMode } from "./scale.ts";
import { TimelineWindow } from "./window.ts";
import type { Render } from "../part/index.ts";
import { CollectionBuilder } from "../collection/index.ts";

/** The height of a row in pixels until it is measured, about Jaeger's. */
const ROW_HEIGHT = 32;

/** The width of the labels' column by default. */
const LABEL_WIDTH = `min(${width.sidebar}, 50%)`;

/** The narrowest labels' column in pixels, room for a short label beside its fold. */
export const MIN_LABEL_WIDTH = 120;

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
    /** Read every item a change moves, such as the tasks a schedule pushes. */
    readonly propagate?: (changed: readonly TimelineItem[]) => readonly TimelineItem[];
    /** Handle the person moving a row, which makes the rows movable by a grip and keys. */
    readonly onRowMove?: (move: TimelineRowMove) => void;
    /** Handle the person opening an item. */
    readonly onAction?: (key: string) => void;
    /** The selection of items, several replaced by a plain press by default. */
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
    /** The working hours the track keeps while hiding, the locale's weekdays whole by default. */
    readonly workWeek?: WorkWeek;
    /** The IANA time zone the axis marks local times in, the environment's by default. */
    readonly timeZone?: string;
    /** The height of each row's lanes in pixels, or a row's from the standard, a control's. */
    readonly rowHeight?: number | ((node: TimelineNode, standard: number) => number);
    /** How the bars of a row that overlap share it: in lanes by default, or over one another. */
    readonly overlap?: "stack" | "cover";
    /** The width of the labels' column at first, a sidebar's by default. */
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
    /** Read the height of a lane of a row in pixels. */
    readonly rowHeight: (node: TimelineNode) => number;
    /** The length that measures a row's height, as tall as a control. */
    readonly rowLength: Length;
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
    readonly #headers: CollectionBuilder<TimelineColumnHeader>;

    /** Follow a timeline's options in a locale on an environment. */
    constructor(
        options: TimelineOptions,
        manager: DragManager,
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
        this.rowLength = new Length(() => size[2]);
        this.rowHeight = (node) => {
            // take the caller's height, or the standard's
            const given = options.rowHeight;
            const standard = this.rowLength.pixels() ?? ROW_HEIGHT;

            return typeof given === "function" ? given(node, standard) : (given ?? standard);
        };
        const [labelWidth, setLabelWidth] = createSignal(
            untrack(() => options.labelWidth ?? LABEL_WIDTH),
        );
        this.labelWidth = labelWidth;
        this.#setLabelWidth = setLabelWidth;

        // lay out the columns beside the labels in their headers' order
        this.#headers = new CollectionBuilder((header) => header.element());
        this.columns = () => this.#headers.items().map((header) => header.width());

        // stack the rows, each as high as its lanes
        this.tops = createMemo(() => {
            const tops = [0];
            for (const row of this.tree.rows()) {
                const lanes = this.tree.laneCount(row.item.key);
                tops.push(nth(tops, -1) + lanes * this.rowHeight(row));
            }

            return tops;
        });

        // render only the rows in view, measured again as their lanes change
        const [scroller, setScroller] = createSignal<HTMLElement | undefined>(undefined, {
            ownedWrite: true,
        });
        this.attachScroller = setScroller;
        const estimate = createMemo(() => {
            // size each row by its lanes, a new size laying the rows out again
            const tops = this.tops();

            return (index: number) => nth(tops, index + 1) - nth(tops, index);
        });
        this.virtualizer = new Virtualizer({
            count: () => this.tree.rows().length,
            estimateSize: estimate,
            scrollElement: scroller,
            itemKey: createMemo(() => {
                // key each virtual row by its item, from the rows the count reads too
                const rows = this.tree.rows();

                return (index: number) => nth(rows, index).item.key;
            }),
        });

        // move one focus through the rows, and select several
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
            manager,
            locale,
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

    /** Move a row a place up or down among the rows. */
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

        // count the lanes down the row
        const height = this.rowHeight(nth(this.tree.rows(), index));

        return { index, lane: Math.floor((offset - nth(tops, index)) / height) };
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
        this.#headers.add(header);
    }

    /** Place an item without dates in the window's middle, a step long. */
    schedule(key: string): void {
        // snap the window's middle, then end a step later
        const start = this.window.snap(this.window.instant(0.5), "start");
        const end = this.window.step(start, 1, "end");
        this.tree.commit([{ ...this.tree.item(key), start, end }]);
    }

    /** Report whether a bar's label overflows it as the bar resizes, for each element it mounts. */
    observeClip(
        element: Accessor<HTMLElement | undefined>,
        onClip: (isClipped: boolean) => void,
    ): void {
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
        const observer = this.#clipObserver;

        // observe each element the bar mounts, and stop as it unmounts
        createEffect(element, (each) => {
            if (each === undefined) {
                return undefined;
            }
            this.#clips.set(each, onClip);
            observer.observe(each);

            return () => {
                this.#clips.delete(each);
                observer.unobserve(each);
            };
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
        const labelWidth = `${String(Math.round(Math.min(Math.max(pixels, MIN_LABEL_WIDTH), total / 2)))}px`;
        this.#setLabelWidth(labelWidth);
        this.options.onLabelWidthChange?.(labelWidth);
    }

    /** Bring an item into view: unfold the rows over it, scroll to its row and show its time. */
    reveal(key: string): void {
        // unfold the rows over it, and scroll once they show
        const item = untrack(() => this.tree.item(key));
        untrack(() => this.tree.unfold(key));
        queueMicrotask(() => {
            // find the row the item draws on
            const rows = untrack(() => this.tree.rows());
            const index = rows.findIndex((row) => row.item.key === (item.row ?? key));
            if (index !== -1) {
                this.virtualizer.scrollToIndex(index);
            }
        });

        // show its time where it lies past the window
        untrack(() => this.#revealTime(item));
    }

    /** Show an item's time where it lies past the window. */
    #revealTime(item: TimelineItem): void {
        // leave a window that shows any of it
        if (TimelineItem.isDated(item)) {
            const { start, end } = this.tree.drawn(item);
            const isShown = this.window.share(end) >= 0 && this.window.share(start) <= 1;
            if (!isShown) {
                this.window.reveal((start + end) / 2);
            }
        }
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

/** A bar as its parts read it: its item, its place, and what drags it. */
export interface TimelineBarControl {
    /** The item's key. */
    readonly key: string;
    /** The item, as a drag previews it. */
    readonly item: Accessor<TimelineItem>;
    /** The bar's place among the bars of its row that overlap, from the top. */
    readonly lane: Accessor<number>;
    /** The bar's start, as a share of the window. */
    readonly from: Accessor<number>;
    /** The bar's end, as a share of the window. */
    readonly to: Accessor<number>;
    /** The bar's color. */
    readonly hue: Accessor<Hue>;
    /** How the bar is drawn. */
    readonly variant: Accessor<TimelineBarVariant>;
    /** Whether the item is one instant. */
    readonly isMilestone: Accessor<boolean>;
    /** Whether the bar is being moved. */
    readonly isMoving: Accessor<boolean>;
    /** The edge without a date, none on a dated item. */
    readonly open: Accessor<"start" | "end" | undefined>;
    /** How much of the item is done, from 0 to 1, as its handle is dragged. */
    readonly progress: Accessor<number | undefined>;
    /** Set how much of the item is done. */
    readonly commitProgress: (done: number) => void;
    /** The drags of the bar's parts, none while nothing moves or links. */
    readonly drags: TimelineBarDrags | undefined;
    /** The attributes of the handle that drags how much is done. */
    readonly progressDrag: ReturnType<Move["attributes"]>;
    /** Add the card that describes the bar, until it unmounts. */
    readonly addCard: () => void;
    /** Register a part's element, which drags lift. */
    readonly register: (part: TimelineBarPart) => (element: HTMLElement) => void;
}

/** Keep a press on what a person drags from what it stands on. */
export function leave(event: Event): void {
    event.stopPropagation();
}

/** Whether the nearest part stands on a track, which places it by the track's width. */
export const TimelineTrackContext = createContext(false);

/** The nearest bar of a timeline, null outside one. */
export const TimelineBarContext = createContext<TimelineBarControl | null>(null);

/** Read the nearest bar of a timeline, refusing a part outside one. */
export function useTimelineBar(): TimelineBarControl {
    const bar = useContext(TimelineBarContext);
    if (bar === null) {
        throw new TypeError("timeline bar part is invalid: it renders outside a timeline's track");
    }

    return bar;
}

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
    /** Render another element with its attributes, its own by default. */
    readonly render?: Render;
}

/** The properties of an element part of a timeline, the native element's attributes included. */
export type TimelineElementProperties<Target extends HTMLElement = HTMLSpanElement> = Omit<
    JSX.HTMLAttributes<Target>,
    "class"
> &
    ElementPartProperties;
