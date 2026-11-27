import { state } from "@destack/theme/state";
import { type Axis, ORIENTATION } from "../position/index.ts";
import { Icon } from "@destack/ui/icon";
import { t } from "@destack/locale";
import * as style from "@destack/style";
import { color, motion, radius, space, stroke, swatch } from "@destack/theme/tokens.stylex";
import {
    type Accessor,
    createContext,
    createMemo,
    createSignal,
    createUniqueId,
    type JSX,
    omit,
    onCleanup,
    Show,
    untrack,
    useContext,
    useLocale,
} from "@destack/view";
import {
    type DragItem,
    DragAnnouncer,
    Draggable,
    type DragManager,
    type DragOperation,
    DragPreview,
    type DragPlace,
    type DragStep,
    DragTarget,
    Droppable,
    type DroppableOptions,
    useDragManager,
} from "../drag/index.ts";
import { startViewTransition } from "@destack/view/motion";
import { present } from "@destack/schema";
import {
    contentOr,
    type ElementPartProperties,
    mergeProperties,
    renderPart,
} from "../part/index.ts";
import { type VirtualHost, VirtualHostContext, type VirtualWindow } from "../virtual-list/index.ts";
import { CollectionBuilder, inDocumentOrder } from "../collection/index.ts";
import {
    type ComponentProperties,
    defineComponent,
    type RenderProperties,
} from "@destack/view/declare";

/** The distance in pixels a pointer moves past an item's start to nest under it in a tree. */
const NEST_DISTANCE = 32;

/** The room items make while nothing is dragged. */
const STILL: ReadonlyMap<string, number> = new Map();

/** The line a target item shows along the edge the dragged item lands on. */
const LINE = `calc(2 * ${stroke.border})`;

/** The type of an item and the type a container takes when neither sets one. */
const DEFAULT_TYPE = "item";

/** The styles of a sortable and its parts. */
const styles = style.create({
    item: {
        position: "relative",
        transitionProperty: "opacity",
        transitionDuration: motion.durationShort,
        transitionTimingFunction: motion.easingStandard,
    },
    dragging: {
        opacity: 0.5,
        zIndex: 1,
    },
    // leave a gap where the dragged item lands
    gone: {
        opacity: 0,
    },
    shifting: {
        transitionProperty: "translate, opacity",
        transitionDuration: motion.durationShort,
        transitionTimingFunction: motion.easingStandard,
    },
    room: (inline: string, block: string, backward: string) => ({
        translate: { default: `${inline} ${block}`, ":dir(rtl)": `${backward} ${block}` },
    }),
    lifted: {
        userSelect: "none",
    },
    marked: {
        "::after": {
            content: '""',
            position: "absolute",
            borderRadius: radius.full,
            backgroundColor: swatch.accentMark,
            pointerEvents: "none",
        },
    },
    before: {
        "::after": { insetInline: 0, insetBlockStart: 0, height: LINE },
    },
    after: {
        "::after": { insetInline: 0, insetBlockEnd: 0, height: LINE },
    },
    beforeInline: {
        "::after": { insetBlock: 0, insetInlineStart: 0, width: LINE },
    },
    afterInline: {
        "::after": { insetBlock: 0, insetInlineEnd: 0, width: LINE },
    },
    inside: {
        "::after": { inset: 0, opacity: 0.16, borderRadius: "inherit" },
    },
    empty: {
        "::before": {
            content: '""',
            display: "block",
            flexShrink: 0,
            borderRadius: radius.full,
            backgroundColor: swatch.accentMark,
            pointerEvents: "none",
        },
    },
    emptyBlock: {
        "::before": { height: LINE },
    },
    emptyInline: {
        "::before": { width: LINE, alignSelf: "stretch" },
    },
    handle: {
        display: "inline-flex",
        alignItems: "center",
        justifyContent: "center",
        padding: space[1],
        borderRadius: radius[1],
        color: color.mutedForeground,
        cursor: { default: "grab", ":is(:disabled, [aria-disabled=true])": "not-allowed" },
    },
    grabbing: {
        cursor: "grabbing",
    },
});

/** A move a person made: the item, the container it lands in and the items it lands between. */
export interface SortableMove {
    /** The id of the moved item. */
    readonly id: string;
    /** The type of the moved item, another view's type for an item dropped in from there. */
    readonly type: string;
    /** The id of the container it lands in, an item's id when it nests under that item. */
    readonly container: string;
    /** The id of the item before it, undefined at the start. */
    readonly previous: string | undefined;
    /** The id of the item after it, undefined at the end. */
    readonly next: string | undefined;
    /** What it brings from outside the page, such as files, nothing of an item of the page. */
    readonly items: readonly DragItem[];
}

