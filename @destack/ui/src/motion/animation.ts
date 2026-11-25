import { motion } from "@destack/theme/tokens.stylex";
import { createEffect } from "@destack/view";
import { isMotionReduced } from "@destack/view/motion";

/** The id of an element's animation, which a later one replaces where it stands. */
const ANIMATION_ID = "destack-layout";

/** The share of its size an element enters from and leaves to, as it never comes from nothing. */
const FROM_SCALE = 0.95;

/** How long and along which curve something moves. */
export interface LayoutTiming {
    /** The milliseconds it takes. */
    readonly duration: number;
    /** The curve it follows. */
    readonly easing: string;
}

/** The box of an element within the element that places it. */
interface LayoutBox {
    /** The distance from its left. */
    readonly left: number;
    /** The distance from its top. */
    readonly top: number;
    /** The width. */
    readonly width: number;
    /** The height. */
    readonly height: number;
}

/** The motion of an element that changes place, by the theme's times and curves. */
export class LayoutAnimation {
    /** The element. */
    readonly #element: HTMLElement;

    /** Create the motion of an element. */
    constructor(element: HTMLElement) {
        this.#element = element;
    }

    /** Read the theme's timing where the element stands, as the browser resolves it. */
    timing(length: "short" | "medium" = "medium"): LayoutTiming {
        // have a child take it, as a theme may calculate it
        const probe = this.#element.ownerDocument.createElement("span");
        probe.hidden = true;
        probe.style.transitionDuration =
            length === "short" ? motion.durationShort : motion.durationMedium;
        probe.style.transitionTimingFunction = motion.easingStandard;
        this.#element.append(probe);
        const { transitionDuration, transitionTimingFunction } = getComputedStyle(probe);
        probe.remove();

        return {
            duration: (Number.parseFloat(transitionDuration) || 0) * 1000,
            easing: transitionTimingFunction || "ease",
        };
    }

    /** Report whether it moves: the page animates, and the person wants motion. */
    isEnabled(): boolean {
        return typeof this.#element.animate === "function" && !isMotionReduced(this.#element);
    }

    /** Move it from where it was to where it is, taking over a move under way. */
    from(x: number, y: number, length: "short" | "medium" = "medium"): void {
        // replace the move under way, whose place the caller measured
        this.stop();
        if ((x === 0 && y === 0) || !this.isEnabled()) {
            return;
        }
        this.#element.animate(
            [{ translate: `${String(x)}px ${String(y)}px` }, { translate: "0px 0px" }],
            { ...this.timing(length), id: ANIMATION_ID },
        );
    }

    /** Bring it in. */
    enter(): void {
        if (this.isEnabled()) {
            this.#element.animate(
                [
                    { opacity: 0, scale: FROM_SCALE },
                    { opacity: 1, scale: 1 },
                ],
                { ...this.timing("short"), id: ANIMATION_ID },
            );
        }
    }

    /** Let a picture of it leave, once it is gone, from where it was in another. */
    leave(around: HTMLElement, box: LayoutBox): void {
        // draw a picture nothing reads or reaches
        const animation = new LayoutAnimation(around);
        const picture = this.#element.cloneNode(true);
        if (!(picture instanceof HTMLElement) || !animation.isEnabled()) {
            return;
        }
        for (const each of [picture, ...picture.querySelectorAll("[id], [role]")]) {
            each.removeAttribute("id");
            each.removeAttribute("role");
        }
        picture.inert = true;
        picture.setAttribute("aria-hidden", "true");

        // float over the page, so that it takes no room in what scrolls
        const corner = around.getBoundingClientRect();
        Object.assign(picture.style, {
            position: "fixed",
            margin: "0",
            left: `${String(corner.left + box.left - around.scrollLeft)}px`,
            top: `${String(corner.top + box.top - around.scrollTop)}px`,
            width: `${String(box.width)}px`,
            height: `${String(box.height)}px`,
            pointerEvents: "none",
        });
        around.append(picture);
        const leaving = picture.animate(
            [
                { opacity: 1, scale: 1 },
                { opacity: 0, scale: FROM_SCALE },
            ],
            animation.timing("short"),
        );
        leaving.addEventListener("finish", () => picture.remove());
        leaving.addEventListener("cancel", () => picture.remove());
    }

    /** Stop its move. */
    stop(): void {
        if (typeof this.#element.getAnimations === "function") {
            this.#element
                .getAnimations()
                .filter(({ id }) => id === ANIMATION_ID)
                .forEach((each) => each.cancel());
        }
    }
}

/** Move elements to their new places as they change, the new in and the gone out. */
export function createLayoutAnimation(options: {
    /** Read the elements in order. */
    readonly elements: () => readonly HTMLElement[];
    /** Read the element around them, whose corner their places count from. */
    readonly around: () => HTMLElement | undefined;
}): void {
    // remember where each is
    let boxes = new Map<HTMLElement, LayoutBox>();
    createEffect(options.elements, (elements, before) => {
        // move those that are elsewhere than they were, a hidden one nowhere
        const next = boxesOf(elements);
        const around = options.around();
        const isFirst = before === undefined || before.length === 0;
        for (const [element, box] of next) {
            const was = boxes.get(element);
            if (was === undefined && !isFirst && box.width > 0) {
                new LayoutAnimation(element).enter();
            } else if (was !== undefined && was.width > 0 && box.width > 0) {
                new LayoutAnimation(element).from(was.left - box.left, was.top - box.top, "short");
            }
        }

        // let those that are gone leave
        for (const [element, box] of boxes) {
            if (
                !next.has(element) &&
                !element.isConnected &&
                around !== undefined &&
                box.width > 0
            ) {
                new LayoutAnimation(element).leave(around, box);
            }
        }
        boxes = next;

        // follow what moves them without a change of their own
        const resizes = new ResizeObserver(() => {
            boxes = boxesOf(elements);
        });
        elements.forEach((element) => resizes.observe(element));
        if (around !== undefined) {
            resizes.observe(around);
        }

        return () => resizes.disconnect();
    });
}

/** Read the box of each element. */
function boxesOf(elements: readonly HTMLElement[]): Map<HTMLElement, LayoutBox> {
    return new Map(elements.map((element) => [element, boxOf(element)]));
}

/** Read the box of an element within the element that places it. */
function boxOf(element: HTMLElement): LayoutBox {
    return {
        left: element.offsetLeft,
        top: element.offsetTop,
        width: element.offsetWidth,
        height: element.offsetHeight,
    };
}
