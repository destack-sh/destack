import type { Axis } from "../position/index.ts";
import type { Clock } from "@destack/environment";
import { present } from "@destack/schema";
import { type Accessor, createSignal, onCleanup, type Setter } from "@destack/view";
import { access, type MaybeAccessor } from "@destack/view/primitives/utils";
import type { Orientation } from "../focus/index.ts";
import {
    centerOf,
    type InteractionEvent,
    interactionOf,
    type PointerType,
    pointerTypeOf,
} from "./interaction.ts";

/** The step each arrow key moves by, one unit along its axis. */
const ARROWS: Readonly<Record<string, readonly [number, number]>> = {
    ArrowLeft: [-1, 0],
    ArrowRight: [1, 0],
    ArrowUp: [0, -1],
    ArrowDown: [0, 1],
};

/** The pixels a pointer travels before it moves an element, so a click stays a click. */
export const MOVE_DISTANCE = 4;

/** The pixels a finger may drift while it waits for the delay, as UIKit's long press allows. */
const STILL_TOLERANCE = 10;

/** One step of a move. */
export interface MoveEvent extends InteractionEvent<"movestart" | "move" | "moveend"> {
    /** The horizontal viewport position. */
    readonly clientX: number;
    /** The vertical viewport position. */
    readonly clientY: number;
    /** The horizontal distance since the last step, in pixels or one per arrow key. */
    readonly deltaX: number;
    /** The vertical distance since the last step, in pixels or one per arrow key. */
    readonly deltaY: number;
    /** The horizontal distance since the move started. */
    readonly offsetX: number;
    /** The vertical distance since the move started. */
    readonly offsetY: number;
    /** The horizontal speed over the last step, in pixels per millisecond. */
    readonly velocityX: number;
    /** The vertical speed over the last step, in pixels per millisecond. */
    readonly velocityY: number;
}

/** The handlers and limits of a move. */
export interface MoveOptions {
    /** Handle a move starting. */
    readonly onMoveStart?: (event: MoveEvent) => void;
    /** Handle each step. */
    readonly onMove?: (event: MoveEvent) => void;
    /** Handle the move ending, released or cancelled. */
    readonly onMoveEnd?: (event: MoveEvent) => void;
    /** The axis the move follows. */
    readonly orientation?: MaybeAccessor<Orientation>;
    /** The pixels a pointer travels before the move starts, 4 by default. */
    readonly distance?: number;
    /** The milliseconds a finger stays still before the move starts, scrolling until then. */
    readonly delay?: number;
    /** The arrow keys that move one step each along the followed axes. */
    readonly keys?: readonly string[];
    /** Report whether a pointer may start a move here. */
    readonly canStart?: (event: PointerEvent) => boolean;
    /** Report whether the element takes no moves. */
    readonly isDisabled?: () => boolean;
}

/** A pointer sample. */
interface Sample {
    /** The horizontal position. */
    readonly x: number;
    /** The vertical position. */
    readonly y: number;
    /** The time, in milliseconds. */
    readonly time: number;
}

/** A pointer pressed on a movable element. */
interface MoveRun {
    /** The pointer. */
    readonly pointerId: number;
    /** What moves. */
    readonly pointerType: PointerType;
    /** The element. */
    readonly element: HTMLElement;
    /** Where the pointer pressed. */
    readonly start: Sample;
    /** The latest sample. */
    last: Sample;
    /** The sample before the latest. */
    previous: Sample;
    /** The latest pointer event. */
    event: PointerEvent;
    /** The timer of a finger waiting for the delay. */
    timer: number | undefined;
    /** The listeners that follow the pressed pointer anywhere on the page until it lets go. */
    readonly listening: AbortController;
    /** Whether the move started. */
    isMoving: boolean;
}

/** A pressed pointer or the arrow keys moving an element, after React Aria's useMove. */
export class Move {
    /** Whether the element moves. */
    readonly isMoving: Accessor<boolean>;
    /** The handlers and limits. */
    readonly #options: MoveOptions;
    /** The clock a finger's delay runs on. */
    readonly #clock: Pick<Clock, "setTimeout" | "clearTimeout">;
    /** Replace the moving state. */
    readonly #setMoving: Setter<boolean>;
    /** The pressed pointer. */
    #run: MoveRun | undefined = undefined;
    /** Whether the move takes its pointer from the element's other gestures right now. */
    #isTaking = false;

