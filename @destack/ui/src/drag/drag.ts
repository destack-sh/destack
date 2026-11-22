import type { Environment } from "@destack/environment";
import { type Direction, type Localization, type Message, t } from "@destack/locale";
import {
    type Accessor,
    createSignal,
    createUniqueId,
    flush,
    onCleanup,
    type Setter,
} from "@destack/view";
import { Move, type PointerType } from "../interaction/index.ts";
import { mergeProperties } from "../part/index.ts";

/** The milliseconds a finger rests on an item before it lifts, as iOS and Android wait. */
const LIFT_DELAY = 500;

/** The pixels from a scroller's edge where a drag scrolls it, about a finger's width. */
const SCROLL_EDGE = 48;

/** The pixels a drag scrolls per frame at the very edge, about 1200 pixels per second at 60 Hz. */
const SCROLL_SPEED = 20;

/** What a drag does to its item. */
export type DragKind = "move" | "resize-start" | "resize-end" | "create";

/** A keyboard step along the flow of the text, in CSS's logical directions. */
export type DragStep = "block-start" | "block-end" | "inline-start" | "inline-end";

/** Where an item lands. */
export type DragTarget =
    | {
          /** A place in a list. */
          readonly kind: "list";
          /** The droppable. */
          readonly droppable: string;
          /** The place among the list's other items. */
          readonly index: number;
      }
    | {
          /** A cell of a grid. */
          readonly kind: "cell";
          /** The droppable. */
          readonly droppable: string;
          /** The cell's key, such as a date. */
          readonly key: string;
      }
    | {
          /** A time on an axis. */
          readonly kind: "time";
          /** The droppable. */
          readonly droppable: string;
          /** The time, in milliseconds since the epoch. */
          readonly at: number;
      };

/** A place an announcement describes. */
export type DragPlace =
    | {
          /** A position in a list. */
          readonly kind: "position";
          /** The position, from one. */
          readonly position: number;
          /** The number of positions. */
          readonly count: number;
          /** The list's name. */
          readonly list: string;
      }
    | {
          /** A labeled place. */
          readonly kind: "label";
          /** The label, such as Tuesday 14 May, 10:30. */
          readonly label: string;
      };

/** A drag under way. */
export interface DragOperation {
    /** The dragged item. */
    readonly item: string;
    /** The draggable that lifted it. */
    readonly draggable: string;
    /** What the drag does to it. */
    readonly kind: DragKind;
    /** What moves it: a mouse, a pen, a finger or the keyboard. */
    readonly pointer: PointerType;
    /** Where the item stood. */
    readonly from: DragTarget;
    /** Where the drag took the item: under the pointer as it began, else where the item stood. */
    readonly grab: DragTarget;
    /** Where it lands, absent outside every droppable. */
    readonly target: DragTarget | undefined;
    /** The horizontal distance a pointer moved it, zero on the keyboard. */
    readonly offsetX: number;
    /** The vertical distance a pointer moved it, zero on the keyboard. */
    readonly offsetY: number;
}

/** The options of a draggable. */
export interface DraggableOptions {
    /** The draggable's id, unique in its manager. */
    readonly id: string;
    /** The item it drags, its id unless several draggables drag one item. */
    readonly item?: string;
    /** The kind of item. */
    readonly type: string;
    /** What a drag does to it. */
    readonly kind: DragKind;
    /** The name announcements call it by. */
    readonly label: () => string;
    /** The element that takes the focus back after a keyboard drag. */
    readonly element: () => HTMLElement | undefined;
    /** Read where the item stands. */
    readonly home: () => DragTarget;
}

/** The options of a droppable. */
export interface DroppableOptions {
    /** The droppable's id. */
    readonly id: string;
    /** The droppables a keyboard step crosses between. */
    readonly group: string;
    /** The axis its own steps follow. */
    readonly axis: "block" | "inline" | "both";
    /** The element, whose bounds take the pointer. */
    readonly element: () => HTMLElement | undefined;
    /** Report whether an item may land here by a kind of drag. */
    readonly accepts: (draggable: Draggable, kind: DragKind) => boolean;
    /** Read the layout once as a drag lifts or scrolls. */
    readonly measure: (operation: DragOperation) => void;
    /** Find where a point lands. */
    readonly targetAt: (x: number, y: number, operation: DragOperation) => DragTarget | undefined;
    /** Step a target along the droppable's axis. */
    readonly step: (target: DragTarget, step: DragStep) => DragTarget | undefined;
    /** Enter from a neighbouring droppable's target. */
    readonly enter: (from: DragTarget) => DragTarget;
    /** Describe a target for announcements. */
    readonly describe: (target: DragTarget, operation: DragOperation) => DragPlace;
}

