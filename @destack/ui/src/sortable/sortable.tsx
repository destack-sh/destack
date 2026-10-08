import { Icon } from "@destack/icon";
import { type Direction, type Localization, type Message, t } from "@destack/locale";
import * as style from "@destack/style";
import { color, motion, radius, space, stroke } from "@destack/theme/tokens.stylex";
import {
    type Accessor,
    createContext,
    createMemo,
    createSignal,
    createUniqueId,
    type JSX,
    omit,
    onCleanup,
    type Setter,
    useContext,
    useLocale,
} from "@destack/view";
import { visuallyHiddenStyle } from "../visually-hidden/index.ts";

/** The distance in pixels a pointer travels before it lifts an item, so a click stays a click. */
const LIFT_DISTANCE = 4;

/** The distance in pixels a pointer moves past an item's start to nest under it in a tree. */
const NEST_DISTANCE = 32;

/** The line a target item shows on the side the dragged item lands on. */
const LINE = `calc(2 * ${stroke.border})`;

/** The styles of a sortable and its parts. */
const styles = style.create({
    item: {
        position: "relative",
        transitionProperty: "opacity, box-shadow",
        transitionDuration: motion.durationShort,
        transitionTimingFunction: motion.easingStandard,
    },
    dragging: {
        opacity: 0.5,
        zIndex: 1,
    },
    lifted: {
        transitionProperty: "none",
    },
    before: { boxShadow: `0 calc(-1 * ${LINE}) 0 0 ${color.primary}` },
    after: { boxShadow: `0 ${LINE} 0 0 ${color.primary}` },
    beforeInline: { boxShadow: `calc(-1 * ${LINE}) 0 0 0 ${color.primary}` },
    afterInline: { boxShadow: `${LINE} 0 0 0 ${color.primary}` },
    inside: { boxShadow: `inset 0 0 0 ${LINE} ${color.primary}` },
    handle: {
        display: "inline-flex",
        alignItems: "center",
        justifyContent: "center",
        padding: space[1],
        borderRadius: radius[1],
        color: color.mutedForeground,
        cursor: { default: "grab", ":disabled": "not-allowed" },
        touchAction: "none",
        outlineStyle: { default: "none", ":focus-visible": "solid" },
        outlineWidth: stroke.ring,
        outlineColor: `color-mix(in oklab, ${color.ring} 50%, transparent)`,
    },
    grabbing: {
        cursor: "grabbing",
    },
});

/** The offset a pointer drag moves the dragged item by. */
const offsets = style.create({
    translate: (x: number, y: number) => ({
        transform: `translate(${String(x)}px, ${String(y)}px)`,
    }),
});

/** The direction a container lays out its items along, which the arrow keys follow. */
export type SortableOrientation = "vertical" | "horizontal";

/** A move a person made: the item, the container it lands in and the items it lands between. */
export interface SortableMove {
    /** The id of the moved item. */
    readonly id: string;
    /** The id of the container it lands in, an item's id when it nests under that item. */
    readonly container: string;
    /** The id of the item before it, undefined at the start. */
    readonly previous: string | undefined;
    /** The id of the item after it, undefined at the end. */
    readonly next: string | undefined;
}

/** The side of an item a dragged item lands on, or inside an item without items of its own. */
type SortableDrop = "before" | "after" | "inside";

/** The place a dragged item would land: a container and its position among the container's other items. */
interface SortableTarget {
    /** The id of the container. */
    readonly container: string;
    /** The position among the container's items without the dragged one. */
    readonly index: number;
}

/** A drag under way. */
interface SortableDrag {
    /** The id of the dragged item. */
    readonly id: string;
    /** Whether the pointer or the keyboard moves it. */
    readonly mode: "pointer" | "keyboard";
    /** Where it started. */
    readonly from: SortableTarget;
    /** Where it would land. */
    readonly target: SortableTarget;
    /** How far the pointer moved it, in pixels. */
    readonly offset: { readonly x: number; readonly y: number };
}

