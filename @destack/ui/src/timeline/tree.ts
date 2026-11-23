import type { Hue } from "@destack/color";
import type { Direction } from "@destack/locale";
import { Duration, nth, type TimeScale } from "@destack/schema";
import { type Accessor, createMemo, createSignal, untrack } from "@destack/view";
import { Collection } from "../collection/index.ts";
import type { KeyboardDelegate } from "../focus/index.ts";
import type { TimelineOptions } from "./control.ts";
import { DAY } from "./window.ts";

/** The milliseconds of an hour, the span an empty timeline shows. */
const HOUR = Duration.milliseconds({ hours: 1 });

/** The share of the span added on each side, so the first and last bars clear the edges. */
const MARGIN = 0.02;

/** The share of the items' span a bar with one date draws toward its missing date. */
const OPEN_SHARE = 0.08;

/** An item on a timeline: a task of a plan, or a span of a trace. */
export interface TimelineItem {
    /** The item's key. */
    readonly key: string;
    /** The item's label. */
    readonly label: string;
    /** The start, in epoch milliseconds, unknown when absent. */
    readonly start?: number;
    /** The end, in epoch milliseconds, at or after the start, unknown when absent. */
    readonly end?: number;
    /** The key of the item it belongs under, absent at the top. */
    readonly parent?: string;
    /** The key of the row it is drawn on, such as a person's, its row when absent. */
    readonly row?: string;
    /** The item's color, blue by default. */
    readonly color?: Hue;
    /** How much of it is done, from 0 to 1, drawn as a fill. */
    readonly progress?: number;
    /** The dates it was planned for, drawn beneath it. */
    readonly baseline?: TimeScale;
    /** The time it can slip without moving the plan's end, drawn as a line after it. */
    readonly slack?: number;
    /** The time it is due, in epoch milliseconds, past which its bar shows as late. */
    readonly deadline?: number;
    /** The milestones along it, drawn as diamonds in its row. */
    readonly milestones?: readonly TimelineMilestone[];
}

/** A milestone along an item: a label at a time. */
export interface TimelineMilestone {
    /** The milestone's key among its item's milestones. */
    readonly key: string;
    /** The milestone's label. */
    readonly label: string;
    /** The time it falls on, in epoch milliseconds. */
    readonly at: number;
}

/** The verbs of items. */
export const TimelineItem = {
    /** Report whether an item has a date, which a bar draws from. */
    isDated(item: TimelineItem): boolean {
        return item.start !== undefined || item.end !== undefined;
    },

    /** Report whether an item ends past its deadline, an item without an end never. */
    isLate(item: TimelineItem): boolean {
        return item.deadline !== undefined && item.end !== undefined && item.end > item.deadline;
    },
};

/** A row of a timeline: an item, its depth and whether items belong under it. */
export interface TimelineNode {
    /** The item. */
    readonly item: TimelineItem;
    /** The depth below the top, zero at the top. */
    readonly depth: number;
    /** Whether items belong under it. */
    readonly hasChildren: boolean;
}

/** A row a person moved: where it lands, after Sortable's moves. */
export interface TimelineRowMove {
    /** The moved item's key. */
    readonly key: string;
    /** The key of the item it lands under, absent at the top. */
    readonly parent?: string;
    /** The key of the item before it among its siblings, absent at the start. */
    readonly previous?: string;
    /** The key of the item after it among its siblings, absent at the end. */
    readonly next?: string;
}

/** The depths a moved row may take in a gap. */
export interface TimelineDepths {
    /** The shallowest, the row after the gap's. */
    readonly min: number;
    /** The deepest, one below the row before the gap where that row is open. */
    readonly max: number;
}

/** The arrow keys that point into and out of a row along a text direction. */
interface TimelineArrows {
    /** The key that steps into a row's children and unfolds it. */
    readonly inward: string;
    /** The key that steps out to a row's parent and folds it. */
    readonly outward: string;
}