/** The handlers of a drag manager. */
export interface DragOptions {
    /** Handle a drop at a target other than where the item stood. */
    readonly onDrop: (operation: DragOperation & { readonly target: DragTarget }) => void;
}

/** An item a person drags. */
export class Draggable {
    /** The draggable's id, unique in its manager. */
    readonly id: string;
    /** The item it drags. */
    readonly item: string;
    /** The kind of item. */
    readonly type: string;
    /** What a drag does to it. */
    readonly kind: DragKind;
    /** The name announcements call it by. */
    readonly label: () => string;
    /** The element that takes the focus back after a keyboard drag. */
    readonly element: () => HTMLElement | undefined;
    /** Read where the item stands. */
    readonly home: () => DragTarget;
    /** The manager that follows its drags. */
    readonly #manager: DragManager;

    /** Create a draggable and add it to a manager until its owner goes. */
    constructor(options: DraggableOptions, manager: DragManager) {
        // copy the options
        this.id = options.id;
        this.item = options.item ?? options.id;
        this.type = options.type;
        this.kind = options.kind;
        this.label = options.label;
        this.element = options.element;
        this.home = options.home;
        this.#manager = manager;
        manager.addDraggable(this);
    }

    /** Read whether the item is dragged. */
    isDragging(): boolean {
        return this.#manager.isDragging(this.item);
    }

    /** Read the handlers and state the element that lifts it spreads. */
    attributes(lifts: readonly string[]) {
        // lift a mouse or pen past the distance and a finger held still
        const manager = this.#manager;
        const move = new Move(
            {
                delay: LIFT_DELAY,
                keys: [],
                onMoveStart: (event) => manager.lift(this.id, this.kind, event.pointerType),
                onMove: (event) =>
                    manager.follow(event.clientX, event.clientY, event.offsetX, event.offsetY),
                onMoveEnd: (event) => (event.isCancelled ? manager.cancel() : manager.drop()),
            },
            manager.environment.clock,
        );

        // lift, drop, cancel and step on the keys
        const keys = {
            onKeyDown: (event: KeyboardEvent) => this.#key(event, lifts),
            onBlur: () =>
                manager.current()?.pointer === "keyboard" ? manager.cancel() : undefined,
        };

        // describe the element and mark it while dragged
        const isDragging = () => this.isDragging();
        const state = {
            role: "button" as const,
            "aria-roledescription": manager.locale.render(t`draggable`),
            "aria-describedby": manager.instructionsId,
            get "data-dragging"(): "" | undefined {
                return isDragging() ? "" : undefined;
            },
        };

        return mergeProperties(mergeProperties(state, keys), move.attributes());
    }

    /** Lift, drop, cancel or step a keyboard drag on a key pressed on the element. */
    #key(event: KeyboardEvent, lifts: readonly string[]): void {
        // lift on a lifting key, else hand the key to the drag in progress
        const manager = this.#manager;
        if (manager.current() === undefined && lifts.includes(event.key)) {
            event.preventDefault();
            manager.lift(this.id, this.kind, "keyboard");
        } else {
            manager.key(event);
        }
    }
}

/** A place items land in. */
export class Droppable {
    /** The droppable's id. */
    readonly id: string;
    /** The droppables a keyboard step crosses between. */
    readonly group: string;
    /** The axis its own steps follow. */
    readonly axis: "block" | "inline" | "both";
    /** The element, whose bounds take the pointer. */
    readonly element: DroppableOptions["element"];
    /** Report whether an item may land here by a kind of drag. */
    readonly accepts: DroppableOptions["accepts"];
    /** Read the layout once as a drag lifts or scrolls. */
    readonly measure: DroppableOptions["measure"];
    /** Find where a point lands. */
    readonly targetAt: DroppableOptions["targetAt"];
    /** Step a target along the droppable's axis. */
    readonly step: DroppableOptions["step"];
    /** Enter from a neighbouring droppable's target. */
    readonly enter: DroppableOptions["enter"];
    /** Describe a target for announcements. */
    readonly describe: DroppableOptions["describe"];

