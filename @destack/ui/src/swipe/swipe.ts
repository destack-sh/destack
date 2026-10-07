import * as style from "@destack/style";
import { type Accessor, createSignal } from "@destack/view";

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

/** A swipe that moves an element toward an edge: its pointer handlers and the distance it travelled. */
export interface Swipe {
    /** The distance the pointer travelled toward the edge, negative away from it, undefined while no swipe runs. */
    readonly offset: Accessor<number | undefined>;
    /** The translation that moves the element toward the edge by the offset, as CSS `translate` takes it. */
    readonly translate: Accessor<string | undefined>;
    /** Start a swipe away from controls. */
    readonly onPointerDown: (event: PointerEvent & { readonly currentTarget: HTMLElement }) => void;
    /** Follow the pointer along the swipe's axis. */
    readonly onPointerMove: (event: PointerEvent) => void;
    /** Release the swipe to its handler and spring back. */
    readonly onPointerUp: () => void;
    /** Spring back from an interrupted swipe. */
    readonly onPointerCancel: () => void;
}

/** Report whether a released swipe went past a share of its element or flicked fast enough to dismiss it. */
export function isDismissal(release: SwipeRelease): boolean {
    const isFar = release.travel > Math.max(release.extent * DISMISS_SHARE, DISMISS_DISTANCE);
    const isFlick = release.speed > FLICK_SPEED && release.travel > FLICK_DISTANCE;

    return isFar || isFlick;
}

/** Follow a pointer swiping an element along an edge's axis and hand the release to its handler, after drawers and toasts on touch screens. */
export function createSwipe(
    direction: () => SwipeDirection,
    onRelease: (release: SwipeRelease) => void,
    canStart: (element: HTMLElement) => boolean = () => true,
): Swipe {
    // keep the swipe in progress, its distance toward the edge and its last two pointer samples
    const [offset, setOffset] = createSignal<number | undefined>(undefined, { ownedWrite: true });
    let swipe:
        | {
              readonly start: number;
              readonly extent: number;
              last: Sample;
              previous: Sample;
          }
        | undefined;
    const isVertical = (): boolean => direction() === "up" || direction() === "down";
    const sign = (): number => (direction() === "down" || direction() === "right" ? 1 : -1);
    const end = (): void => {
        swipe = undefined;
        setOffset(undefined);
    };

    return {
        offset,
        translate: () => {
            // move toward the edge only
            const distance = offset();
            if (distance === undefined) {
                return undefined;
            }
            const toward = Math.max(distance, 0) * sign();

            return isVertical() ? `0 ${toward}px` : `${toward}px 0`;
        },
        onPointerDown: (event) => {
            // start away from controls and where the element allows
            const target = event.target;
            if (
                (target instanceof Element && target.closest(INTERACTIVE) !== null) ||
                !canStart(event.currentTarget)
            ) {
                return;
            }
            const box = event.currentTarget.getBoundingClientRect();
            const sample = {
                position: isVertical() ? event.clientY : event.clientX,
                time: performance.now(),
            };
            swipe = {
                start: sample.position,
                extent: isVertical() ? box.height : box.width,
                last: sample,
                previous: sample,
            };
            event.currentTarget.setPointerCapture(event.pointerId);
        },
        onPointerMove: (event) => {
            // follow the pointer along the axis, keeping the last two samples for the speed
            if (swipe !== undefined) {
                const position = isVertical() ? event.clientY : event.clientX;
                swipe.previous = swipe.last;
                swipe.last = { position, time: performance.now() };
                setOffset((position - swipe.start) * sign());
            }
        },
        onPointerUp: () => {
            // hand over the travel and the speed of the last move as the pointer let go
            if (swipe !== undefined) {
                const moved = (swipe.last.position - swipe.previous.position) * sign();
                const elapsed = Math.max(performance.now() - swipe.previous.time, 1);
                onRelease({ travel: offset() ?? 0, speed: moved / elapsed, extent: swipe.extent });
            }
            end();
        },
        onPointerCancel: end,
    };
}

/** A pointer position along the swipe's axis and when it was read. */
interface Sample {
    /** The position, in pixels. */
    readonly position: number;
    /** The time, in milliseconds. */
    readonly time: number;
}
