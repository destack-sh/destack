import { state } from "@destack/theme/state";
import { isMotionReduced } from "@destack/view/motion";
import type { Axis } from "../position/index.ts";
import { Icon } from "@destack/ui/icon";
import { type Localization, t } from "@destack/locale";
import * as style from "@destack/style";
import { color, motion, radius, space, stroke, swatch } from "@destack/theme/tokens.stylex";
import {
    type Accessor,
    createContext,
    createMemo,
    type JSX,
    omit,
    onCleanup,
    untrack,
    useContext,
    useEnvironment,
    useLocale,
} from "@destack/view";
import type { Environment } from "@destack/environment";
import {
    DragAnnouncer,
    Draggable,
    DragManager,
    type DragOperation,
    DragPreview,
    type DragPlace,
    type DragStep,
    type DragTarget,
    Droppable,
    type DroppableOptions,
} from "../drag/index.ts";
import { mergeProperties } from "../part/index.ts";

/** The distance in pixels a pointer moves past an item's start to nest under it in a tree. */
const NEST_DISTANCE = 32;

/** The line a target item shows along the edge the dragged item lands on. */
const LINE = `calc(2 * ${stroke.border})`;

/** The time a moved item glides to its new place, in milliseconds: an on-screen move. */
const GLIDE_TIME = 250;

/** The curve a moved item glides along: Emil Kowalski's strong ease-in-out for on-screen moves. */
const GLIDE_EASING = "cubic-bezier(0.77, 0, 0.175, 1)";

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
    asHandle: {
        userSelect: "none",
    },
    marked: {
        "::after": {
            content: '""',
            position: "absolute",
            borderRadius: LINE,
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
    handle: {
        display: "inline-flex",
        alignItems: "center",
        justifyContent: "center",
        padding: space[1],
        borderRadius: radius[1],
        color: color.mutedForeground,
        cursor: { default: "grab", ":disabled": "not-allowed" },
    },
    grabbing: {
        cursor: "grabbing",
    },
});

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

/** An item of a sortable. */
interface SortableEntry {
    /** The item's id. */
    readonly id: string;
    /** The id of its container. */
    readonly container: string;
    /** The item's element. */
    readonly element: () => HTMLElement | undefined;
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
    /** Report whether it takes a given item. */
    readonly allows: (id: string) => boolean;
    /** The lists a keyboard step crosses between. */
    readonly group: () => string;
}

/** The items of a sortable and the drags that move them. */
class SortableControl {
    /** The drags of the sortable. */
    readonly manager: DragManager;
    /** The root's properties, read for nesting. */
    readonly properties: SortableProperties;
    /** The items by id. */
    readonly #items = new Map<string, SortableEntry>();
    /** The one item the drag marks and the side it lands on, found once per step of the drag. */
    readonly #marked: Accessor<{ readonly id: string; readonly drop: SortableDrop } | undefined>;
    /** The display whose next frame a glide waits for. */
    readonly #environment: Environment<"display">;
    /** The bounds of each container's items as the drag lifted. */
    readonly #measured = new Map<
        string,
        readonly { readonly id: string; readonly bounds: DOMRect }[]
    >();

    /** Create the state of a sortable from its root's properties. */
    constructor(
        properties: SortableProperties,
        locale: Localization,
        environment: Environment<"display">,
    ) {
        // glide every move the drags drop
        this.properties = properties;
        this.#environment = environment;
        this.manager = new DragManager(
            {
                onDrop: (operation) => {
                    const move = this.moveOf(operation);
                    this.glide(() => properties.onMove?.(move));
                },
            },
            locale,
            environment,
        );
        this.#marked = createMemo(() => this.#markedOf());
    }

    /** Run a move and glide each item from where it was to where the move puts it, after FLIP. */
    glide(move: () => void): void {
        // remember where each item was, unless the person wants reduced motion
        const elements = [...this.#items.values()].flatMap((entry) => {
            const element = entry.element();

            return element === undefined ? [] : [element];
        });
        const [first] = elements;
        const isStill = first !== undefined && isMotionReduced(first);
        const before = new Map(
            elements.map((element) => [element, element.getBoundingClientRect()]),
        );
        move();
        if (isStill) {
            return;
        }