/** Apply a move to the owner's lists: take the item out and insert it before its next neighbour. */
export const SortableMove = {
    /** Move an item between keyed lists, before its next neighbour in the list it lands in. */
    apply<Item>(
        lists: Readonly<Record<string, readonly Item[]>>,
        move: Pick<SortableMove, "id" | "container" | "next">,
        idOf: (item: Item) => string,
    ): Record<string, readonly Item[]> {
        // take the item out of its list
        const item = Object.values(lists)
            .flat()
            .find((each) => idOf(each) === move.id);
        if (item === undefined) {
            throw new RangeError(`sortable item '${move.id}' not found`);
        }
        const rest = Object.fromEntries(
            Object.entries(lists).map(([id, items]) => [id, items.filter((each) => each !== item)]),
        );

        // place it in the list it lands in
        const target = rest[move.container];
        if (target === undefined) {
            throw new RangeError(`sortable list '${move.container}' not found`);
        }

        return { ...rest, [move.container]: SortableMove.place(target, item, move, idOf) };
    },

    /** Place an item in a list before the move's next neighbour, at the end without one. */
    place<Item>(
        items: readonly Item[],
        item: Item,
        move: Pick<SortableMove, "id" | "next">,
        idOf: (item: Item) => string,
    ): Item[] {
        // take the item out, then insert it before its neighbour
        const others = items.filter((each) => idOf(each) !== move.id);
        const at =
            move.next === undefined
                ? others.length
                : others.findIndex((each) => idOf(each) === move.next);
        if (at === -1) {
            throw new RangeError(`sortable neighbour '${String(move.next)}' not found`);
        }

        return others.toSpliced(at, 0, item);
    },
};

/** The side of an item a dragged item lands on, or inside an item without items of its own. */
type SortableDrop = "before" | "after" | "inside";

/** An item of a sortable. */
interface SortableEntry {
    /** The item's id. */
    readonly id: string;
    /** The id of its container. */
    readonly container: string;
    /** The item's element. */
    readonly element: () => HTMLElement | undefined;
}

/** An item's bounds and its place among its container's items. */
interface SortableBounds {
    /** The item's id. */
    readonly id: string;
    /** Its place among the container's items, those out of view included. */
    readonly position: number;
    /** Its bounds. */
    readonly bounds: DOMRect;
}

/** A container of a sortable's items. */
interface SortableList {
    /** The container's id, an item's id for its children. */
    readonly id: string;
    /** The id of the item it nests in. */
    readonly parent: string | undefined;
    /** The element around the items. */
    readonly element: () => HTMLElement | undefined;
    /** The direction its items lay out along. */
    readonly orientation: () => Axis;
    /** The name announcements call it by. */
    readonly label: () => string;
    /** The types of items it takes. */
    readonly accepts: () => readonly string[];
    /** Read how high it floats over the others. */
    readonly height: () => number;
    /** Report whether it takes a given item. */
    readonly allows: (id: string) => boolean;
    /** The lists a keyboard step crosses between. */
    readonly group: () => string;
    /** Whether items land only at its end, as in a folded list whose items do not show. */
    readonly isEndOnly: () => boolean;
    /** The virtual list inside, whose items mount only in view, none for mounted items. */
    readonly window: Accessor<VirtualWindow | undefined>;
}

/** The items of a sortable and the drags that move them. */
export class SortableControl {
    /** The drags of the sortable. */
    readonly manager: DragManager;
    /** The root's properties, read for nesting. */
    readonly properties: SortableProperties;
    /** The items by id. */
    readonly #items = new Map<string, SortableEntry>();
    /** The one item the drag marks and the side it lands on, found once per step of the drag. */
    readonly #marked: Accessor<{ readonly id: string; readonly drop: SortableDrop } | undefined>;
    /** How far each item makes room for the dragged one, found once per step of the drag. */
    readonly #rooms: Accessor<ReadonlyMap<string, number>>;
    /** The sortable's element, which its transitions run in. */
    #element: HTMLElement | undefined;
    /** The name a dropped item and its preview share as the item lands. */
    readonly #name = `sortable-${createUniqueId().replaceAll(/[^\w-]/gu, "")}`;
    /** The containers by their ids. */
    readonly #lists = new Map<string, SortableList>();
    /** The bounds of each container's items as the drag lifted, none for a virtual list's. */
    readonly #measured = new Map<string, readonly SortableBounds[]>();
    /** The length the dragged item takes in each container, its gap to the next included. */
    readonly #lengths = new Map<string, number>();
    /** Count each item that mounts or unmounts, which the marked item follows. */
    readonly #mounts: CollectionBuilder<string>;

    /** Create the state of a sortable from its root's properties. */
    constructor(properties: SortableProperties, manager: DragManager) {
        // animate every move the drags drop on the lists
        this.properties = properties;
        this.manager = manager;
        this.#mounts = new CollectionBuilder();

        // mark the item a drag lands beside once per target, and again as items mount
        const target = createMemo(() => manager.operation()?.target, {
            equals: (previous, next) =>
                previous === undefined || next === undefined
                    ? previous === next
                    : DragTarget.isSame(previous, next),
        });
        this.#marked = createMemo(
            () => {
                this.#mounts.items();

                return this.#markedOf(target());
            },
            {
                equals: (previous, next) =>
                    previous?.id === next?.id && previous?.drop === next?.drop,
            },
        );

        // make room once per target, and again as items mount
        this.#rooms = createMemo(() => {
            // make none without a drag, and with a line
            const aim = target();
            this.#mounts.items();
            if (manager.item() === undefined || this.properties.indicator !== "gap") {
                return STILL;
            }