    /** Create a droppable and add it to a manager until its owner goes. */
    constructor(options: DroppableOptions, manager: DragManager) {
        // copy the options
        this.id = options.id;
        this.group = options.group;
        this.axis = options.axis;
        this.element = options.element;
        this.accepts = options.accepts;
        this.measure = options.measure;
        this.targetAt = options.targetAt;
        this.step = options.step;
        this.enter = options.enter;
        this.describe = options.describe;
        manager.addDroppable(this);
    }
}

/** A droppable's measured bounds and the scroller around it. */
interface Measured {
    /** The droppable. */
    readonly droppable: Droppable;
    /** Its bounds as the drag lifted or last scrolled. */
    readonly bounds: DOMRect;
    /** The nearest scrolling element around it. */
    readonly scroller: HTMLElement | undefined;
}

/** The drags of one view. */
export class DragManager {
    /** The id of the instructions every draggable points to. */
    readonly instructionsId: string;
    /** The drag under way. */
    readonly operation: Accessor<DragOperation | undefined>;
    /** The last announcement. */
    readonly announcement: Accessor<string>;
    /** The draggables by id. */
    readonly #draggables = new Map<string, Draggable>();
    /** The droppables by id. */
    readonly #droppables = new Map<string, Droppable>();
    /** The handlers. */
    readonly #options: DragOptions;
    /** The reader's locale. */
    readonly locale: Localization;
    /** The clock a finger's lift waits on and the display autoscroll steps with. */
    readonly environment: Environment<"display">;
    /** Replace the drag. */
    readonly #setOperation: Setter<DragOperation | undefined>;
    /** Replace the announcement. */
    readonly #setAnnouncement: Setter<string>;
    /** The drag as the last step left it, ahead of the signal's next flush. */
    #current: DragOperation | undefined = undefined;
    /** The accepting droppables measured as the drag lifted. */
    #measured: readonly Measured[] = [];
    /** The last pointer position. */
    #pointer: { readonly x: number; readonly y: number } | undefined = undefined;
    /** The wait for autoscroll's next frame. */
    #frame: AbortController | undefined = undefined;