/** The items of a timeline as rows: nested under their parents, folded, and moved between. */
export class TimelineTree {
    /** The items by key. */
    readonly items: Accessor<ReadonlyMap<string, TimelineItem>>;
    /** The visible rows, children under their unfolded parents. */
    readonly rows: Accessor<readonly TimelineNode[]>;
    /** The keys of the folded items, whose children are hidden. */
    readonly folded: Accessor<ReadonlySet<string>>;
    /** Whether an item belongs under another, so the labels leave room for a fold. */
    readonly isNested: Accessor<boolean>;
    /** The span from the first start to the last end, with a margin on each side. */
    readonly span: Accessor<TimeScale>;
    /** The first start, which a trace's offsets count from. */
    readonly origin: Accessor<number>;
    /** The rows shown, in order, which a selection's range runs along. */
    readonly collection: Collection<TimelineNode>;
    /** The items drawn on their rows, by the key of the item they belong under. */
    readonly #children: Accessor<ReadonlyMap<string | undefined, readonly TimelineItem[]>>;
    /** The dated items of each row by its key, packed into lanes where they overlap. */
    readonly #lanes: Accessor<ReadonlyMap<string, readonly (readonly TimelineItem[])[]>>;
    /** The dated items under each folded item, by its key. */
    readonly #rollups: Accessor<ReadonlyMap<string, readonly TimelineItem[]>>;
    /** The timeline's options. */
    readonly #options: TimelineOptions;
    /** Replace the folds. */
    readonly #setFolded: (folded: ReadonlySet<string>) => void;