            return this.#roomsAt(aim);
        });
    }

    /** Run a move as a view transition, each item morphing to where the move puts it. */
    animate(move: () => void): void {
        // land the dragged item from its preview under the pointer, sharing one name
        const root = present(this.#element, "a sortable's element");
        const dragged = this.manager.current()?.item;
        const landing = dragged === undefined ? undefined : this.elementOf(dragged);
        const preview = root.querySelector<HTMLElement>("[data-slot=drag-preview]");
        const isLanding = landing !== undefined && preview !== null;
        if (isLanding) {
            preview.style.viewTransitionName = this.#name;
        }

        // morph every other item to where the move puts it
        const items = [...this.#items.values()]
            .flatMap((entry) => entry.element() ?? [])
            .filter((element) => !isLanding || element !== landing);
        void startViewTransition(
            root,
            () => {
                move();
                if (isLanding) {
                    landing.style.viewTransitionName = this.#name;
                }
            },
            items,
        ).then(() => {
            if (isLanding) {
                landing.style.viewTransitionName = "";
            }
        });
    }

    /** Attach the sortable's element, which its transitions run in. */
    setElement(element: HTMLElement): void {
        this.#element = element;
    }

    /** Read the id of the dragged item, none between drags. */
    dragged(): string | undefined {
        return this.manager.current()?.item;
    }

    /** Add an item until it unmounts, refusing an item its virtual list lacks. */
    addItem(entry: SortableEntry): void {
        // refuse an item whose id is none of its virtual list's keys
        const window = untrack(() => this.#lists.get(entry.container)?.window());
        if (window !== undefined && untrack(() => !window.positions().has(entry.id))) {
            throw new TypeError(`sortable item '${entry.id}' is not among its virtual list's keys`);
        }

        // add the item, counting its mount and its unmount
        this.#items.set(entry.id, entry);
        this.#mounts.add(entry.id);
        onCleanup(() => {
            if (this.#items.get(entry.id) === entry) {
                this.#items.delete(entry.id);
            }
        });
    }

    /** Add a container's droppable to the drags until it unmounts. */
    addList(list: SortableList): void {
        // add the list by its id, and its droppable
        this.#lists.set(list.id, list);
        onCleanup(() => {
            if (this.#lists.get(list.id) === list) {
                this.#lists.delete(list.id);
            }
        });
        void new Droppable(this.#droppableOf(list), this.manager);
    }

    /** Read the id of the item an element stands in, none outside every item. */
    idOf(target: EventTarget | null): string | undefined {
        const element =
            target instanceof Element ? target.closest("[data-slot=sortable-item]") : null;

        return [...this.#items.values()].find((entry) => entry.element() === element)?.id;
    }

    /** Read an item's element. */
    elementOf(id: string): HTMLElement | undefined {
        return this.#items.get(id)?.element();
    }

    /** List a container's item ids in order, all of a virtual list's, without the dragged one. */
    idsOf(container: string): readonly string[] {
        // read a virtual list's keys without the dragged one
        const window = this.#lists.get(container)?.window();
        const lifted = window === undefined ? undefined : this.#liftedIn(window);
        if (window !== undefined) {
            return lifted === undefined ? window.keys() : window.keys().toSpliced(lifted, 1);
        }

        // else read the mounted items in document order
        const dragged = this.dragged();

        return this.#entriesOf(container).flatMap(({ id }) => (id === dragged ? [] : [id]));
    }

    /** List a container's mounted items in document order. */
    #entriesOf(container: string): readonly SortableEntry[] {
        return inDocumentOrder(
            [...this.#items.values()].filter((entry) => entry.container === container),
            (entry) => entry.element(),
        );
    }

    /** Read the id at a place among a container's items without the dragged one. */
    idAt(container: string, index: number): string | undefined {
        // read a virtual list's key past the dragged one, which leaves no copy of its keys
        const window = this.#lists.get(container)?.window();
        const lifted = window === undefined ? undefined : this.#liftedIn(window);
        if (window !== undefined) {
            return index < 0
                ? undefined
                : window.keys()[lifted !== undefined && index >= lifted ? index + 1 : index];
        }

        return this.idsOf(container)[index];
    }

    /** Count a container's items without the dragged one. */
    countOf(container: string): number {
        const window = this.#lists.get(container)?.window();

        return window === undefined
            ? this.idsOf(container).length
            : window.keys().length - (this.#liftedIn(window) === undefined ? 0 : 1);
    }

    /** Read the target of the item that stands at a place in its container. */
    homeOf(id: string): DragTarget {
        // find the item's container, refusing an item that is not mounted
        const entry = this.#items.get(id);
        if (entry === undefined) {
            throw new TypeError(`sortable item '${id}' is not mounted`);
        }

        // read its place among a virtual list's keys, refusing an id the list lacks
        const window = this.#lists.get(entry.container)?.window();
        const position = window?.positions().get(id);
        if (window !== undefined && position === undefined) {
            throw new TypeError(`sortable item '${id}' is not among its virtual list's keys`);
        }

        // else read its place among its siblings in document order
        const index = position ?? this.#entriesOf(entry.container).indexOf(entry);

        return { kind: "list", droppable: entry.container, index };
    }

    /** Read how far an item makes room for the dragged one along its container, in pixels. */
    roomOf(id: string): number {
        return this.#rooms().get(id) ?? 0;
    }

    /** Read how far each item makes room at a target, none with a line. */
    #roomsAt(target: DragTarget | undefined): ReadonlyMap<string, number> {
        // add up what each item makes
        const from = this.manager.current()?.from;
        const rooms = new Map<string, number>();

        // take the room at the place it lands, and close up where it stood
        for (const at of [from, target]) {
            if (at?.kind === "list") {
                const sign = at === target ? 1 : -1;
                const length = this.#lengths.get(at.droppable) ?? 0;
                for (const id of this.idsOf(at.droppable).slice(at.index)) {
                    rooms.set(id, (rooms.get(id) ?? 0) + sign * length);
                }
            }
        }

        return rooms;
    }

    /** Read the side of an item the dragged item lands on. */
    dropOf(id: string): SortableDrop | undefined {
        // mark nothing where the others make room
        if (this.properties.indicator === "gap") {
            return undefined;
        }
        const marked = this.#marked();

        return marked?.id === id ? marked.drop : undefined;
    }

    /** Find the one item a drag at a target marks and the side it lands on. */
    #markedOf(
        target: DragTarget | undefined,
    ): { readonly id: string; readonly drop: SortableDrop } | undefined {
        // mark nothing without a list target
        if (target?.kind !== "list") {
            return undefined;
        }

        // mark the item the target nests in while it has no items
        const next = this.idAt(target.droppable, target.index);
        const previous = this.idAt(target.droppable, target.index - 1);
        if (this.countOf(target.droppable) === 0) {
            return { id: target.droppable, drop: "inside" };
        }
        // mark the item after the place, else the one before where the one after is not mounted
        else if (
            next !== undefined &&
            (previous === undefined || this.elementOf(next) !== undefined)
        ) {
            return { id: next, drop: "before" };
        }

        return previous === undefined ? undefined : { id: previous, drop: "after" };
    }

    /** Report a drop as a move between the items it lands between. */
    moveOf(operation: DragOperation & { readonly target: DragTarget }): SortableMove {
        // read the list it lands in and its neighbours there
        const target = operation.target;
        if (target.kind !== "list") {
            throw new TypeError(`sortable drop outside a list: ${target.kind}`);
        }

        return {
            id: operation.item,
            type: operation.type,
            container: target.droppable,
            previous: this.idAt(target.droppable, target.index - 1),
            next: this.idAt(target.droppable, target.index),
            items: operation.items,
        };
    }

    /** Build the droppable of a container. */
    #droppableOf(list: SortableList): DroppableOptions {
        return {
            id: list.id,
            group: untrack(list.group),
            axis: untrack(list.orientation) === "vertical" ? "block" : "inline",
            element: list.element,
            accepts: (draggable) =>
                list.accepts().includes(draggable.type) && list.allows(draggable.id),
            measure: () => this.#measure(list),
            targetAt: (x, y) => this.#targetAt(list, x, y),
            step: (target, step) => this.#step(list, target, step),
            enter: (from) => ({
                kind: "list",
                droppable: list.id,
                index: list.isEndOnly()
                    ? this.countOf(list.id)
                    : Math.min(from.kind === "list" ? from.index : 0, this.countOf(list.id)),
            }),
            describe: (target) => this.#describe(list, target),
            drop: (operation) => {
                const move = this.moveOf(operation);
                this.animate(() => this.properties.onMove?.(move));
            },
            height: list.height,
            scroller: () => list.window()?.scroller(),
            reveal: (target, item) => this.#reveal(list, target, item),
        };
    }

    /** Read the bounds of a container's items as a drag lifts, a virtual list's at each move. */
    #measure(list: SortableList): void {
        // read the items' bounds, and the room the dragged one takes among them
        this.#measured.set(list.id, list.window() === undefined ? this.#boundsOf(list) : []);
        const dragged = this.dragged();
        const first = this.#entriesOf(list.id)[0]?.element();
        const element = (dragged === undefined ? undefined : this.elementOf(dragged)) ?? first;
        const row = first?.parentElement ?? undefined;
        const isVertical = untrack(list.orientation) === "vertical";
        const gap =
            row === undefined ? "" : getComputedStyle(row)[isVertical ? "rowGap" : "columnGap"];
        const length = isVertical ? element?.offsetHeight : element?.offsetWidth;
        this.#lengths.set(list.id, (length ?? 0) + (Number.parseFloat(gap) || 0));
    }

    /** Find the place a point lands at among a container's items. */
    #targetAt(list: SortableList, x: number, y: number): DragTarget {
        // find the first mounted item past the pointer along the container's direction
        const isVertical = list.orientation() === "vertical";
        const items =
            list.window() === undefined
                ? (this.#measured.get(list.id) ?? [])
                : this.#boundsOf(list);
        const isBackward = this.manager.locale.direction === "rtl";
        const found = items.findIndex(({ bounds }) => {
            // compare with its middle in the direction of the text
            const middle = bounds.left + bounds.width / 2;

            return isVertical
                ? y < bounds.top + bounds.height / 2
                : isBackward
                  ? x > middle
                  : x < middle;
        });
        const index = found === -1 || list.isEndOnly() ? items.length : found;

        // nest under the item before when the pointer moves past its start in a tree
        const previous = items[index - 1];
        if (
            this.properties.nesting === true &&
            previous !== undefined &&
            x > previous.bounds.left + NEST_DISTANCE
        ) {
            return { kind: "list", droppable: previous.id, index: this.countOf(previous.id) };
        }

        return { kind: "list", droppable: list.id, index: this.#placeOf(list, items, index) };
    }

    /** Read the bounds of a container's mounted items in order, leaving out the dragged one. */
    #boundsOf(list: SortableList): readonly SortableBounds[] {
        // read a virtual list's mounted items by their places in its order, past the dragged one
        const window = list.window();
        const dragged = this.dragged();
        if (window !== undefined) {
            const positions = window.positions();
            const lifted = this.#liftedIn(window);

            return [...this.#items.values()]
                .flatMap((entry) => {
                    // place the item among the others, one earlier past the dragged one
                    const at = entry.id === dragged ? undefined : positions.get(entry.id);
                    const position =
                        at === undefined || lifted === undefined || at < lifted ? at : at - 1;
                    const element = entry.element();

                    // leave out items of other lists and items not mounted
                    return entry.container !== list.id ||
                        position === undefined ||
                        element === undefined
                        ? []
                        : [{ id: entry.id, position, bounds: element.getBoundingClientRect() }];
                })
                .toSorted((left, right) => left.position - right.position);
        }

        // else read every item, all of them mounted
        return this.idsOf(list.id).flatMap((id, position) => {
            const element = this.elementOf(id);

            return element === undefined
                ? []
                : [{ id, position, bounds: element.getBoundingClientRect() }];
        });
    }

    /** Read a place among a container's mounted items as its place among every item. */
    #placeOf(list: SortableList, items: readonly SortableBounds[], index: number): number {
        // land at the end of a list that takes items only there, or that shows none
        const last = items.at(-1);
        const at = items[index];
        if (list.isEndOnly() || last === undefined) {
            return this.countOf(list.id);
        }
        // land before the mounted item at the place
        else if (at !== undefined) {
            return at.position;
        }

        return last.position + 1;
    }

    /** Step a target along the container, or in and out of nesting across it in a tree. */
    #step(list: SortableList, target: DragTarget, step: DragStep): DragTarget | undefined {
        // stay at the end of a list that takes items only there
        if (list.isEndOnly()) {
            return undefined;
        }

        // step along the container's own axis within its ends
        const index = target.kind === "list" ? target.index : 0;
        const count = this.countOf(list.id);
        const isVertical = list.orientation() === "vertical";
        const forward = isVertical ? "block-end" : "inline-end";
        const backward = isVertical ? "block-start" : "inline-start";
        if (step === forward) {
            return index < count
                ? { kind: "list", droppable: list.id, index: index + 1 }
                : undefined;
        }
        // step back along the axis
        else if (step === backward) {
            return index > 0 ? { kind: "list", droppable: list.id, index: index - 1 } : undefined;
        }
        // nest under the item before
        else if (this.properties.nesting === true && step === "inline-end") {
            const previous = this.idAt(list.id, index - 1);

            return previous === undefined
                ? undefined
                : { kind: "list", droppable: previous, index: this.countOf(previous) };
        }
        // move out after the item it nests in
        else if (this.properties.nesting === true && step === "inline-start") {
            const outer = list.parent === undefined ? undefined : this.#items.get(list.parent);

            return outer === undefined
                ? undefined
                : {
                      kind: "list",
                      droppable: outer.container,
                      index: this.idsOf(outer.container).indexOf(outer.id) + 1,
                  };
        }

        return undefined;
    }

    /** Bring a target into view: the item back at its place, else the item beside the place. */
    #reveal(list: SortableList, target: DragTarget, item: string): void {
        // read the item back at its place, else the item after the place or the one before it
        const from = this.manager.current()?.from;
        const isHome = from !== undefined && DragTarget.isSame(target, from);
        const index = target.kind === "list" ? target.index : 0;
        const key = isHome ? item : (this.idAt(list.id, index) ?? this.idAt(list.id, index - 1));
        if (key === undefined) {
            return;
        }

        // scroll a virtual list to its key, else the mounted element into view
        const window = list.window();
        if (window !== undefined) {
            window.reveal(key);
        } else {
            this.elementOf(key)?.scrollIntoView({ block: "nearest", inline: "nearest" });
        }
    }

    /** Read the dragged item's place among a virtual list's keys, none where it is not there. */
    #liftedIn(window: VirtualWindow): number | undefined {
        const dragged = this.dragged();

        return dragged === undefined ? undefined : window.positions().get(dragged);
    }

    /** Describe a place in a container as its position among the items. */
    #describe(list: SortableList, target: DragTarget): DragPlace {
        return {
            kind: "position",
            position: (target.kind === "list" ? target.index : 0) + 1,
            count: this.countOf(list.id) + 1,
            list: list.label(),
        };
    }
}