        // once the move renders, play each item that moved from its old place to its new one
        void this.#environment.display.frame().then(() => {
            for (const [element, was] of before) {
                const now = element.getBoundingClientRect();
                const [x, y] = [was.left - now.left, was.top - now.top];
                if (element.isConnected && (x !== 0 || y !== 0)) {
                    element.animate(
                        [
                            { transform: `translate(${String(x)}px, ${String(y)}px)` },
                            { transform: "none" },
                        ],
                        { duration: GLIDE_TIME, easing: GLIDE_EASING },
                    );
                }
            }
        });
    }

    /** Add an item until it unmounts. */
    addItem(entry: SortableEntry): void {
        this.#items.set(entry.id, entry);
        onCleanup(() => {
            if (this.#items.get(entry.id) === entry) {
                this.#items.delete(entry.id);
            }
        });
    }

    /** Add a container's droppable to the drags until it unmounts. */
    addList(list: SortableList): void {
        void new Droppable(this.#droppableOf(list), this.manager);
    }

    /** Read an item's element. */
    elementOf(id: string): HTMLElement | undefined {
        return this.#items.get(id)?.element();
    }

    /** List a container's items in document order, leaving out the dragged one. */
    itemsOf(container: string): SortableEntry[] {
        const dragged = this.manager.current()?.item;

        return [...this.#items.values()]
            .filter((entry) => entry.container === container && entry.id !== dragged)
            .toSorted((left, right) => order(left.element(), right.element()));
    }

    /** Read the target of the item that stands at a place in its container. */
    homeOf(id: string): DragTarget {
        // find the item among its container's items in document order
        const entry = this.#items.get(id);
        if (entry === undefined) {
            throw new TypeError(`sortable item without an entry: '${id}'`);
        }
        const siblings = [...this.#items.values()]
            .filter((each) => each.container === entry.container)
            .toSorted((left, right) => order(left.element(), right.element()));

        return { kind: "list", droppable: entry.container, index: siblings.indexOf(entry) };
    }

    /** Read the side of an item the dragged item lands on. */
    dropOf(id: string): SortableDrop | undefined {
        const marked = this.#marked();

        return marked?.id === id ? marked.drop : undefined;
    }

    /** Find the one item a drag marks and the side it lands on. */
    #markedOf(): { readonly id: string; readonly drop: SortableDrop } | undefined {
        // mark nothing without a list target
        const target = this.manager.operation()?.target;
        if (target?.kind !== "list") {
            return undefined;
        }

        // mark the item the target nests in while it has no items
        const items = this.itemsOf(target.droppable);
        const next = items[target.index];
        const last = items.at(-1);
        if (items.length === 0) {
            return { id: target.droppable, drop: "inside" };
        } else if (next !== undefined) {
            return { id: next.id, drop: "before" };
        }

        return last === undefined ? undefined : { id: last.id, drop: "after" };
    }

    /** Report a drop as a move between the items it lands between. */
    moveOf(operation: DragOperation & { readonly target: DragTarget }): SortableMove {
        // read the list it lands in and its neighbours there
        const target = operation.target;
        if (target.kind !== "list") {
            throw new TypeError(`sortable drop outside a list: ${target.kind}`);
        }
        const items = this.itemsOf(target.droppable);

        return {
            id: operation.item,
            container: target.droppable,
            previous: items[target.index - 1]?.id,
            next: items[target.index]?.id,
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
                index: Math.min(
                    from.kind === "list" ? from.index : 0,
                    this.itemsOf(list.id).length,
                ),
            }),
            describe: (target) => this.#describe(list, target),
        };
    }

    /** Read the bounds of a container's items once as a drag lifts. */
    #measure(list: SortableList): void {
        this.#measured.set(
            list.id,
            this.itemsOf(list.id).flatMap((entry) => {
                const element = entry.element();

                return element === undefined
                    ? []
                    : [{ id: entry.id, bounds: element.getBoundingClientRect() }];
            }),
        );
    }

    /** Find the place a point lands at among a container's items. */
    #targetAt(list: SortableList, x: number, y: number): DragTarget {
        // find the first item past the pointer along the container's direction
        const isVertical = list.orientation() === "vertical";
        const items = this.#measured.get(list.id) ?? [];
        const found = items.findIndex(({ bounds }) =>
            isVertical ? y < bounds.top + bounds.height / 2 : x < bounds.left + bounds.width / 2,
        );
        const index = found === -1 ? items.length : found;

        // nest under the item before when the pointer moves past its start in a tree
        const previous = items[index - 1];
        if (
            this.properties.nesting === true &&
            previous !== undefined &&
            x > previous.bounds.left + NEST_DISTANCE
        ) {
            return {
                kind: "list",
                droppable: previous.id,
                index: this.itemsOf(previous.id).length,
            };
        }

        return { kind: "list", droppable: list.id, index };
    }

    /** Step a target along the container, or in and out of nesting across it in a tree. */
    #step(list: SortableList, target: DragTarget, step: DragStep): DragTarget | undefined {
        // step along the container's own axis within its ends
        const index = target.kind === "list" ? target.index : 0;
        const count = this.itemsOf(list.id).length;
        const isVertical = list.orientation() === "vertical";
        const forward = isVertical ? "block-end" : "inline-end";
        const backward = isVertical ? "block-start" : "inline-start";
        if (step === forward) {
            return index < count
                ? { kind: "list", droppable: list.id, index: index + 1 }
                : undefined;
        } else if (step === backward) {
            return index > 0 ? { kind: "list", droppable: list.id, index: index - 1 } : undefined;
        }
        // nest under the item before, or move out after the item it nests in
        else if (this.properties.nesting === true && step === "inline-end") {
            const previous = this.itemsOf(list.id)[index - 1];

            return previous === undefined
                ? undefined
                : { kind: "list", droppable: previous.id, index: this.itemsOf(previous.id).length };
        } else if (this.properties.nesting === true && step === "inline-start") {
            const outer = list.parent === undefined ? undefined : this.#items.get(list.parent);

            return outer === undefined
                ? undefined
                : {
                      kind: "list",
                      droppable: outer.container,
                      index:
                          this.itemsOf(outer.container).findIndex((each) => each.id === outer.id) +
                          1,
                  };
        }

        return undefined;
    }

    /** Describe a place in a container as its position among the items. */
    #describe(list: SortableList, target: DragTarget): DragPlace {
        return {
            kind: "position",
            position: (target.kind === "list" ? target.index : 0) + 1,
            count: this.itemsOf(list.id).length + 1,
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
    /** Set the element of the item's handle. */
    readonly setHandle: (element: HTMLElement) => void;
    /** Set the element around the item's children in a tree. */
    readonly setChildren: (element: HTMLElement) => void;
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
    /** Whether items nest under one another as a tree. */
    readonly nesting?: boolean;
}

