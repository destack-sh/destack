import { present } from "@destack/schema";
import { createEffect, createSignal, useEnvironment } from "@destack/view";
import { makeEventListener } from "@destack/view/primitives/event-listener";
import { requestFrame } from "@destack/view/primitives/raf";

/** The reader's place among a page's dictionary entries, counted from one. */
export type Folio = {
    /** The entry the reader is in. */
    index: number;
    /** The number of entries on the page. */
    total: number;
    /** Whether the page's footer has come into view, past every entry. */
    isFooterShown: boolean;
};

/** Follow which dictionary entry the reader has scrolled to, while the page is followed. */
export function createFolio(isFollowed: () => boolean): () => Folio | undefined {
    // hold the environment, whose display only the browser has, and the folio
    const environment = useEnvironment();
    const [folio, setFolio] = createSignal<Folio | undefined>(undefined);

    // follow the entries while the page is followed
    createEffect(isFollowed, (followed) => {
        // follow no entries on a page that is not followed
        const entries = followed ? [...document.querySelectorAll("main header[id]")] : [];
        if (entries.length === 0) {
            setFolio(undefined);
            return;
        }

        // count the entries risen past the screen's middle, as a frame's work
        const display = environment.display;
        const measure = () => {
            // clear the pending frame, then count
            cancel = undefined;
            const middle = window.innerHeight / 2;
            const passed = entries.filter((entry) => entry.getBoundingClientRect().top < middle);
            const footer = present(document.querySelector("footer"), "the page footer");
            const isFooterShown = footer.getBoundingClientRect().top < window.innerHeight;
            setFolio({ index: passed.length, total: entries.length, isFooterShown });
        };

        // measure now, then once a frame while the page scrolls or resizes
        let cancel: (() => void) | undefined;
        const schedule = () => {
            cancel ??= requestFrame(measure, display);
        };
        measure();
        const stopScrolling = makeEventListener(window, "scroll", schedule, { passive: true });
        const stopResizing = makeEventListener(window, "resize", schedule, { passive: true });

        // stop following once the page changes
        return () => {
            cancel?.();
            stopScrolling();
            stopResizing();
        };
    });

    return folio;
}

/** Set a folio number in two digits, such as 02. */
export function folioNumber(index: number): string {
    return String(index).padStart(2, "0");
}
