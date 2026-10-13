import { flush } from "@destack/view";

/** Apply a state change as one view transition, morphing the elements both states share. */
export function morph(update: () => void): void {
    // apply the change at once where the browser cannot morph, the reader asks for less motion, or nothing that morphs is on screen
    const isStill = window.matchMedia("(prefers-reduced-motion: reduce)").matches;
    const shown = document.querySelector("[data-morph]")?.getBoundingClientRect();
    const isShown = shown !== undefined && shown.bottom > 0 && shown.top < window.innerHeight;
    if (typeof document.startViewTransition !== "function" || isStill || !isShown) {
        update();
        return;
    }

    // capture the page before the change, then render the change at once for the morph's end state
    document.startViewTransition(() => {
        update();
        flush();
    });
}