/** An item of a sortable. */
interface SortableEntry {
    /** The item's id. */
    readonly id: string;
    /** The id of its container. */
    readonly container: string;
    /** The item's element. */
    readonly element: () => HTMLElement | undefined;
    /** The element of its handle, which takes the focus back after a drop. */
    readonly handle: () => HTMLElement | undefined;
    /** The name announcements call it by. */
    readonly label: () => string;
}

/** A container of a sortable: a list, a board's column or an item's children in a tree. */
interface SortableContainerEntry {
    /** The container's id. */
    readonly id: string;
    /** The id of the item it nests in, undefined at the top. */
    readonly parent: string | undefined;
    /** The container's element. */
    readonly element: () => HTMLElement | undefined;
    /** The direction its items lay out along. */
    readonly orientation: () => SortableOrientation;
    /** The name announcements call it by. */
    readonly label: () => string;
}

/** The items, containers and drag of a sortable, which its parts share. */
class SortableControl {
    /** The id of the instructions every handle points to. */
    readonly instructionsId: string;
    /** The drag under way, undefined while nothing moves. */
    readonly drag: Accessor<SortableDrag | undefined>;
    /** The last announcement. */
    readonly announcement: Accessor<string>;
    /** The item the drag's target marks and the side it marks, undefined without a drag. */
    readonly #mark: Accessor<{ readonly id: string; readonly drop: SortableDrop } | undefined>;
    /** The root's properties, read for nesting and the move handler. */
    readonly properties: SortableProperties;
    /** Replace the drag's signal. */
    readonly #setDrag: Setter<SortableDrag | undefined>;
    /** The drag as the last step left it, ahead of the signal's next flush. */
    #current: SortableDrag | undefined = undefined;
    /** The point a pointer pressed a handle at, undefined while no pointer holds one. */
    #start: { readonly x: number; readonly y: number } | undefined = undefined;
    /** Replace the announcement. */
    readonly #setAnnouncement: Setter<string>;
    /** The items by id. */
    readonly #items = new Map<string, SortableEntry>();
    /** The containers by id. */
    readonly #containers = new Map<string, SortableContainerEntry>();
    /** The reader's locale, which writes the announcements. */
    readonly #locale: Localization;

    /** Create the state of a sortable from its root's properties. */
    constructor(properties: SortableProperties, locale: Localization) {
        // start with nothing dragged and nothing announced
        const [drag, setDrag] = createSignal<SortableDrag | undefined>(undefined, {
            ownedWrite: true,
        });
        const [announcement, setAnnouncement] = createSignal("", { ownedWrite: true });
        this.instructionsId = createUniqueId();
        this.drag = drag;
        this.announcement = announcement;
        this.properties = properties;
        this.#setDrag = setDrag;
        this.#setAnnouncement = setAnnouncement;
        this.#locale = locale;
        this.#mark = createMemo(() => this.#markOf(this.drag()));
    }

    /** Add an item until it unmounts. */
    register(entry: SortableEntry): void {
        this.#items.set(entry.id, entry);
        onCleanup(() => {
            if (this.#items.get(entry.id) === entry) {
                this.#items.delete(entry.id);
            }
        });
    }

    /** Add a container until it unmounts. */
    registerContainer(entry: SortableContainerEntry): void {
        this.#containers.set(entry.id, entry);
        onCleanup(() => {
            if (this.#containers.get(entry.id) === entry) {
                this.#containers.delete(entry.id);
            }
        });
    }