/** The nearest sortable, null outside one. */
const SortableContext = createContext<SortableControl | null>(null);

/** The id and direction of the nearest container, null outside one. */
const SortableContainerContext = createContext<{
    readonly id: string;
    readonly orientation: () => Axis;
} | null>(null);

/** The nearest item, null outside one. */
const SortableItemContext = createContext<SortableItemControl | null>(null);

/** The id and state of an item, which its handle and nested containers read. */
interface SortableItemControl {
    /** The item's id. */
    readonly id: string;
    /** The item as its handle drags it. */
    readonly draggable: Draggable;
    /** Whether the item ignores the person. */
    readonly isDisabled: () => boolean;
    /** The ways a person lifts and moves the item. */
    readonly sensors: readonly SortableSensor[];
    /** Set the element of the item's handle. */
    readonly setHandle: (element: HTMLElement) => void;
    /** Set the element around the item's children in a tree. */
    readonly setChildren: (element: HTMLElement) => void;
}

/** The properties of an element of a sortable, the native element's attributes included. */
export type SortableElementProperties<Attributes> = Omit<Attributes, "class" | "style"> &
    ElementPartProperties;

/** How a drag shows where it lands: a line, or a gap the others make. */
export type SortableIndicator = "line" | "gap";

/** The properties of a sortable, the native element's attributes included. */
export interface SortableProperties extends SortableElementProperties<
    JSX.HTMLAttributes<HTMLDivElement>
