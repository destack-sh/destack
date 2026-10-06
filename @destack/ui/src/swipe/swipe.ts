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

/** A swipe that dismisses an element toward an edge: its pointer handlers and the distance it travelled. */
export interface Swipe {
    /** The distance the element follows the pointer toward the edge, undefined while no swipe runs. */
    readonly offset: Accessor<number | undefined>;
    /** The translation that moves the element by the offset, as CSS `translate` takes it. */
    readonly translate: Accessor<string | undefined>;
    /** Start a swipe away from controls. */
    readonly onPointerDown: (event: PointerEvent & { readonly currentTarget: HTMLElement }) => void;
    /** Follow the pointer toward the edge. */
    readonly onPointerMove: (event: PointerEvent) => void;
    /** Dismiss past a share of the element or on a fast flick, else spring back. */
    readonly onPointerUp: () => void;
    /** Spring back from an interrupted swipe. */
    readonly onPointerCancel: () => void;
}

/** Follow a pointer swiping an element toward an edge, dismissing it once the swipe goes far or fast enough, after drawers and toasts on touch screens. */
export function createSwipe(
    direction: () => SwipeDirection,
    onDismiss: () => void,
    canStart: (element: HTMLElement) => boolean = () => true,
): Swipe {
    // keep the swipe in progress and its distance toward the edge
    const [offset, setOffset] = createSignal<number | undefined>(undefined, { ownedWrite: true });
    let swipe:
        | { readonly start: number; readonly time: number; readonly extent: number }
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
            const distance = offset();
            if (distance === undefined) {
                return undefined;
            }

            return isVertical() ? `0 ${distance * sign()}px` : `${distance * sign()}px 0`;
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
            swipe = {
                start: isVertical() ? event.clientY : event.clientX,
                time: performance.now(),
                extent: isVertical() ? box.height : box.width,
            };
            event.currentTarget.setPointerCapture(event.pointerId);
        },
        onPointerMove: (event) => {
            // follow the pointer toward the edge only
            if (swipe !== undefined) {
                const travel =
                    ((isVertical() ? event.clientY : event.clientX) - swipe.start) * sign();
                setOffset(Math.max(travel, 0));
            }
        },
        onPointerUp: () => {
            // dismiss past a share of the element or on a fast flick
            if (swipe !== undefined) {
                const travel = offset() ?? 0;
                const speed = travel / Math.max(performance.now() - swipe.time, 1);
                const isFar = travel > Math.max(swipe.extent * DISMISS_SHARE, DISMISS_DISTANCE);
                const isFlick = speed > FLICK_SPEED && travel > FLICK_DISTANCE;
                if (isFar || isFlick) {
                    onDismiss();
                }
            }
            end();
        },
        onPointerCancel: end,
    };
}