    /** List a container's items in document order, leaving out the dragged one. */
    itemsOf(container: string, dragged: string | undefined = this.#current?.id): SortableEntry[] {
        return [...this.#items.values()]
            .filter((entry) => entry.container === container && entry.id !== dragged)
            .toSorted((left, right) => order(left.element(), right.element()));
    }

    /** Read the side of an item the dragged item lands on, undefined for an unmarked item. */
    dropOf(id: string): SortableDrop | undefined {
        const mark = this.#mark();

        return mark?.id === id ? mark.drop : undefined;
    }

    /** Report whether a container is the drag's target. */
    isTarget(container: string): boolean {
        return this.drag()?.target.container === container;
    }

    /** Lift an item where it stands, announcing it. */
    lift(id: string, mode: SortableDrag["mode"]): void {
        // find the item's place among its container's other items
        const entry = this.#items.get(id);
        if (entry === undefined) {
            return;
        }
        const siblings = this.itemsOf(entry.container);
        const index = siblings.findIndex((sibling) => sibling.id === id);
        const from = { container: entry.container, index: index === -1 ? siblings.length : index };

        // start the drag from there
        const drag = { id, mode, from, target: from, offset: { x: 0, y: 0 } };
        this.#set(drag);
        this.#announce("lift", drag);
    }

    /** Move the drag's target, announcing a keyboard move. */
    retarget(target: SortableTarget): void {
        // aim the drag anew, announcing only a keyboard step to another place
        const drag = this.#current;
        if (drag === undefined) {
            return;
        }
        const isChanged =
            target.container !== drag.target.container || target.index !== drag.target.index;
        const next = { ...drag, target };
        this.#set(next);
        if (isChanged && drag.mode === "keyboard") {
            this.#announce("move", next);
        }
    }

    /** Move the dragged item along with the pointer and aim at the place under it. */
    follow(x: number, y: number, offset: { readonly x: number; readonly y: number }): void {
        const drag = this.#current;
        if (drag === undefined) {
            return;
        }
        this.#set({ ...drag, offset, target: this.#targetAt(x, y) ?? drag.target });
    }

    /** Move the keyboard drag's target on an arrow key, reporting whether the key moved it. */
    step(key: string, direction: Direction): boolean {
        // read the target container's axis, mirrored left and right in right-to-left text
        const drag = this.#current;
        if (drag === undefined) {
            return false;
        }
        const container = this.#containers.get(drag.target.container);
        const isVertical = (container?.orientation() ?? "vertical") === "vertical";
        const forward = direction === "rtl" ? "ArrowLeft" : "ArrowRight";
        const backward = direction === "rtl" ? "ArrowRight" : "ArrowLeft";
        const { index } = drag.target;

        // step along the container, or across to a neighbouring container or nesting level
        let target: SortableTarget | undefined;
        if (key === (isVertical ? "ArrowDown" : forward)) {
            const count = this.itemsOf(drag.target.container).length;
            target = { ...drag.target, index: Math.min(index + 1, count) };
        } else if (key === (isVertical ? "ArrowUp" : backward)) {
            target = { ...drag.target, index: Math.max(index - 1, 0) };
        } else if (key === (isVertical ? forward : "ArrowDown")) {
            target = this.#across(drag.target, 1);
        } else if (key === (isVertical ? backward : "ArrowUp")) {
            target = this.#across(drag.target, -1);
        }
        if (target === undefined) {
            return false;
        }
        this.retarget(target);

        return true;
    }

    /** Take a key pressed on an item's handle: Space or Enter lifts or drops, the arrow keys move and Escape cancels. */
    press(event: KeyboardEvent, id: string, direction: Direction): void {
        // lift or drop on Space or Enter
        const drag = this.#current;
        if (event.key === " " || event.key === "Enter") {
            event.preventDefault();
            if (drag === undefined) {
                this.lift(id, "keyboard");
            } else {
                this.drop();
            }
        }
        // cancel on Escape
        else if (drag !== undefined && event.key === "Escape") {
            event.preventDefault();
            this.cancel();
        }
        // move on the arrow keys
        else if (drag !== undefined && this.step(event.key, direction)) {
            event.preventDefault();
        }
    }

    /** Put a keyboard drag back once the focus leaves its handle. */
    leave(): void {
        if (this.#current?.mode === "keyboard") {
            this.cancel();
        }
    }

    /** Hold the point a pointer pressed a handle at, waiting for it to travel before lifting. */
    hold(x: number, y: number): void {
        if (this.#current === undefined) {
            this.#start = { x, y };
        }
    }

    /** Follow a held pointer: lift the item once it travels far enough, then move the item with it. */
    track(id: string, x: number, y: number): void {
        // follow only a pointer that pressed a handle
        const start = this.#start;
        if (start === undefined) {
            return;
        }

        // lift once the pointer travelled far enough, then follow it
        const offset = { x: x - start.x, y: y - start.y };
        if (this.#current === undefined && Math.hypot(offset.x, offset.y) >= LIFT_DISTANCE) {
            this.lift(id, "pointer");
        }
        this.follow(x, y, offset);
    }

    /** Let go of a held pointer: drop a lifted item, leaving a click alone, or cancel the drag. */
    release(isCancelled: boolean): void {
        this.#start = undefined;
        if (isCancelled) {
            this.cancel();
        } else if (this.#current?.mode === "pointer") {
            this.drop();
        }
    }

    /** Drop the dragged item at its target, reporting a changed place to the owner. */
    drop(): void {
        // end the drag and announce where the item landed
        const drag = this.#current;
        if (drag === undefined) {
            return;
        }
        this.#announce("drop", drag);
        const items = this.itemsOf(drag.target.container, drag.id);
        this.#set(undefined);

        // report the move unless the item lands where it started
        const isSame =
            drag.target.container === drag.from.container && drag.target.index === drag.from.index;
        if (!isSame) {
            this.properties.onMove?.({
                id: drag.id,
                container: drag.target.container,
                previous: items[drag.target.index - 1]?.id,
                next: items[drag.target.index]?.id,
            });
        }

        // hand the focus back to the item's handle once the owner moved it
        if (drag.mode === "keyboard") {
            queueMicrotask(() => this.#items.get(drag.id)?.handle()?.focus());
        }
    }

    /** Put the dragged item back where it started. */
    cancel(): void {
        // announce the return to the start and end the drag
        const drag = this.#current;
        if (drag === undefined) {
            return;
        }
        this.#announce("cancel", { ...drag, target: drag.from });
        this.#set(undefined);
    }

    /** Replace the drag, keeping the latest for the steps that follow before the next flush. */
    #set(drag: SortableDrag | undefined): void {
        this.#current = drag;
        this.#setDrag(drag);
    }

    /** Find the item a drag's target marks: the item after the target, the last item at the end, or an item without items of its own it nests in. */
    #markOf(
        drag: SortableDrag | undefined,
    ): { readonly id: string; readonly drop: SortableDrop } | undefined {
        // mark nothing without a drag
        if (drag === undefined) {
            return undefined;
        }

        // mark the item the target nests in while it has no items of its own
        const { container, index } = drag.target;
        const items = this.itemsOf(container, drag.id);
        const next = items[index];
        const last = items.at(-1);
        if (items.length === 0) {
            return this.#items.has(container) ? { id: container, drop: "inside" } : undefined;
        }
        // mark the item after the target
        else if (next !== undefined) {
            return { id: next.id, drop: "before" };
        }
        // mark the last item when the target is the end
        else {
            return last === undefined ? undefined : { id: last.id, drop: "after" };
        }
    }

