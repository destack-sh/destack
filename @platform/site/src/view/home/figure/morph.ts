import { flush } from "@destack/view";

/** Apply a state change as one view transition, morphing the elements both states share. */
export function morph(update: () => void): void {
    // apply the change at once where the browser cannot morph, the reader asks for less motion, or nothing that morphs is on screen
    const isStill = window.matchMedia("(prefers-reduced-motion: reduce)").matches;
    const isShown = [...document.querySelectorAll("[data-morph]")].some((element) => {
        const bounds = element.getBoundingClientRect();

        return bounds.bottom > 0 && bounds.top < window.innerHeight;
    });
    if (typeof document.startViewTransition !== "function" || isStill || !isShown) {
        update();
        return;
    }

    // capture the page before the change, then render the change at once for the morph's end state
    const root = document.documentElement;
    root.dataset["morphing"] = "";
    const transition = document.startViewTransition(() => {
        update();
        flush();
    });

    // mark the page while it morphs, so effects that draw every frame can rest until it settles
    void transition.finished.finally(() => {
        delete root.dataset["morphing"];
    });
}