    /** Create a move with its handlers and limits. */
    constructor(options: MoveOptions, clock: Pick<Clock, "setTimeout" | "clearTimeout">) {
        // start still
        const [isMoving, setMoving] = createSignal(false, { ownedWrite: true });
        this.isMoving = isMoving;
        this.#options = options;
        this.#clock = clock;
        this.#setMoving = setMoving;

        // let go of a pressed pointer once the owner goes
        onCleanup(() => (this.#run === undefined ? undefined : this.#release(this.#run)));
    }

    /** Read the handlers and state a movable element spreads. */
    attributes() {
        const isMoving = this.isMoving;

        return {
            get "data-moving"(): "" | undefined {
                return isMoving() ? "" : undefined;
            },
            onPointerDown: (event: PointerEvent) => this.#pointerDown(event),
            onKeyDown: (event: KeyboardEvent) => this.#keyDown(event),
        };
    }

    /** Hold a primary pointer where the element allows. */
    #pointerDown(event: PointerEvent): void {
        // leave two fingers to another gesture, such as a pinch, unless one moves already
        const pressed = this.#run;
        const isSecond = pressed !== undefined && pressed.pointerId !== event.pointerId;
        if (isSecond && event.pointerType === "touch" && !pressed.isMoving) {
            this.#release(pressed);

            return;
        }

        // leave other buttons, a second pointer and refused starts alone
        const element = event.currentTarget;
        if (
            event.button !== 0 ||
            this.#run !== undefined ||
            this.#options.isDisabled?.() === true ||
            this.#options.canStart?.(event) === false ||
            !(element instanceof HTMLElement)
        ) {
            return;
        }

        // capture the pointer on what it pressed, as a finger does, which leaves its click there
        if (event.target instanceof Element) {
            event.target.setPointerCapture(event.pointerId);
        }

        // sample where it pressed
        const sample = { x: event.clientX, y: event.clientY, time: event.timeStamp };
        const run: MoveRun = {
            pointerId: event.pointerId,
            pointerType: pointerTypeOf(event),
            element,
            start: sample,
            last: sample,
            previous: sample,
            event,
            timer: undefined,
            listening: new AbortController(),
            isMoving: false,
        };
        this.#run = run;

        // follow the pointer on the window as events start out, before any element stops them
        const signal = run.listening.signal;
        const view = present(element.ownerDocument.defaultView, "the moved element's window");
        const options = { signal, capture: true };
        view.addEventListener("pointermove", (each) => this.#pointerMove(each), options);
        view.addEventListener("pointerup", (each) => this.#pointerEnd(each, false), options);
        view.addEventListener("pointercancel", (each) => this.#pointerEnd(each, true), options);

        // select no text on the way
        view.document.addEventListener("selectstart", (select) => select.preventDefault(), {
            signal,
        });

        // take a finger's gesture from the page's scrolling wherever it moves the element
        if (run.pointerType === "touch") {
            const claim = (touch: TouchEvent) => {
                if (this.#claims(run)) {
                    touch.preventDefault();
                }
            };
            element.addEventListener("touchmove", claim, { passive: false, signal });
        }

        // start a finger's move once it rested for the delay
        const delay = this.#options.delay;
        if (run.pointerType === "touch" && delay !== undefined) {
            run.timer = this.#clock.setTimeout(() => {
                run.timer = undefined;
                this.#start(run);
            }, delay);
        }
    }

    /** Follow the pressed pointer. */
    #pointerMove(event: PointerEvent): void {
        // follow only the pressed pointer
        const run = this.#run;
        if (run?.pointerId !== event.pointerId) {
            return;
        }
        run.previous = run.last;
        run.last = { x: event.clientX, y: event.clientY, time: event.timeStamp };
        run.event = event;

        // report a step of a move under way
        if (run.isMoving) {
            this.#options.onMove?.(this.#stepOf("move", run, run.previous, false));

            return;
        }

        // measure the travel along the followed axes
        const travel = Math.hypot(
            this.#along("horizontal", run.last.x - run.start.x),
            this.#along("vertical", run.last.y - run.start.y),
        );

        // let go of a finger that drifts before its delay passed
        if (run.timer !== undefined) {
            if (travel > STILL_TOLERANCE) {
                this.#release(run);
            }
        }
        // start past the distance
        else if (travel >= (this.#options.distance ?? MOVE_DISTANCE)) {
            this.#start(run);
            this.#options.onMove?.(this.#stepOf("move", run, run.previous, false));
        }
    }

    /** End the move as the pointer lets go or the browser takes it over. */
    #pointerEnd(event: PointerEvent, isCancelled: boolean): void {
        // end only the pressed pointer
        const run = this.#run;
        if (run?.pointerId !== event.pointerId || this.#isTaking) {
            return;
        }
        run.event = event;
        this.#release(run);
        if (run.isMoving) {
            this.#setMoving(false);
            this.#swallowClick(run.element);
            this.#options.onMoveEnd?.(this.#stepOf("moveend", run, run.previous, isCancelled));
        }
    }

    /** Swallow the click the browser sends the captured element after a move ends. */
    #swallowClick(element: HTMLElement): void {
        // stop the next click on its way to the element, and stop listening once this task ends
        element.addEventListener("click", stopClick, { capture: true, once: true });
        this.#clock.setTimeout(() => element.removeEventListener("click", stopClick, true), 0);
    }