    /** Write the live region's announcement of a drag's step in the reader's language. */
    #announce(step: "lift" | "move" | "drop" | "cancel", drag: SortableDrag): void {
        // read the labels of the item and its container and the item's place
        const item = this.#items.get(drag.id)?.label() ?? drag.id;
        const container =
            this.#containers.get(drag.target.container)?.label() ??
            this.#items.get(drag.target.container)?.label() ??
            drag.target.container;
        const position = drag.target.index + 1;
        const count = this.itemsOf(drag.target.container, drag.id).length + 1;

        // say what happened
        let announcement: Message;
        if (step === "lift") {
            announcement = t`Picked up ${item}, position ${position} of ${count} in ${container}.`;
        } else if (step === "move") {
            announcement = t`${item} moved to position ${position} of ${count} in ${container}.`;
        } else if (step === "drop") {
            announcement = t`${item} dropped at position ${position} of ${count} in ${container}.`;
        } else {
            announcement = t`Moving ${item} was cancelled. It returned to position ${position} of ${count} in ${container}.`;
        }
        this.#setAnnouncement(this.#locale.render(announcement));
    }

    /** Find the target one step across: the neighbouring container of a board, or one nesting level in a tree. */
    #across(target: SortableTarget, offset: 1 | -1): SortableTarget | undefined {
        // nest under the item before the target, or move out after the item it nests in
        if (this.properties.nesting === true) {
            const items = this.itemsOf(target.container);
            const parent =
                this.#containers.get(target.container)?.parent ?? this.#parentOf(target.container);
            if (offset === 1) {
                const previous = items[target.index - 1];

                return previous === undefined
                    ? undefined
                    : { container: previous.id, index: this.itemsOf(previous.id).length };
            }
            const outer = parent === undefined ? undefined : this.#items.get(parent);
            if (outer === undefined) {
                return undefined;
            }
            const place = this.itemsOf(outer.container).findIndex((entry) => entry.id === outer.id);

            return { container: outer.container, index: place + 1 };
        }

        // move to the neighbouring container beside this one, at the same position or its end
        const current = this.#containers.get(target.container);
        const siblings = [...this.#containers.values()]
            .filter((entry) => entry.parent === current?.parent)
            .toSorted((left, right) => order(left.element(), right.element()));
        const neighbour =
            siblings[siblings.findIndex((entry) => entry.id === target.container) + offset];

        return neighbour === undefined
            ? undefined
            : {
                  container: neighbour.id,
                  index: Math.min(target.index, this.itemsOf(neighbour.id).length),
              };
    }

    /** Read the item a container id belongs to in a tree, as an item nests children without a container of its own. */
    #parentOf(container: string): string | undefined {
        return this.#items.has(container) ? container : undefined;
    }

    /** Find the place under the pointer: the innermost container holding it and the position its axis puts it at. */
    #targetAt(x: number, y: number): SortableTarget | undefined {
        // take the innermost container under the pointer outside the dragged item
        const dragged = this.#items.get(this.#current?.id ?? "")?.element();
        const holding = [...this.#containers.values()].filter((entry) => {
            const element = entry.element();

            return (
                element !== undefined &&
                dragged?.contains(element) !== true &&
                isWithin(element.getBoundingClientRect(), x, y)
            );
        });
        const innermost = holding.find((entry) =>
            holding.every(
                (other) =>
                    other === entry || other.element()?.contains(entry.element() ?? null) !== false,
            ),
        );
        if (innermost === undefined) {
            return undefined;
        }

        // land before the first item whose middle lies past the pointer, else at the end
        const isVertical = innermost.orientation() === "vertical";
        const items = this.itemsOf(innermost.id);
        const found = items.findIndex((entry) => {
            const rect = entry.element()?.getBoundingClientRect();

            return (
                rect !== undefined &&
                (isVertical ? y < rect.top + rect.height / 2 : x < rect.left + rect.width / 2)
            );
        });
        const index = found === -1 ? items.length : found;

        // nest under the item before the place when the pointer moves past its start in a tree
        const previous = items[index - 1]?.element()?.getBoundingClientRect();
        if (
            this.properties.nesting === true &&
            previous !== undefined &&
            x > previous.left + NEST_DISTANCE
        ) {
            const owner = items[index - 1];
            if (owner !== undefined) {
                return { container: owner.id, index: this.itemsOf(owner.id).length };
            }
        }

        return { container: innermost.id, index };
    }
}

