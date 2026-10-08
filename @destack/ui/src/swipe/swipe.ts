import * as style from "@destack/style";
import { type Accessor, createSignal, type Setter } from "@destack/view";

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

/** Return the StyleX styles that move an element with a swipe's translation, none while no swipe runs. */
export function swipeStyle(translate: string | undefined): style.Styles {
    return translate === undefined ? null : [styles.swiping, styles.offset(translate)];
}

/** The direction a swipe dismisses toward. */
export type SwipeDirection = "up" | "down" | "left" | "right";

/** A released swipe: how far and fast it went toward the edge, and the extent of the swiped element. */
export interface SwipeRelease {
    /** The distance travelled toward the edge in pixels, negative away from it. */
    readonly travel: number;
    /** The speed toward the edge as the pointer let go in pixels per millisecond, negative away from it. */
    readonly speed: number;
    /** The element's extent along the swipe, in pixels. */
    readonly extent: number;
}

/** Report whether a released swipe went past a share of its element or flicked fast enough to dismiss it. */
export function isDismissal(release: SwipeRelease): boolean {
    const isFar = release.travel > Math.max(release.extent * DISMISS_SHARE, DISMISS_DISTANCE);
    const isFlick = release.speed > FLICK_SPEED && release.travel > FLICK_DISTANCE;

    return isFar || isFlick;
}

/** A pointer swiping an element along an edge's axis, which hands its release to a handler, after drawers and toasts on touch screens. */
export class Swipe {
    /** The distance the pointer travelled toward the edge, negative away from it, undefined while no swipe runs. */
    readonly offset: Accessor<number | undefined>;
    /** The direction the swipe moves the element toward. */
    readonly #direction: () => SwipeDirection;
    /** Handle a released swipe. */
    readonly #onRelease: (release: SwipeRelease) => void;
    /** Report whether a swipe may start on the element. */
    readonly #canStart: (element: HTMLElement) => boolean;
    /** Replace the distance travelled. */
    readonly #setOffset: Setter<number | undefined>;
    /** The swipe in progress: where it started, the element's extent and the last two pointer samples. */
    #run: SwipeRun | undefined = undefined;

    /** Create a swipe toward a direction, starting wherever the element allows. */
    constructor(
        direction: () => SwipeDirection,
        onRelease: (release: SwipeRelease) => void,
        canStart: (element: HTMLElement) => boolean = () => true,
    ) {
        // start without a swipe in progress
        const [offset, setOffset] = createSignal<number | undefined>(undefined, {
            ownedWrite: true,
        });
        this.offset = offset;
        this.#direction = direction;
        this.#onRelease = onRelease;
        this.#canStart = canStart;
        this.#setOffset = setOffset;
    }

    /** The translation that moves the element toward the edge by the offset, as CSS `translate` takes it. */
    translate(): string | undefined {
        // move toward the edge only
        const distance = this.offset();
        if (distance === undefined) {
            return undefined;
        }
        const toward = Math.max(distance, 0) * this.#sign();

        return this.#isVertical() ? `0 ${String(toward)}px` : `${String(toward)}px 0`;
    }

    /** Start a swipe away from controls and where the element allows. */
    start(event: PointerEvent & { readonly currentTarget: HTMLElement }): void {
        // leave a press on a control to the control
        const target = event.target;
        if (
            (target instanceof Element && target.closest(INTERACTIVE) !== null) ||
            !this.#canStart(event.currentTarget)
        ) {
            return;
        }

        // sample the pointer and the element's extent along the axis
        const box = event.currentTarget.getBoundingClientRect();
        const sample = { position: this.#position(event), time: performance.now() };
        this.#run = {
            start: sample.position,
            extent: this.#isVertical() ? box.height : box.width,
            last: sample,
            previous: sample,
        };
        event.currentTarget.setPointerCapture(event.pointerId);
    }

    /** Follow the pointer along the axis, keeping the last two samples for the speed. */
    follow(event: PointerEvent): void {
        const run = this.#run;
        if (run !== undefined) {
            const position = this.#position(event);
            run.previous = run.last;
            run.last = { position, time: performance.now() };
            this.#setOffset((position - run.start) * this.#sign());
        }
    }

    /** Hand the travel and the speed of the last move to the handler as the pointer lets go, and spring back. */
    release(): void {
        const run = this.#run;
        if (run !== undefined) {
            const moved = (run.last.position - run.previous.position) * this.#sign();
            const elapsed = Math.max(performance.now() - run.previous.time, 1);
            this.#onRelease({
                travel: this.offset() ?? 0,
                speed: moved / elapsed,
                extent: run.extent,
            });
        }
        this.cancel();
    }

    /** Spring back from an interrupted swipe. */
    cancel(): void {
        this.#run = undefined;
        this.#setOffset(undefined);
    }

    /** Report whether the swipe runs along the vertical axis. */
    #isVertical(): boolean {
        const direction = this.#direction();

        return direction === "up" || direction === "down";
    }

    /** Read the sign that turns a move along the axis into travel toward the edge. */
    #sign(): number {
        const direction = this.#direction();

        return direction === "down" || direction === "right" ? 1 : -1;
    }

    /** Read a pointer's position along the axis. */
    #position(event: PointerEvent): number {
        return this.#isVertical() ? event.clientY : event.clientX;
    }
}

/** A swipe in progress. */
interface SwipeRun {
    /** The position it started at, in pixels. */
    readonly start: number;
    /** The element's extent along the axis, in pixels. */
    readonly extent: number;
    /** The latest pointer sample. */
    last: Sample;
    /** The pointer sample before the latest. */
    previous: Sample;
}

/** A pointer position along the swipe's axis and when it was read. */
interface Sample {
    /** The position, in pixels. */
    readonly position: number;
    /** The time, in milliseconds. */
    readonly time: number;
}