> {
    /** Handle a person dropping an item at another place, which the owner renders the items in. */
    readonly onMove?: (move: SortableMove) => void;
    /** Whether items nest under one another as a tree. */
    readonly nesting?: boolean;
    /** How a drag shows where it lands: a line, or the others making room, a line by default. */
    readonly indicator?: SortableIndicator;
}

/** The properties people change in a sortable container's controls and inspector. */
const SORTABLE_CONTAINER_PROPERTIES = {
    orientation: ORIENTATION.described(t`Whether its items lie in a column or a row.`).default(
        "vertical",
    ),
};

/** The properties of a sortable container besides its declared ones: its id and name. */
export interface SortableContainerAttributes extends SortableElementProperties<
    JSX.HTMLAttributes<HTMLDivElement>
> {
    /** The container's id. */
    readonly id: string;
    /** The name announcements call it by, a generic one by default. */
    readonly label?: string;
    /** The types of items it takes. */
    readonly accepts?: readonly string[];
    /** How high it floats over the others, zero by default. */
    readonly height?: number;
    /** Report whether it takes a given item beside its types. */
    readonly allows?: (id: string) => boolean;
    /** The lists a keyboard step crosses between, those of the same accepted types by default. */
    readonly group?: string;
    /** Whether items land only at its end, as in a folded list whose items do not show. */
    readonly isEndOnly?: boolean;
    /** Map the container's element to the element drags aim at to land in it. */
    readonly target?: (element: HTMLElement) => HTMLElement;
}