/** The nearest sortable, null outside one. */
const SortableContext = createContext<SortableControl | null>(null);

/** The id of the nearest container, null outside one. */
const SortableContainerContext = createContext<string | null>(null);

/** The nearest item, null outside one. */
const SortableItemContext = createContext<SortableItemControl | null>(null);

/** The id and state of an item, which its handle and nested containers read. */
interface SortableItemControl {
    /** The item's id. */
    readonly id: string;
    /** Whether the item is dragged. */
    readonly isDragging: Accessor<boolean>;
    /** Whether the item ignores the person. */
    readonly isDisabled: Accessor<boolean>;
    /** Set the element of the item's handle. */
    readonly setHandle: (element: HTMLElement) => void;
}

/** The properties of an element of a sortable, the native element's attributes included. */
export type SortableElementProperties<Attributes> = Omit<Attributes, "class"> & {
    /** The StyleX styles applied after the element's styles. */
    readonly xstyle?: style.Styles;
};

/** The properties of a sortable, the native element's attributes included. */
export interface SortableProperties extends SortableElementProperties<
    JSX.HTMLAttributes<HTMLDivElement>
> {
    /** Handle a person dropping an item at another place, which the owner renders the items in. */
    readonly onMove?: (move: SortableMove) => void;
    /** Whether items nest under one another as a tree: the cross arrows and a pointer moved past an item's start nest and unnest. */
    readonly nesting?: boolean;
}