/** The properties of a sortable's container, the native element's attributes included. */
export interface SortableContainerProperties extends SortableElementProperties<
    JSX.HTMLAttributes<HTMLDivElement>
> {
    /** The container's id. */
    readonly id: string;
    /** The direction its items lay out along, vertical by default. */
    readonly orientation?: Axis;
    /** The name announcements call it by, a generic one by default. */
    readonly label?: string;
    /** The types of items it takes. */
    readonly accepts?: readonly string[];
    /** Report whether it takes a given item beside its types. */
    readonly allows?: (id: string) => boolean;
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
    /** The type of item. */
    readonly type?: string;
    /** Whether the whole item lifts. */
    readonly asHandle?: boolean;
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
    const control = new SortableControl(properties, useLocale(), useEnvironment());
    const manager = control.manager;
    const rest = omit(properties, "onMove", "nesting", "xstyle", "style", "children");

    return (
        <SortableContext value={control}>
            <div
                data-slot="sortable"
                data-state={manager.operation() === undefined ? "idle" : "dragging"}
                {...rest}
                {...style.attributes([properties.xstyle], properties.style)}
            >
                {properties.children}
                <DragPreview manager={manager} element={(id) => control.elementOf(id)} />
                <DragAnnouncer manager={manager} />
            </div>
        </SortableContext>
    );
}

/** Render a container of the nearest sortable. */
export function SortableContainer(properties: SortableContainerProperties): JSX.Element {
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
        "allows",
        "xstyle",
        "style",
    );
    let element: HTMLDivElement | undefined;

    // read the container's id and the sortable's nesting once
    const id = untrack(() => properties.id);
    const isNesting = untrack(() => control.properties.nesting === true);

    // leave an item's children to its own droppable, or add the container's own
    const isChildren = isNesting && parent?.id === id;
    if (!isChildren) {
        control.addList({
            id,
            parent: parent?.id,
            element: () => element,
            orientation: () => properties.orientation ?? "vertical",
            label: () => properties.label ?? locale.render(t`the list`),
            accepts: () => properties.accepts ?? [DEFAULT_TYPE],
            allows: (item) => properties.allows?.(item) !== false,
            group: () => (properties.accepts ?? [DEFAULT_TYPE]).join(" "),
        });
    }

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
        <SortableContainerContext
            value={{ id, orientation: () => properties.orientation ?? "vertical" }}
        >
            <div
                data-slot="sortable-container"
                data-state={stateOf()}
                data-orientation={properties.orientation ?? "vertical"}
                {...rest}
                ref={(created) => {
                    element = created;
                    if (isChildren) {
                        parent.setChildren(created);
                    }
                }}
                {...style.attributes([properties.xstyle], properties.style)}
            />
        </SortableContainerContext>
    );
}