/** The element that lifts a sortable item: its handle, or the whole item. */
export type SortableActivator = "handle" | "item";

/** A way a person lifts and moves a sortable item: a pointer or touch, or the keys. */
export type SortableSensor = "pointer" | "keyboard";

/** The sensors of an item that sets none: every one. */
const SENSORS: readonly SortableSensor[] = ["pointer", "keyboard"];

/** The properties of a sortable's item, the native element's attributes included. */
export interface SortableItemProperties extends SortableElementProperties<
    JSX.HTMLAttributes<HTMLDivElement>
> {
    /** The item's id, which a move reports. */
    readonly id: string;
    /** The name announcements call it by, its text by default. */
    readonly label?: string;
    /** Whether the item stays where it is. */
    readonly disabled?: boolean;
    /** The type of item. */
    readonly type?: string;
    /** The element that lifts the item, its handle by default. */
    readonly activator?: SortableActivator;
    /** The ways a person lifts and moves the item, the pointer and the keys by default. */
    readonly sensors?: readonly SortableSensor[];
}

/** The properties of an item's handle, the native button's attributes included. */
export type SortableHandleProperties = SortableElementProperties<
    Omit<
        JSX.ButtonHTMLAttributes<HTMLButtonElement>,
        | "onKeyDown"
        | "onPointerDown"
        | "onPointerMove"
        | "onPointerUp"
        | "onPointerCancel"
        | "onBlur"
    >
>;

/** Render a sortable around its containers, announcing each move in a live region. */
export function Sortable(properties: SortableProperties): JSX.Element {
    // move the items by drags that report each drop as a move
    const drags = useDragManager();
    const control = new SortableControl(properties, drags.manager);
    const manager = control.manager;
    const rest = omit(properties, "onMove", "nesting", "indicator", "children");

    return (
        <SortableContext value={control}>
            {renderPart("div", "sortable", rest, null, {
                ref: (element: HTMLElement) => control.setElement(element),
                get "data-dragging"() {
                    return manager.operation() === undefined ? undefined : "";
                },
                get children() {
                    return (
                        <>
                            {properties.children}
                            <DragPreview
                                manager={manager}
                                element={(id) => control.elementOf(id)}
                            />
                            <Show when={!drags.isShared}>
                                <DragAnnouncer manager={manager} />
                            </Show>
                        </>
                    );
                },
            })}
        </SortableContext>
    );
}