/** The properties of a sortable's container, the native element's attributes included. */
export interface SortableContainerProperties extends SortableElementProperties<
    JSX.HTMLAttributes<HTMLDivElement>
> {
    /** The container's id, which a move reports. */
    readonly id: string;
    /** The direction its items lay out along, vertical by default. */
    readonly orientation?: SortableOrientation;
    /** The name announcements call it by, a generic one by default. */
    readonly label?: string;
}

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
    // share the drag with the parts
    const locale = useLocale();
    const control = new SortableControl(properties, locale);
    const rest = omit(properties, "onMove", "nesting", "xstyle", "style", "children");

    return (
        <SortableContext value={control}>
            <div
                data-slot="sortable"
                data-state={control.drag() === undefined ? "idle" : "dragging"}
                {...rest}
                {...style.attributes([properties.xstyle], properties.style)}
            >
                {properties.children}
                <div id={control.instructionsId} hidden>
                    {locale.render(
                        t`Press Space to pick up an item. Use the arrow keys to move it, Space to drop it and Escape to cancel.`,
                    )}
                </div>
                <div
                    role="status"
                    aria-live="assertive"
                    aria-atomic="true"
                    data-slot="sortable-announcer"
                    {...style.attrs(visuallyHiddenStyle())}
                >
                    {control.announcement()}
                </div>
            </div>
        </SortableContext>
    );
}

/** Render a container of the nearest sortable, such as a list, a board's column or an item's children. */
export function SortableContainer(properties: SortableContainerProperties): JSX.Element {
    // join the sortable under the item the container nests in, if any
    const control = useSortable();
    const locale = useLocale();
    const parent = useContext(SortableItemContext);
    const rest = omit(properties, "id", "orientation", "label", "xstyle", "style");
    let element: HTMLDivElement | undefined;
    control.registerContainer({
        id: properties.id,
        parent: parent?.id,
        element: () => element,
        orientation: () => properties.orientation ?? "vertical",
        label: () => properties.label ?? locale.render(t`the list`),
    });

    return (
        <SortableContainerContext value={properties.id}>
            <div
                data-slot="sortable-container"
                data-state={control.isTarget(properties.id) ? "over" : "idle"}
                data-orientation={properties.orientation ?? "vertical"}
                {...rest}
                ref={(created) => (element = created)}
                {...style.attributes([properties.xstyle], properties.style)}
            />
        </SortableContainerContext>
    );
}

/** Render an item of the nearest container, marked while dragged and on the side a dragged item lands. */
export function SortableItem(properties: SortableItemProperties): JSX.Element {
    // join the container, refusing an item outside one
    const control = useSortable();
    const container = useContext(SortableContainerContext);
    if (container === null) {
        throw new TypeError("a sortable item needs a sortable container around it");
    }
    const rest = omit(properties, "id", "label", "disabled", "xstyle", "style", "children");
    let element: HTMLDivElement | undefined;
    let handle: HTMLElement | undefined;
    control.register({
        id: properties.id,
        container,
        element: () => element,
        handle: () => handle,
        label: () => properties.label ?? element?.textContent?.trim() ?? properties.id,
    });
    const isDragging = (): boolean => control.drag()?.id === properties.id;
    const isVertical = (): boolean =>
        element?.closest("[data-slot=sortable-container]")?.getAttribute("data-orientation") !==
        "horizontal";

    return (
        <SortableItemContext
            value={{
                id: properties.id,
                isDragging,
                isDisabled: () => properties.disabled === true,
                setHandle: (created) => (handle = created),
            }}
        >
            <div
                data-slot="sortable-item"
                data-state={isDragging() ? "dragging" : "idle"}
                data-drop={control.dropOf(properties.id)}
                data-disabled={properties.disabled === true ? "" : undefined}
                {...rest}
                ref={(created) => (element = created)}
                {...style.attributes(
                    [
                        styles.item,
                        isDragging() && styles.dragging,
                        dropStyle(control.dropOf(properties.id), isVertical()),
                        offsetStyle(control.drag(), properties.id),
                        properties.xstyle,
                    ],
                    properties.style,
                )}
            >
                {properties.children}
            </div>
        </SortableItemContext>
    );
}

