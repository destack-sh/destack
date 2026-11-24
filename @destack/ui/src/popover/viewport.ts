import { isMotionReduced } from "@destack/view/motion";
import { GLIDE } from "../position/index.ts";

/** How far content slides as another takes its place, in pixels, as much as a row is tall. */
const SLIDE = 32;

/** The content of a popover as it slides out of the way of another trigger's. */
export class Viewport {
    /** The element whose content slides. */
    readonly #element: HTMLElement;
    /** The content kept, or that still leaves. */
    #leaving: HTMLElement | undefined;
    /** Whether the content kept has not slid yet. */
    #isKept: boolean;

    /** Slide the content of an element. */
    constructor(element: HTMLElement) {
        this.#element = element;
        this.#leaving = undefined;
        this.#isKept = false;
    }

    /** Keep the content as it is, before another takes its place. */
    keep(): void {
        // keep nothing where nothing moves
        const current = this.#element.firstElementChild;
        this.#leaving?.remove();
        this.#leaving = undefined;
        this.#isKept = false;
        if (
            !(current instanceof HTMLElement) ||
            typeof current.animate !== "function" ||
            isMotionReduced(this.#element)
        ) {
            return;
        }

        // copy it as a picture no person reads or reaches, without the new content's ids and names
        const copy = current.cloneNode(true);
        if (copy instanceof HTMLElement) {
            for (const each of [copy, ...copy.querySelectorAll("[id], [name]")]) {
                each.removeAttribute("id");
                each.removeAttribute("name");
            }
            copy.inert = true;
            copy.setAttribute("aria-hidden", "true");
            copy.dataset["slot"] = "popover-viewport-previous";
            Object.assign(copy.style, {
                position: "absolute",
                insetBlockStart: "0",
                insetInlineStart: "0",
                width: `${String(current.offsetWidth)}px`,
                pointerEvents: "none",
            });
            this.#leaving = copy;
            this.#isKept = true;
        }
    }

    /** Slide the content kept out, and the new in, the way from one trigger to the next. */
    slide(from: DOMRect, to: DOMRect): void {
        // slide nothing where nothing was kept
        const leaving = this.#leaving;
        const current = this.#element.firstElementChild;
        if (leaving === undefined || current === null || !this.#isKept) {
            return;
        }
        this.#isKept = false;

        // follow the longer way between the triggers
        const [x, y] = [to.left - from.left, to.top - from.top];
        const isAcross = Math.abs(x) > Math.abs(y);
        const way = (sign: number): string =>
            isAcross
                ? `${String(sign * Math.sign(x) * SLIDE)}px 0px`
                : `0px ${String(sign * Math.sign(y) * SLIDE)}px`;
        this.#element.append(leaving);
        current.animate(
            [
                { translate: way(1), opacity: 0 },
                { translate: "0px 0px", opacity: 1 },
            ],
            GLIDE,
        );
        const out = leaving.animate(
            [
                { translate: "0px 0px", opacity: 1 },
                { translate: way(-1), opacity: 0 },
            ],
            GLIDE,
        );
        out.addEventListener("finish", () => leaving.remove());
        out.addEventListener("cancel", () => leaving.remove());
    }
}