    /** Create the drags of a view. */
    constructor(options: DragOptions, locale: Localization, environment: Environment<"display">) {
        // start with nothing dragged and nothing announced
        const [operation, setOperation] = createSignal<DragOperation | undefined>(undefined, {
            ownedWrite: true,
        });
        const [announcement, setAnnouncement] = createSignal("", { ownedWrite: true });
        this.instructionsId = createUniqueId();
        this.operation = operation;
        this.announcement = announcement;
        this.#options = options;
        this.locale = locale;
        this.environment = environment;
        this.#setOperation = setOperation;
        this.#setAnnouncement = setAnnouncement;

        // stop scrolling once the manager goes
        onCleanup(() => this.#stopScroll());
    }

    /** Add a draggable until its owner goes. */
    addDraggable(draggable: Draggable): void {
        this.#draggables.set(draggable.id, draggable);
        onCleanup(() => {
            if (this.#draggables.get(draggable.id) === draggable) {
                this.#draggables.delete(draggable.id);
            }
        });
    }

    /** Add a droppable until its owner goes. */
    addDroppable(droppable: Droppable): void {
        this.#droppables.set(droppable.id, droppable);
        onCleanup(() => {
            if (this.#droppables.get(droppable.id) === droppable) {
                this.#droppables.delete(droppable.id);
            }
        });
    }

    /** Read the drag as its last step left it, ahead of the signal's next flush. */
    current(): DragOperation | undefined {
        return this.#current;
    }

    /** Read whether an item is dragged. */
    isDragging(item: string): boolean {
        return this.operation()?.item === item;
    }

    /** Read the target the drag aims at inside a droppable. */
    targetIn(droppable: string): DragTarget | undefined {
        const target = this.operation()?.target;

        return target?.droppable === droppable ? target : undefined;
    }

    /** Lift a draggable where it stands. */
    lift(id: string, kind: DragKind, pointer: PointerType): void {
        // lift a known draggable once
        const draggable = this.#draggables.get(id);
        if (draggable === undefined || this.#current !== undefined) {
            return;
        }

        // start where it stands and measure every droppable that takes it
        const from = draggable.home();
        const operation = {
            item: draggable.item,
            draggable: id,
            kind,
            pointer,
            from,
            grab: from,
            target: from,
            offsetX: 0,
            offsetY: 0,
        };
        this.#set(operation);
        this.#measure(draggable, operation);
        this.#announce("lift", operation);
    }

    /** Follow a pointer to the target under it and scroll near a scroller's edge. */
    follow(x: number, y: number, offsetX: number, offsetY: number): void {
        // aim at the innermost droppable under the pointer
        const operation = this.#current;
        if (operation === undefined || operation.pointer === "keyboard") {
            return;
        }
        const next = { ...operation, offsetX, offsetY };
        const under = this.#measured.find((each) => contains(each.bounds, x, y));
        const target = under?.droppable.targetAt(x, y, next);

        // take the target at the press, before any travel, as where the drag took the item
        const [pressX, pressY] = [x - offsetX, y - offsetY];
        const pressed = this.#measured.find((each) => contains(each.bounds, pressX, pressY));
        const isFirst = operation.offsetX === 0 && operation.offsetY === 0;
        const still = { ...next, offsetX: 0, offsetY: 0 };
        const grab =
            (isFirst ? pressed?.droppable.targetAt(pressX, pressY, still) : undefined) ??
            operation.grab;
        const moved = { ...next, grab, target };
        this.#set(moved);

        // announce each new target the pointer reaches
        const last = operation.target;
        if (target !== undefined && (last === undefined || !isSameTarget(target, last))) {
            this.#announce("move", moved);
        }

        // scroll while the pointer rests near an edge
        this.#pointer = { x, y };
        this.#scroll();
    }

    /** Step a keyboard drag by an arrow key. */
    step(key: string): boolean {
        // read the arrow as a step along the text's flow
        const operation = this.#current;
        const step = stepOf(key, this.locale.direction);
        if (
            operation?.pointer !== "keyboard" ||
            step === undefined ||
            operation.target === undefined
        ) {
            return false;
        }

        // step within the droppable, or cross to its neighbour across its axis
        const target = operation.target;
        const droppable = this.#droppables.get(target.droppable);
        const inside = droppable?.step(target, step);
        const next =
            inside ?? (droppable === undefined ? undefined : this.#cross(droppable, target, step));
        if (next === undefined) {
            return false;
        }
        const moved = { ...operation, target: next };
        this.#set(moved);
        this.#announce("move", moved);

        return true;
    }

    /** Take a key of a drag in progress: drop, cancel with Escape, or step with an arrow. */
    key(event: KeyboardEvent): boolean {
        // leave every key alone while nothing is dragged
        if (this.#current === undefined) {
            return false;
        }
        // drop on Space or Enter
        else if (event.key === " " || event.key === "Enter") {
            this.drop();
        }
        // cancel on Escape
        else if (event.key === "Escape") {
            this.cancel();
        }
        // step on the arrow keys, leaving any other key alone
        else if (!this.step(event.key)) {
            return false;
        }
        event.preventDefault();

        return true;
    }

    /** Drop the dragged item. */
    drop(): void {
        // announce where the item landed
        const operation = this.#current;
        if (operation === undefined) {
            return;
        }
        this.#announce(operation.target === undefined ? "cancel" : "drop", operation);

        // hand on a drop at another target while the drag still stands
        const target = operation.target;
        if (target !== undefined && !isSameTarget(target, operation.from)) {
            this.#options.onDrop({ ...operation, target });
        }
        this.#end();

        // render the owner's move, then give the focus back to the moved item
        if (operation.pointer === "keyboard") {
            flush();
            this.#draggables.get(operation.draggable)?.element()?.focus();
        }
    }

    /** Put the dragged item back where it stood. */
    cancel(): void {
        const operation = this.#current;
        if (operation !== undefined) {
            this.#announce("cancel", operation);
            this.#end();
        }
    }

    /** Find the target one droppable over across a droppable's axis. */
    #cross(droppable: Droppable, target: DragTarget, step: DragStep): DragTarget | undefined {
        // cross only on steps across the droppable's own axis
        const isBlockStep = step === "block-start" || step === "block-end";
        if (droppable.axis === "both" || (droppable.axis === "block") === isBlockStep) {
            return undefined;
        }

        // enter the neighbour in document order within the group
        const operation = this.#current;
        const draggable =
            operation === undefined ? undefined : this.#draggables.get(operation.draggable);
        const siblings = [...this.#droppables.values()]
            .filter(
                (each) =>
                    each.group === droppable.group &&
                    operation !== undefined &&
                    draggable !== undefined &&
                    each.accepts(draggable, operation.kind),
            )
            .toSorted((left, right) => order(left.element(), right.element()));
        const offset = step === "block-end" || step === "inline-end" ? 1 : -1;
        const neighbour = siblings[siblings.indexOf(droppable) + offset];

        return neighbour?.enter(target);
    }

    /** Measure the droppables that take a draggable. */
    #measure(draggable: Draggable, operation: DragOperation): void {
        // read each accepting droppable's layout and bounds once
        const accepting = [...this.#droppables.values()].filter((each) =>
            each.accepts(draggable, operation.kind),
        );
        const measured = accepting.flatMap((droppable) => {
            const element = droppable.element();
            droppable.measure(operation);

            return element === undefined
                ? []
                : [
                      {
                          droppable,
                          bounds: element.getBoundingClientRect(),
                          scroller: scrollerOf(element),
                      },
                  ];
        });

        // order nested droppables before the ones around them
        this.#measured = measured.toSorted((left, right) => {
            // compare only droppables one of which is around the other
            const outer = left.droppable.element();
            const inner = right.droppable.element();
            if (outer === undefined || inner === undefined || outer === inner) {
                return 0;
            } else if (outer.contains(inner)) {
                return 1;
            }

            return inner.contains(outer) ? -1 : 0;
        });
    }

    /** Scroll the scroller under the pointer while it rests near an edge. */
    #scroll(): void {
        // scroll a pointer drag only, once per frame
        const operation = this.#current;
        const pointer = this.#pointer;
        if (
            operation === undefined ||
            operation.pointer === "keyboard" ||
            pointer === undefined ||
            this.#frame !== undefined
        ) {
            return;
        }

        // find the scroller under the pointer and how far into its edges the pointer reaches
        const under = this.#measured.find(
            (each) => each.scroller !== undefined && contains(each.bounds, pointer.x, pointer.y),
        );
        const scroller = under?.scroller;
        if (scroller === undefined) {
            return;
        }
        const bounds = scroller.getBoundingClientRect();
        const deltaX = edgeSpeed(pointer.x, bounds.left, bounds.right);
        const deltaY = edgeSpeed(pointer.y, bounds.top, bounds.bottom);
        if (deltaX === 0 && deltaY === 0) {
            return;
        }

        // scroll on the next frame unless the drag ends first
        const frame = new AbortController();
        this.#frame = frame;
        const step = () => {
            // scroll by the edge's speed
            this.#frame = undefined;
            scroller.scrollBy(deltaX, deltaY);

            // measure the moved droppables and aim again from the latest pointer
            const draggable = this.#draggables.get(operation.draggable);
            const current = this.#current;
            const latest = this.#pointer;
            if (draggable !== undefined && current !== undefined && latest !== undefined) {
                this.#measure(draggable, current);
                this.follow(latest.x, latest.y, current.offsetX, current.offsetY);
            }
        };

        // wait for the frame
        void this.environment.display.frame(frame.signal).then(step, (error: unknown) => {
            if (!frame.signal.aborted) {
                throw error;
            }
        });
    }

    /** End the drag and its scrolling. */
    #end(): void {
        // clear the drag, its measures and its scrolling
        this.#set(undefined);
        this.#measured = [];
        this.#pointer = undefined;
        this.#stopScroll();
    }

    /** Stop a pending autoscroll frame. */
    #stopScroll(): void {
        this.#frame?.abort();
        this.#frame = undefined;
    }

    /** Replace the drag. */
    #set(operation: DragOperation | undefined): void {
        this.#current = operation;
        this.#setOperation(operation);
    }

    /** Write the live region's announcement of a drag's step in the reader's language. */
    #announce(step: "lift" | "move" | "drop" | "cancel", operation: DragOperation): void {
        // read the item's name and where it stands
        const item = this.#draggables.get(operation.draggable)?.label() ?? operation.item;
        const target = step === "cancel" ? operation.from : operation.target;
        const droppable = target === undefined ? undefined : this.#droppables.get(target.droppable);
        const place = target === undefined ? undefined : droppable?.describe(target, operation);
        this.#setAnnouncement(this.locale.render(announcementOf(step, item, place)));
    }
}

