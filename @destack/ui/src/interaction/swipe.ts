import type { Clock } from "@destack/environment";
import * as style from "@destack/style";
import { type Accessor, createSignal, type Setter } from "@destack/view";
import type { Side } from "../position/index.ts";
import { Move, type MoveEvent } from "./move.ts";

/** The share of its own extent an element is swiped past to dismiss on release. */
const DISMISS_SHARE = 0.25;

/** The distance an element is swiped past to dismiss on release however small it is, in pixels. */
const DISMISS_DISTANCE = 48;

/** The speed past which a release dismisses, in pixels per millisecond. */
const FLICK_SPEED = 0.5;

/** The distance a flick travels at least before its speed counts, in pixels. */
const FLICK_DISTANCE = 16;

/** The elements a swipe starting on them leaves to their own handling. */
const INTERACTIVE = "a, button, input, select, textarea, [contenteditable], [role=slider]";

/** The styles of an element following a swipe. */
const styles = style.create({
    swiping: {
        transitionDuration: "0s",
        userSelect: "none",
    },
    offset: (translate: string) => ({ translate }),
});

/** The end of a swipe. */
export interface SwipeEvent extends Omit<MoveEvent, "type"> {
    /** The step. */
    readonly type: "swipeend";
    /** The distance travelled toward the side in pixels, negative away from it. */
    readonly travel: number;
    /** The speed toward the side as the pointer let go, in pixels per millisecond. */
    readonly speed: number;
    /** Whether the swipe dismisses its element. */
    readonly isDismissal: boolean;
}

/** The side and handlers of a swipe. */
export interface SwipeOptions {
    /** Read the side the swipe moves the element toward. */
    readonly side: () => Side;
    /** Handle the swipe ending, released or cancelled. */
    readonly onSwipeEnd?: (event: SwipeEvent) => void;
    /** Report whether a pointer may start a swipe here. */
    readonly canStart?: (event: PointerEvent) => boolean;
}

/** A move that follows an element toward a side and dismisses it. */
export class Swipe {
    /** Whether a swipe is under way. */
    readonly isSwiping: Accessor<boolean>;
    /** The distance the pointer travelled toward the side, negative away from it. */
    readonly offset: Accessor<number | undefined>;
    /** The side and handlers. */
    readonly #options: SwipeOptions;
    /** The move the swipe follows. */
    readonly #move: Move;
    /** Replace the distance travelled. */
    readonly #setOffset: Setter<number | undefined>;

    /** Create a swipe toward a side with its handlers. */
    constructor(options: SwipeOptions, clock: Pick<Clock, "setTimeout" | "clearTimeout">) {
        // start without a swipe under way
        const [offset, setOffset] = createSignal<number | undefined>(undefined, {
            ownedWrite: true,
        });
        this.isSwiping = () => offset() !== undefined;
        this.offset = offset;
        this.#options = options;
        this.#setOffset = setOffset;

        // follow a pointer from where it pressed
        this.#move = new Move(
            {
                distance: 0,
                keys: [],
                orientation: () => (isVertical(options.side()) ? "vertical" : "horizontal"),
                canStart: (event) => this.#canStart(event),
                onMove: (event) => setOffset(this.#toward(event.offsetX, event.offsetY)),
                onMoveEnd: (event) => this.#end(event),
            },
            clock,
        );
    }

    /** Read the handlers and state a swipeable element spreads. */
    attributes() {
        const move = this.#move.attributes();
        const offset = this.offset;

        return {
            get "data-swiping"(): "" | undefined {
                return offset() === undefined ? undefined : "";
            },
            onPointerDown: move.onPointerDown,
            onPointerMove: move.onPointerMove,
            onPointerUp: move.onPointerUp,
            onPointerCancel: move.onPointerCancel,
        };
    }

    /** Read the StyleX styles that move the element toward the side by the offset. */
    style(): style.Styles {
        // rest without a swipe
        const offset = this.offset();
        if (offset === undefined) {
            return null;
        }

        // move toward the side only
        const side = this.#options.side();
        const toward = Math.max(offset, 0) * (side === "bottom" || side === "right" ? 1 : -1);
        const translate = isVertical(side) ? `0 ${String(toward)}px` : `${String(toward)}px 0`;

        return [styles.swiping, styles.offset(translate)];
    }

    /** Report whether a pointer may start a swipe. */
    #canStart(event: PointerEvent): boolean {
        const target = event.target;
        const isControl = target instanceof Element && target.closest(INTERACTIVE) !== null;

        return !isControl && this.#options.canStart?.(event) !== false;
    }

    /** End the swipe. */
    #end(event: MoveEvent): void {
        // measure the release toward the side against the element's extent
        const side = this.#options.side();
        const bounds = event.target.getBoundingClientRect();
        const extent = isVertical(side) ? bounds.height : bounds.width;
        const travel = this.#toward(event.offsetX, event.offsetY);
        const speed = this.#toward(event.velocityX, event.velocityY);

        // dismiss past a share of the extent, or on a flick
        const isFar = travel > Math.max(extent * DISMISS_SHARE, DISMISS_DISTANCE);
        const isFlick = speed > FLICK_SPEED && travel > FLICK_DISTANCE;
        this.#setOffset(undefined);
        this.#options.onSwipeEnd?.({
            ...event,
            type: "swipeend",
            travel,
            speed,
            isDismissal: !event.isCancelled && (isFar || isFlick),
        });
    }

    /** Read the part of a horizontal and a vertical distance that points toward the side. */
    #toward(x: number, y: number): number {
        const side = this.#options.side();
        const along = isVertical(side) ? y : x;

        return side === "bottom" || side === "right" ? along : -along;
    }
}

/** Report whether a side lies along the vertical axis. */
function isVertical(side: Side): boolean {
    return side === "top" || side === "bottom";
}