/** Render a container of the nearest sortable. */
export const SortableContainer = defineComponent({
    name: "sortableContainer",
    properties: SORTABLE_CONTAINER_PROPERTIES,
    render(
        properties: RenderProperties<
            SortableContainerAttributes,
            typeof SORTABLE_CONTAINER_PROPERTIES
        >,
    ): JSX.Element {
        // join the sortable under the item the container nests in, if any
        const control = useSortable();
        const locale = useLocale();
        const parent = useContext(SortableItemContext);
        const rest = omit(
            properties,
            "id",
            "orientation",
            "label",
            "accepts",
            "height",
            "group",
            "isEndOnly",
            "allows",
            "target",
        );
        let element: HTMLElement | undefined;

        // read the container's id and the sortable's nesting once
        const id = untrack(() => properties.id);
        const isNesting = untrack(() => control.properties.nesting === true);

        // leave an item's children to its own droppable, or add the container's own
        const isChildren = isNesting && parent?.id === id;

        // take the window of the one virtual list inside
        const outer = useContext(VirtualHostContext);
        const windows = new CollectionBuilder<VirtualWindow>();
        const window = (): VirtualWindow | undefined => windows.items()[0];
        const [focused, setFocused] = createSignal<string | undefined>(undefined, {
            ownedWrite: true,
        });
        const host: VirtualHost = {
            add: (each) => {
                // take the window, refusing one in an item's children, which reads no window
                if (isChildren) {
                    throw new TypeError(`sortable children of '${id}' are invalid: a virtual list`);
                }
                // refuse a second window
                else if (untrack(window) !== undefined) {
                    throw new TypeError(`sortable container '${id}' has several virtual lists`);
                }
                windows.add(each);
                outer?.add(each);
            },
            persisted: () => {
                // mount the dragged and the focused item, and those the list around asks for
                const dragged = control.manager.item();
                const each = focused();

                return [
                    ...(dragged === undefined ? [] : [dragged]),
                    ...(each === undefined ? [] : [each]),
                    ...(outer?.persisted() ?? []),
                ];
            },
        };

        // add the container's droppable, leaving an item's children to the item's
        if (!isChildren) {
            control.addList({
                id,
                parent: parent?.id,
                element: () =>
                    element !== undefined && properties.target !== undefined
                        ? properties.target(element)
                        : element,
                orientation: () => properties.orientation,
                label: () => properties.label ?? locale.render(t.context("title")`the list`),
                accepts: () => properties.accepts ?? [DEFAULT_TYPE],
                height: () => properties.height ?? 0,
                allows: (item) => properties.allows?.(item) !== false,
                group: () => properties.group ?? (properties.accepts ?? [DEFAULT_TYPE]).join(" "),
                isEndOnly: () => properties.isEndOnly === true,
                window,
            });
        }

        // mark the empty container a drag lands in, its line at its start, as items mark theirs
        const isEmptyTarget = () => !isChildren && control.dropOf(id) === "inside";

        // mark the container a drag aims at, and one refusing the dragged item
        const stateOf = (): "over" | "refused" | "idle" => {
            const dragged = control.manager.operation()?.item;
            if (control.manager.targetIn(id) !== undefined) {
                return "over";
            } else if (dragged !== undefined && properties.allows?.(dragged) === false) {
                return "refused";
            }

            return "idle";
        };

        return (
            <SortableContainerContext value={{ id, orientation: () => properties.orientation }}>
                <VirtualHostContext value={host}>
                    {renderPart(
                        "div",
                        "sortable-container",
                        rest,
                        () =>
                            isEmptyTarget() && [
                                styles.empty,
                                properties.orientation === "horizontal"
                                    ? styles.emptyInline
                                    : styles.emptyBlock,
                            ],
                        {
                            get "data-over"() {
                                return stateOf() === "over" ? "" : undefined;
                            },
                            get "data-drop"() {
                                return isEmptyTarget() ? "inside" : undefined;
                            },
                            get "data-refused"() {
                                return stateOf() === "refused" ? "" : undefined;
                            },
                            get "data-orientation"() {
                                return properties.orientation;
                            },
                            onFocusIn: (event: FocusEvent) =>
                                setFocused(control.idOf(event.target)),
                            onFocusOut: (event: FocusEvent) => {
                                // forget the focused item once the focus leaves the container
                                if (
                                    !(event.relatedTarget instanceof Node) ||
                                    element?.contains(event.relatedTarget) !== true
                                ) {
                                    setFocused(undefined);
                                }
                            },
                            ref: (created: HTMLElement) => {
                                element = created;
                                if (isChildren) {
                                    parent.setChildren(created);
                                }
                            },
                        },
                    )}
                </VirtualHostContext>
            </SortableContainerContext>
        );
    },
});

/** The properties of a sortable container, its native attributes included. */
export type SortableContainerProperties = ComponentProperties<typeof SortableContainer>;