    /** Move one whole step on an arrow key along the followed axes. */
    #keyDown(event: KeyboardEvent): void {
        // read the arrow's step
        const element = event.currentTarget;
        const [stepX, stepY] = ARROWS[event.key] ?? [0, 0];
        const deltaX = this.#along("horizontal", stepX);
        const deltaY = this.#along("vertical", stepY);
        if (
            (deltaX === 0 && deltaY === 0) ||
            this.#options.keys?.includes(event.key) === false ||
            this.#options.isDisabled?.() === true ||
            event.target !== element ||
            !(element instanceof HTMLElement)
        ) {
            return;
        }
        event.preventDefault();

        // start, step and end at once
        const center = centerOf(element);
        const step: MoveEvent = {
            ...interactionOf("move", "keyboard", element, event, false),
            clientX: center.x,
            clientY: center.y,
            deltaX,
            deltaY,
            offsetX: deltaX,
            offsetY: deltaY,
            velocityX: 0,
            velocityY: 0,
        };
        this.#options.onMoveStart?.({ ...step, type: "movestart" });
        this.#options.onMove?.(step);
        this.#options.onMoveEnd?.({ ...step, type: "moveend" });
    }

    /** Start the move from where the pointer pressed. */
    #start(run: MoveRun): void {
        // capture the pointer for the move, unless it left with the press
        if (run.element.ownerDocument.contains(run.element)) {
            run.element.setPointerCapture(run.pointerId);
        }

        // take the pointer from the element's other gestures
        const taken = {
            bubbles: true,
            pointerId: run.pointerId,
            pointerType: run.event.pointerType,
        };
        this.#isTaking = true;
        run.element.dispatchEvent(new PointerEvent("pointercancel", taken));
        this.#isTaking = false;

        // start from where the pointer pressed
        run.isMoving = true;
        this.#setMoving(true);
        this.#options.onMoveStart?.(this.#stepOf("movestart", run, run.start, false));
    }

    /** Report whether a finger moves the element. */
    #claims(run: MoveRun): boolean {
        // take a move under way
        if (run.isMoving) {
            return true;
        } else if (this.#options.delay !== undefined) {
            return false;
        }

        // take a finger that travels mostly along a followed axis
        const x = Math.abs(run.last.x - run.start.x);
        const y = Math.abs(run.last.y - run.start.y);
        const followed = access(this.#options.orientation) ?? "both";

        return followed === "both" || (followed === "horizontal" ? x >= y : y >= x);
    }

    /** Let go of the pressed pointer, its timer and its finger's gesture. */
    #release(run: MoveRun): void {
        // stop a pending delay
        if (run.timer !== undefined) {
            this.#clock.clearTimeout(run.timer);
            run.timer = undefined;
        }
        run.listening.abort();
        this.#run = undefined;
    }

    /** Read the step from a sample to the latest. */
    #stepOf(type: MoveEvent["type"], run: MoveRun, from: Sample, isCancelled: boolean): MoveEvent {
        // measure the step and its speed
        const elapsed = Math.max(run.last.time - from.time, 1);
        const deltaX = this.#along("horizontal", run.last.x - from.x);
        const deltaY = this.#along("vertical", run.last.y - from.y);

        return {
            ...interactionOf(type, run.pointerType, run.element, run.event, isCancelled),
            clientX: run.last.x,
            clientY: run.last.y,
            deltaX,
            deltaY,
            offsetX: this.#along("horizontal", run.last.x - run.start.x),
            offsetY: this.#along("vertical", run.last.y - run.start.y),
            velocityX: deltaX / elapsed,
            velocityY: deltaY / elapsed,
        };
    }

    /** Keep a distance along a followed axis. */
    #along(axis: Axis, distance: number): number {
        const followed = access(this.#options.orientation) ?? "both";

        return followed === "both" || followed === axis ? distance : 0;
    }
}

/** Stop a click from reaching the element under it, and the default the browser runs for it. */
function stopClick(click: MouseEvent): void {
    click.preventDefault();
    click.stopPropagation();
}