    /** Nest a timeline's items under their parents, spanning them from the time it opened. */
    constructor(options: TimelineOptions, opened: number) {
        // group the items under their parents
        const [folded, setFolded] = createSignal<ReadonlySet<string>>(new Set());
        this.#options = options;
        this.folded = folded;
        this.#setFolded = setFolded;
        this.items = createMemo(() => itemsOf(options.items));
        this.#children = createMemo(() => childrenOf(options.items, this.items()));
        this.rows = createMemo(() => rowsOf(this.#children(), folded()));
        this.isNested = createMemo(() =>
            [...this.#children().keys()].some((key) => key !== undefined),
        );

        // span every item with a margin, a day at least on a plan's axis
        const least = untrack(() => (options.axis === "offset" ? 1 : DAY));
        this.span = createMemo(() => spanOf(options.items, opened, least));
        this.origin = createMemo(() => extentOf(options.items)?.start ?? this.span().start);

        // pack each row's bars into lanes, and gather each folded item's dated descendants
        this.#lanes = createMemo(() => lanesOf(options.items, (item) => this.drawn(item)));
        this.#rollups = createMemo(() => rollupsOf(this.#children(), folded()));
        this.collection = new Collection({
            sections: () => [{ key: "rows", items: this.rows() }],
            key: (node) => node.item.key,
            text: (node) => node.item.label,
        });
    }

    /** Hand on changed items as one change set, with every item they move along. */
    commit(changed: readonly TimelineItem[]): void {
        this.#options.onItemsChange?.(this.propagate(changed));
    }

    /** Read changed items with every item they move along, the changed ones alone by default. */
    propagate(changed: readonly TimelineItem[]): readonly TimelineItem[] {
        return this.#options.propagate?.(changed) ?? changed;
    }

    /** Read an item by its key, refusing a key the timeline does not have. */
    item(key: string): TimelineItem {
        const item = this.items().get(key);
        if (item === undefined) {
            throw new TypeError(`timeline item without an entry: '${key}'`);
        }

        return item;
    }

    /** Read the span an item's bar draws, a missing date open by a share of the items' span. */
    drawn(item: TimelineItem): TimeScale {
        // open a missing date by a share of the span
        const span = this.span();
        const open = (span.end - span.start) * OPEN_SHARE;
        if (item.start !== undefined && item.end !== undefined) {
            return { start: item.start, end: item.end };
        } else if (item.start !== undefined) {
            return { start: item.start, end: item.start + open };
        } else if (item.end !== undefined) {
            return { start: item.end - open, end: item.end };
        }
        // refuse an item without dates, which draws no bar
        else {
            throw new TypeError(`timeline item '${item.key}' has no dates to draw`);
        }
    }

    /** Read the dated items of a row packed into lanes, none for a row without them. */
    lanes(row: string): readonly (readonly TimelineItem[])[] {
        return this.#lanes().get(row) ?? [];
    }

    /** List the items drawn on the visible rows, row by row and lane by lane. */
    bars(): readonly TimelineItem[] {
        return this.rows().flatMap((row) => this.lanes(row.item.key).flat());
    }

    /** Count the lanes of a row, one at least, which its height follows. */
    laneCount(row: string): number {
        return Math.max(1, this.lanes(row).length);
    }

    /** Read the row an item is drawn on and its lane there. */
    where(key: string): { readonly row: string; readonly lane: number } {
        // find the item's row, then its lane among the row's
        const item = this.item(key);
        const row = item.row ?? item.key;
        const lane = this.lanes(row).findIndex((each) => each.some((other) => other.key === key));
        if (lane === -1) {
            throw new RangeError(`timeline item '${key}' is invalid: it has no dates to draw`);
        }

        return { row, lane };
    }

    /** List the dated items under a folded item, which its row draws small, none while unfolded. */
    rollup(key: string): readonly TimelineItem[] {
        return this.#rollups().get(key) ?? [];
    }

    /** Report whether a row shows its children: it has some and is not folded. */
    isUnfolded(node: TimelineNode): boolean {
        return node.hasChildren && !this.folded().has(node.item.key);
    }

    /** Fold or unfold an item's children. */
    toggle(key: string): void {
        const next = new Set(this.folded());
        if (next.has(key)) {
            next.delete(key);
        } else {
            next.add(key);
        }
        this.#setFolded(next);
    }

    /** Fold or unfold a row by the arrow pointing out of or into it, reporting whether it did. */
    fold(key: string, event: KeyboardEvent, direction: Direction): boolean {
        // fold a row with children on the outward arrow, unfold it on the inward one
        const node = this.rows().find((row) => row.item.key === key);
        const { inward, outward } = arrowsOf(direction);
        const isFolded = this.folded().has(key);
        const isFolding = event.key === outward && !isFolded;
        const isUnfolding = event.key === inward && isFolded;
        if (node?.hasChildren === true && (isFolding || isUnfolding)) {
            this.toggle(key);

            return true;
        }

        return false;
    }

    /** Unfold every item. */
    expandAll(): void {
        this.#setFolded(new Set());
    }

    /** Fold every item with children. */
    collapseAll(): void {
        const keys = [...this.#children().keys()].filter((key) => key !== undefined);
        this.#setFolded(new Set(keys));
    }

    /** Unfold an item's siblings with children, as a tree's asterisk does. */
    expandSiblings(key: string): void {
        // unfold every item under the same parent as the given one
        const siblings = this.#children().get(this.item(key).parent) ?? [];
        const next = new Set(this.folded());
        for (const sibling of siblings) {
            next.delete(sibling.key);
        }
        this.#setFolded(next);
    }

    /** Read the gap a moved row lands in: none inside its children, its place just below them. */
    gapOf(key: string, index: number): number | undefined {
        // find the row and the rows under it, which follow it until a row as shallow
        const { at, end } = this.#family(key);
        if (index > at && index < end) {
            return undefined;
        }

        return index === end ? at : index;
    }

    /** Read the depths a moved row may take in a gap, bounded by the rows around it. */
    depths(key: string, index: number): TimelineDepths {
        // read the rows around the gap, past the moved row and its children
        const { before, after } = this.#around(key, index);
        const isOpen = before !== undefined && (!before.hasChildren || this.isUnfolded(before));
        const deepest = before === undefined ? 0 : before.depth + (isOpen ? 1 : 0);

        return { min: after?.depth ?? 0, max: deepest };
    }

    /** Report whether a row landing in a gap at a depth moves from where it stands. */
    isMove(key: string, index: number, depth: number): boolean {
        const { at } = this.#family(key);

        return index !== at || depth !== nth(this.rows(), at).depth;
    }

    /** Read where a row lands in a gap at a depth: under the nearest row a level up. */
    place(key: string, index: number, depth: number): TimelineRowMove {
        // take the nearest row a level up as the parent, and the nearest at the depth as the previous
        const { previous: earlier, after } = this.#around(key, index);
        const parent = earlier.findLast((row) => row.depth < depth);
        const previous = earlier.findLast((row) => row.depth <= depth);
        const isSibling = (row: TimelineNode | undefined): row is TimelineNode =>
            row !== undefined && row.depth === depth;

        return {
            key,
            ...(parent === undefined || depth === 0 ? {} : { parent: parent.item.key }),
            ...(isSibling(previous) ? { previous: previous.item.key } : {}),
            ...(isSibling(after) ? { next: after.item.key } : {}),
        };
    }

    /** Read the gap after or before a gap that a moved row lands in, none past the ends. */
    gapBeside(key: string, index: number, direction: 1 | -1): number | undefined {
        // step past the moved row's children, the gap below them being its own place
        const { at, end } = this.#family(key);
        const forward = index === at ? end + 1 : index + 1;
        const backward = index - 1 === end ? at : index - 1;
        const next = direction > 0 ? forward : backward;

        return next < 0 || next > this.rows().length ? undefined : next;
    }

    /** Read where a row lands a place up or down, at the shallowest depth the gap allows. */
    step(key: string, direction: 1 | -1): TimelineRowMove | undefined {
        // land in the next gap, nesting only where the gap asks for it
        const index = this.gapBeside(key, this.#indexOf(key), direction);

        return index === undefined
            ? undefined
            : this.place(key, index, this.depths(key, index).min);
    }

    /** Read a shown row's index and the index of the first row after its children. */
    #family(key: string): { readonly at: number; readonly end: number } {
        // end at the first row as shallow as the row
        const rows = this.rows();
        const at = this.#indexOf(key);
        const depth = nth(rows, at).depth;
        const after = rows.slice(at + 1).findIndex((row) => row.depth <= depth);

        return { at, end: after === -1 ? rows.length : at + 1 + after };
    }

    /** Read the rows before a gap and the row after it, past a moved row and its children. */
    #around(
        key: string,
        index: number,
    ): {
        readonly previous: readonly TimelineNode[];
        readonly before: TimelineNode | undefined;
        readonly after: TimelineNode | undefined;
    } {
        // leave the moved row and its children out of both sides
        const rows = this.rows();
        const { at, end } = this.#family(key);
        const previous = rows.slice(0, index).filter((_, each) => each < at || each >= end);
        const after = rows.slice(index).find((_, each) => index + each < at || index + each >= end);

        return { previous, before: previous.at(-1), after };
    }

    /** Read the index of a shown row, refusing a row the folds hide. */
    #indexOf(key: string): number {
        const index = this.rows().findIndex((row) => row.item.key === key);
        if (index === -1) {
            throw new RangeError(`timeline row '${key}' moved while not shown`);
        }

        return index;
    }
}

/** The rows a timeline's keys move through, as a tree's: up and down, folds left and right. */
export class RowsDelegate implements KeyboardDelegate<string> {
    /** The rows. */
    readonly #tree: TimelineTree;

    /** Move through a timeline's rows. */
    constructor(tree: TimelineTree) {
        this.#tree = tree;
    }

    /** Read the row a key moves the focus to, the inline arrows stepping in and out of folds. */
    target(event: KeyboardEvent, from: string, direction: Direction): string | undefined {
        // find the row the move starts from
        const rows = this.#tree.rows();
        const index = rows.findIndex((row) => row.item.key === from);
        const row = rows[index];
        if (row === undefined) {
            return undefined;
        }
        const { inward, outward } = arrowsOf(direction);

        // step down and up the rows, or to their ends
        if (event.key === "ArrowDown") {
            return rows[Math.min(index + 1, rows.length - 1)]?.item.key;
        } else if (event.key === "ArrowUp") {
            return rows[Math.max(index - 1, 0)]?.item.key;
        } else if (event.key === "Home") {
            return rows[0]?.item.key;
        } else if (event.key === "End") {
            return rows.at(-1)?.item.key;
        }
        // step into an unfolded row's first child
        else if (event.key === inward && this.#tree.isUnfolded(row)) {
            return rows[index + 1]?.item.key;
        }
        // step out of a folded or childless row to its parent
        else if (event.key === outward && !this.#tree.isUnfolded(row)) {
            return row.item.parent;
        }
        // leave every other key alone
        else {
            return undefined;
        }
    }

    /** Read the first row. */
    first(): string | undefined {
        return this.#tree.rows()[0]?.item.key;
    }

    /** Report whether a key is a visible row. */
    has(key: string): boolean {
        return this.#tree.rows().some((row) => row.item.key === key);
    }
}

/** Read the arrows pointing into and out of a row along a text direction. */
function arrowsOf(direction: Direction): TimelineArrows {
    return direction === "rtl"
        ? { inward: "ArrowLeft", outward: "ArrowRight" }
        : { inward: "ArrowRight", outward: "ArrowLeft" };
}

/** Read items by key, refusing a row the items lack or one drawn on another row. */
function itemsOf(items: readonly TimelineItem[]): Map<string, TimelineItem> {
    // refuse a row the items lack, or one that is drawn on another row itself
    const byKey = new Map(items.map((item) => [item.key, item]));
    for (const item of items) {
        const row = item.row === undefined ? undefined : byKey.get(item.row);
        if (item.row !== undefined && row === undefined) {
            throw new RangeError(
                `timeline item '${item.key}' is invalid: its row '${item.row}' is missing`,
            );
        } else if (row?.row !== undefined) {
            throw new RangeError(
                `timeline item '${item.key}' is invalid: its row '${row.key}' is drawn on another row`,
            );
        }
    }

    return byKey;
}

/** Group the items drawn on their rows by parent, refusing missing parents and loops. */
function childrenOf(
    items: readonly TimelineItem[],
    byKey: ReadonlyMap<string, TimelineItem>,
): Map<string | undefined, TimelineItem[]> {
    // group the items drawn on their rows by parent in order, refusing missing parents
    const children = new Map<string | undefined, TimelineItem[]>();
    const rowed = items.filter((item) => item.row === undefined);
    for (const item of rowed) {
        const parent = item.parent;
        if (parent !== undefined && !byKey.has(parent)) {
            throw new RangeError(
                `timeline item '${item.key}' is invalid: its parent '${parent}' is missing`,
            );
        }
        const siblings = children.get(parent);
        if (siblings === undefined) {
            children.set(parent, [item]);
        } else {
            siblings.push(item);
        }
    }

    // refuse items no top-level item leads to, which their parents loop between
    const reached = new Set<string>();
    const queue = [...(children.get(undefined) ?? [])];
    for (let item = queue.pop(); item !== undefined; item = queue.pop()) {
        reached.add(item.key);
        queue.push(...(children.get(item.key) ?? []));
    }
    if (reached.size < rowed.length) {
        const looped = rowed.filter((item) => !reached.has(item.key)).map((item) => item.key);
        throw new RangeError(`timeline items '${looped.join("', '")}' loop through their parents`);
    }

    return children;
}

/** Flatten grouped items into rows, each child under its parent, below folded items left out. */
function rowsOf(
    children: ReadonlyMap<string | undefined, readonly TimelineItem[]>,
    folded: ReadonlySet<string>,
): TimelineNode[] {
    // walk the tree depth first, skipping below folded items
    const rows: TimelineNode[] = [];
    const visit = (parent: string | undefined, depth: number) => {
        for (const item of children.get(parent) ?? []) {
            const hasChildren = children.has(item.key);
            rows.push({ item, depth, hasChildren });
            if (hasChildren && !folded.has(item.key)) {
                visit(item.key, depth + 1);
            }
        }
    };
    visit(undefined, 0);

    return rows;
}

/** Gather the dated items at any depth under each folded item. */
function rollupsOf(
    children: ReadonlyMap<string | undefined, readonly TimelineItem[]>,
    folded: ReadonlySet<string>,
): Map<string, TimelineItem[]> {
    // walk down from each folded item in the rows' order, collecting the dated ones
    const rollups = new Map<string, TimelineItem[]>();
    for (const key of folded) {
        const below: TimelineItem[] = [];
        const visit = (parent: string) => {
            for (const item of children.get(parent) ?? []) {
                if (TimelineItem.isDated(item)) {
                    below.push(item);
                }
                visit(item.key);
            }
        };
        visit(key);
        rollups.set(key, below);
    }

    return rollups;
}

/** Pack each row's dated items into lanes by start, each into the first lane it fits after. */
function lanesOf(
    items: readonly TimelineItem[],
    drawn: (item: TimelineItem) => TimeScale,
): Map<string, TimelineItem[][]> {
    // group the dated items by the row they are drawn on
    const rows = new Map<string, TimelineItem[]>();
    for (const item of items) {
        const row = item.row ?? item.key;
        const group = rows.get(row);
        if (!TimelineItem.isDated(item)) {
            continue;
        } else if (group === undefined) {
            rows.set(row, [item]);
        } else {
            group.push(item);
        }
    }

    // place each item in the first lane that ends before it starts, or in a new lane
    const packed = new Map<string, TimelineItem[][]>();
    for (const [row, group] of rows) {
        const lanes: TimelineItem[][] = [];
        const ends: number[] = [];
        const byStart = group.toSorted((left, right) => drawn(left).start - drawn(right).start);
        for (const item of byStart) {
            // take an instant at the least, so milestones at one time stack
            const { start, end } = drawn(item);
            const taken = Math.max(end, start + 1);
            const lane = ends.findIndex((laneEnd) => laneEnd <= start);
            if (lane === -1) {
                lanes.push([item]);
                ends.push(taken);
            } else {
                nth(lanes, lane).push(item);
                ends[lane] = taken;
            }
        }
        packed.set(row, lanes);
    }

    return packed;
}

/** Read the first start and last end of the items in one pass, none for no items. */
function extentOf(items: readonly TimelineItem[]): TimeScale | undefined {
    // widen the extent by each item's dates, an open item by its one date
    let extent: TimeScale | undefined;
    for (const item of items) {
        const start = item.start ?? item.end;
        const end = item.end ?? item.start;
        if (start === undefined || end === undefined) {
            continue;
        }
        extent =
            extent === undefined
                ? { start, end }
                : { start: Math.min(extent.start, start), end: Math.max(extent.end, end) };
    }

    return extent;
}

/** Span the items with a margin, at a least length, or the current hour without items. */
function spanOf(items: readonly TimelineItem[], now: number, least: number): TimeScale {
    // show an empty timeline's current hour
    const extent = extentOf(items);
    if (extent === undefined) {
        const hour = Math.floor(now / HOUR) * HOUR;

        return { start: hour, end: hour + HOUR };
    }

    // widen an extent shorter than the least length around its middle, then pad it
    const length = Math.max(extent.end - extent.start, least);
    const middle = (extent.start + extent.end) / 2;
    const margin = length * MARGIN;

    return { start: middle - length / 2 - margin, end: middle + length / 2 + margin };
}