/** Render an item of the nearest container. */
export function SortableItem(properties: SortableItemProperties): JSX.Element {
    // join the container, refusing an item outside one
    const control = useSortable();
    const locale = useLocale();
    const container = useContext(SortableContainerContext);
    if (container === null) {
        throw new TypeError("a sortable item needs a sortable container around it");
    }
    const rest = omit(properties, "id", "label", "disabled", "type", "activator", "sensors");
    let element: HTMLElement | undefined;
    let handle: HTMLElement | undefined;
    let children: HTMLElement | undefined;

    // read the item's fixed properties once
    const id = untrack(() => properties.id);
    const type = untrack(() => properties.type ?? DEFAULT_TYPE);
    const activator = untrack(() => properties.activator ?? "handle");
    const sensors = untrack(() => properties.sensors ?? SENSORS);
    const isLiftedWhole = activator === "item";
    const isNesting = untrack(() => control.properties.nesting === true);

    // add the item and the handle that takes the focus back
    control.addItem({ id, container: container.id, element: () => element });
    const draggable = new Draggable(
        {
            id,
            type,
            kind: "move",
            label: () => properties.label ?? element?.textContent?.trim() ?? id,
            element: () => (isLiftedWhole ? element : handle),
            home: () => control.homeOf(id),
            isDisabled: () => properties.disabled === true,
        },
        control.manager,
    );

    // take children in a tree, in their container or else inside the item
    if (isNesting) {
        control.addList({
            id,
            parent: id,
            element: () => children ?? element,
            orientation: () => "vertical",
            label: () =>
                properties.label ??
                element?.textContent?.trim() ??
                locale.render(t.context("title")`the item`),
            accepts: () => [type],
            height: () => 0,
            allows: () => true,
            group: () => id,
            isEndOnly: () => false,
            window: () => undefined,
        });
    }

    // lift the whole item by its sensors, leaving a handle's item to its handle
    const lifting = isLiftedWhole
        ? sensedAttributes(draggable, sensors, [" "], { tabindex: 0 })
        : {};

    // follow the drag of this item
    const isDragging = (): boolean => control.manager.isDragging(id);
    const isVertical = (): boolean => container.orientation() === "vertical";

    return (
        <SortableItemContext
            value={{
                id,
                draggable,
                isDisabled: () => properties.disabled === true,
                sensors,
                setHandle: (created) => (handle = created),
                setChildren: (created) => (children = created),
            }}
        >
            <VirtualHostContext value={null}>
                {renderPart(
                    "div",
                    "sortable-item",
                    rest,
                    () => {
                        // make room, or mark the side it lands on
                        const room = control.roomOf(id);
                        const along = `${String(room)}px`;
                        const isGap = control.properties.indicator === "gap";

                        return [
                            styles.item,
                            isLiftedWhole && styles.lifted,
                            isDragging() && (isGap ? styles.gone : styles.dragging),
                            isGap &&
                                control.manager.item() !== undefined && [
                                    styles.shifting,
                                    isVertical()
                                        ? styles.room("0px", along, "0px")
                                        : styles.room(along, "0px", `${String(-room)}px`),
                                ],
                            dropStyle(control.dropOf(id), isVertical()),
                        ];
                    },
                    mergeProperties(lifting, {
                        get "data-dragging"() {
                            return isDragging() ? "" : undefined;
                        },
                        get "data-drop"() {
                            return control.dropOf(id);
                        },
                        get "data-disabled"() {
                            return properties.disabled === true ? "" : undefined;
                        },
                        ref: (created: HTMLElement) => (element = created),
                    }),
                )}
            </VirtualHostContext>
        </SortableItemContext>
    );
}

/** Render the button that lifts the nearest item. */
export function SortableHandle(properties: SortableHandleProperties): JSX.Element {
    // read the sortable and the item the handle moves
    const control = useSortable();
    const locale = useLocale();
    const item = useContext(SortableItemContext);
    if (item === null) {
        throw new TypeError("a sortable handle needs a sortable item around it");
    }
    const rest = omit(properties, "xstyle", "style", "children");
    const isDragging = (): boolean => control.manager.isDragging(item.id);

    // drag the item from the handle by pointer, touch and keys
    const handle = mergeProperties(
        {
            type: "button" as const,
            "aria-label": locale.render(t.context("action")`Move`),
            "data-slot": "sortable-handle",
            get "aria-pressed"(): "true" | "false" {
                return isDragging() ? "true" : "false";
            },
            get "data-dragging"() {
                return isDragging() ? "" : undefined;
            },
        },
        sensedAttributes(item.draggable, item.sensors, [" ", "Enter"], {}),
    );

    return (
        <button
            {...handle}
            aria-disabled={item.isDisabled() ? "true" : undefined}
            {...rest}
            ref={(created) => item.setHandle(created)}
            {...style.attributes(
                [
                    state.focus,
                    state.touch,
                    state.disabled,
                    styles.handle,
                    isDragging() && styles.grabbing,
                    properties.xstyle,
                ],
                properties.style,
            )}
        >
            {contentOr(properties, () => (
                <Icon name="dots-six-vertical" />
            ))}
        </button>
    );
}

/** Read the nearest sortable, refusing a part outside one. */
export function useSortable(): SortableControl {
    const control = useContext(SortableContext);
    if (control === null) {
        throw new TypeError("a sortable part needs a sortable around it");
    }

    return control;
}

/** Return the styles that mark where a dragged item lands beside an item. */
export function dropStyle(drop: SortableDrop | undefined, isVertical: boolean): style.Styles {
    if (drop === "inside") {
        return [styles.marked, styles.inside];
    } else if (drop === "before") {
        return [styles.marked, isVertical ? styles.before : styles.beforeInline];
    } else if (drop === "after") {
        return [styles.marked, isVertical ? styles.after : styles.afterInline];
    }

    return false;
}

/** Read the attributes that lift a draggable by its sensors, the keys only with the keyboard. */
function sensedAttributes(
    draggable: Draggable,
    sensors: readonly SortableSensor[],
    keys: readonly string[],
    focusable: { readonly tabindex?: number },
) {
    return sensors.includes("keyboard")
        ? mergeProperties(focusable, draggable.attributes(keys))
        : draggable.pointerAttributes();
}