/** Write the announcement of a drag's step at a place, or over no place outside every droppable. */
function announcementOf(
    step: "lift" | "move" | "drop" | "cancel",
    item: string,
    place: DragPlace | undefined,
): Message {
    // announce a position in a list
    if (place?.kind === "position") {
        const { position, count, list } = place;
        if (step === "lift") {
            return t`Picked up ${item}, position ${position} of ${count} in ${list}.`;
        } else if (step === "move") {
            return t`${item} moved to position ${position} of ${count} in ${list}.`;
        } else if (step === "drop") {
            return t`${item} dropped at position ${position} of ${count} in ${list}.`;
        }

        return t`Moving ${item} was cancelled. It returned to position ${position} of ${count} in ${list}.`;
    }
    // announce a labeled place
    else if (place?.kind === "label") {
        const { label } = place;
        if (step === "lift") {
            return t`Picked up ${item}, ${label}.`;
        } else if (step === "move") {
            return t`${item} moved to ${label}.`;
        } else if (step === "drop") {
            return t`${item} dropped at ${label}.`;
        }

        return t`Moving ${item} was cancelled. It returned to ${label}.`;
    }

    // announce no place while the item is outside every droppable
    return t`${item} is over no place to drop.`;
}

/** Read an arrow key as a step along the text's flow, left and right mirrored in right-to-left. */
function stepOf(key: string, direction: Direction): DragStep | undefined {
    // read up and down along the block axis
    if (key === "ArrowUp") {
        return "block-start";
    } else if (key === "ArrowDown") {
        return "block-end";
    }
    // read left and right along the inline axis, by the text's direction
    else if (key === "ArrowLeft") {
        return direction === "rtl" ? "inline-end" : "inline-start";
    } else if (key === "ArrowRight") {
        return direction === "rtl" ? "inline-start" : "inline-end";
    }

    return undefined;
}