/** Render an item of the nearest container. */
export function SortableItem(properties: SortableItemProperties): JSX.Element {
    // join the container, refusing an item outside one
    const control = useSortable();
    const locale = useLocale();
    const container = useContext(SortableContainerContext);
    if (container === null) {
        throw new TypeError("a sortable item needs a sortable container around it");
    }
    const rest = omit(
        properties,
        "id",
        "label",
        "disabled",
        "type",
        "asHandle",
        "xstyle",
        "style",
        "children",
    );
    let element: HTMLDivElement | undefined;
    let handle: HTMLElement | undefined;
    let children: HTMLElement | undefined;

    // read the item's fixed properties once
    const id = untrack(() => properties.id);
    const type = untrack(() => properties.type ?? DEFAULT_TYPE);
    const isHandle = untrack(() => properties.asHandle === true);
    const isNesting = untrack(() => control.properties.nesting === true);

    // add the item and the handle that takes the focus back
    control.addItem({ id, container: container.id, element: () => element });
    const draggable = new Draggable(
        {
            id,
            type,
            kind: "move",
            label: () => properties.label ?? element?.textContent?.trim() ?? id,
            element: () => (isHandle ? element : handle),
            home: () => control.homeOf(id),
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
                properties.label ?? element?.textContent?.trim() ?? locale.render(t`the item`),
            accepts: () => [type],
            allows: () => true,
            group: () => id,
        });
    }

    // lift the whole item by pointer, touch and keys when it is its own handle
    const lifting = isHandle ? mergeProperties({ tabindex: 0 }, draggable.attributes([" "])) : {};

    // follow the drag of this item
    const isDragging = (): boolean => control.manager.isDragging(id);
    const isVertical = (): boolean => container.orientation() === "vertical";

    return (
        <SortableItemContext
            value={{
                id,
                draggable,
                isDisabled: () => properties.disabled === true,
                setHandle: (created) => (handle = created),
                setChildren: (created) => (children = created),
            }}
        >
            <div
                data-slot="sortable-item"
                data-state={isDragging() ? "dragging" : "idle"}
                data-drop={control.dropOf(id)}
                data-disabled={properties.disabled === true ? "" : undefined}
                {...mergeProperties(
                    mergeProperties(lifting, {
                        ref: (created: HTMLDivElement) => (element = created),
                    }),
                    rest,
                )}
                {...style.attributes(
                    [
                        styles.item,
                        isHandle && styles.asHandle,
                        isDragging() && styles.dragging,
                        dropStyle(control.dropOf(id), isVertical()),
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
            "aria-label": locale.render(t`Move`),
            "data-slot": "sortable-handle",
            get "aria-pressed"(): "true" | "false" {
                return isDragging() ? "true" : "false";
            },
            get "data-state"(): "dragging" | "idle" {
                return isDragging() ? "dragging" : "idle";
            },
        },
        item.draggable.attributes([" ", "Enter"]),
    );

    return (
        <button
            {...handle}
            disabled={item.isDisabled()}
            {...rest}
            ref={(created) => item.setHandle(created)}
            {...style.attributes(
                [
                    state.focus,
                    state.disabled,
                    styles.handle,
                    isDragging() && styles.grabbing,
                    properties.xstyle,
                ],
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

/** Return the styles that mark where a dragged item lands beside an item. */
function dropStyle(drop: SortableDrop | undefined, isVertical: boolean): style.Styles {
    if (drop === "inside") {
        return [styles.marked, styles.inside];
    } else if (drop === "before") {
        return [styles.marked, isVertical ? styles.before : styles.beforeInline];
    } else if (drop === "after") {
        return [styles.marked, isVertical ? styles.after : styles.afterInline];
    }

    return false;
}

/** Compare two elements by their place in the document. */
function order(left: HTMLElement | undefined, right: HTMLElement | undefined): number {
    if (left === undefined || right === undefined || left === right) {
        return 0;
    }

    return left.compareDocumentPosition(right) & Node.DOCUMENT_POSITION_FOLLOWING ? -1 : 1;
}