/** Render the button that lifts the nearest item: a pointer drags it, Space lifts and drops it, the arrow keys move it and Escape puts it back. */
export function SortableHandle(properties: SortableHandleProperties): JSX.Element {
    // read the sortable and the item the handle moves
    const control = useSortable();
    const locale = useLocale();
    const item = useContext(SortableItemContext);
    if (item === null) {
        throw new TypeError("a sortable handle needs a sortable item around it");
    }
    const rest = omit(properties, "xstyle", "style", "children");

    return (
        <button
            type="button"
            aria-roledescription={locale.render(t`sortable`)}
            aria-describedby={control.instructionsId}
            aria-pressed={item.isDragging() ? "true" : "false"}
            aria-label={locale.render(t`Move`)}
            data-slot="sortable-handle"
            data-state={item.isDragging() ? "dragging" : "idle"}
            disabled={item.isDisabled()}
            {...rest}
            ref={(created) => item.setHandle(created)}
            onKeyDown={(event) => control.press(event, item.id, locale.direction)}
            onBlur={() => control.leave()}
            onPointerDown={(event) => {
                // hold the pointer on the handle until it travels or lets go
                if (event.button === 0) {
                    event.currentTarget.setPointerCapture(event.pointerId);
                    control.hold(event.clientX, event.clientY);
                }
            }}
            onPointerMove={(event) => control.track(item.id, event.clientX, event.clientY)}
            onPointerUp={() => control.release(false)}
            onPointerCancel={() => control.release(true)}
            {...style.attributes(
                [styles.handle, item.isDragging() && styles.grabbing, properties.xstyle],
                properties.style,
            )}
        >
            {properties.children ?? <Icon name="dots-six-vertical" />}
        </button>
    );
}

/** Read the nearest sortable, refusing a part outside one. */
function useSortable(): SortableControl {
    const control = useContext(SortableContext);
    if (control === null) {
        throw new TypeError("a sortable part needs a sortable around it");
    }

    return control;
}

/** Return the styles that mark where a dragged item lands beside an item along its container's axis. */
function dropStyle(drop: SortableDrop | undefined, isVertical: boolean): style.Styles {
    if (drop === "inside") {
        return styles.inside;
    } else if (drop === "before") {
        return isVertical ? styles.before : styles.beforeInline;
    } else if (drop === "after") {
        return isVertical ? styles.after : styles.afterInline;
    } else {
        return false;
    }
}

/** Return the styles that move an item a pointer drags along with the pointer, none otherwise. */
function offsetStyle(drag: SortableDrag | undefined, id: string): style.Styles {
    return drag !== undefined && drag.id === id && drag.mode === "pointer"
        ? [styles.lifted, offsets.translate(drag.offset.x, drag.offset.y)]
        : false;
}

/** Compare two elements by their place in the document. */
function order(left: HTMLElement | undefined, right: HTMLElement | undefined): number {
    if (left === undefined || right === undefined || left === right) {
        return 0;
    }

    return left.compareDocumentPosition(right) & Node.DOCUMENT_POSITION_FOLLOWING ? -1 : 1;
}

/** Report whether a point lies within a rectangle. */
function isWithin(rect: DOMRect, x: number, y: number): boolean {
    return x >= rect.left && x <= rect.right && y >= rect.top && y <= rect.bottom;
}