/** Report whether two targets are the same place. */
function isSameTarget(left: DragTarget, right: DragTarget): boolean {
    // compare the droppable, then the place within it
    if (left.droppable !== right.droppable) {
        return false;
    } else if (left.kind === "list") {
        return right.kind === "list" && left.index === right.index;
    } else if (left.kind === "cell") {
        return right.kind === "cell" && left.key === right.key;
    }

    return right.kind === "time" && left.at === right.at;
}

/** Report whether a point lies within bounds. */
function contains(bounds: DOMRect, x: number, y: number): boolean {
    return x >= bounds.left && x <= bounds.right && y >= bounds.top && y <= bounds.bottom;
}

/** Compare two elements by their place in the document. */
function order(left: HTMLElement | undefined, right: HTMLElement | undefined): number {
    if (left === undefined || right === undefined || left === right) {
        return 0;
    }

    return left.compareDocumentPosition(right) & Node.DOCUMENT_POSITION_FOLLOWING ? -1 : 1;
}

/** Find the nearest element around one that scrolls. */
function scrollerOf(element: HTMLElement): HTMLElement | undefined {
    // walk up to the first element whose overflow scrolls
    for (let each: HTMLElement | null = element; each !== null; each = each.parentElement) {
        const { overflowX, overflowY } = getComputedStyle(each);
        if (
            [overflowX, overflowY].some((overflow) => overflow === "auto" || overflow === "scroll")
        ) {
            return each;
        }
    }

    return undefined;
}

/** Read the pixels to scroll along an axis, faster the deeper a point reaches into an edge. */
function edgeSpeed(position: number, start: number, end: number): number {
    // scroll back near the start and forward near the end
    if (position < start + SCROLL_EDGE) {
        return -SCROLL_SPEED * (1 - Math.max(position - start, 0) / SCROLL_EDGE);
    } else if (position > end - SCROLL_EDGE) {
        return SCROLL_SPEED * (1 - Math.max(end - position, 0) / SCROLL_EDGE);
    }

    return 0;
}
