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
    /** Each entry's top edge below the screen's top, in CSS pixels. */
    tops: readonly number[];
    /** How far below an entry's top its headword's middle sits, in CSS pixels. */
    lead: number;
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
            const tops = entries.map((entry) => entry.getBoundingClientRect().top);
            const passed = tops.filter((top) => top < middle);

            // measure how far the first entry's headword sits below its top
            const first = present(entries[0], "the page's first entry");
            const word = present(
                first.querySelector("b"),
                "the first entry's headword",
            ).getBoundingClientRect();
            const lead = word.top + word.height / 2 - first.getBoundingClientRect().top;
            setFolio({ index: passed.length, total: entries.length, tops, lead });
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
