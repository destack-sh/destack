import * as style from "@destack/style";

/** The colours of the chat assistant. */
export const chatgpt = style.defineConsts({
    /** The page background. */
    canvas: "light-dark(#ffffff, #212121)",
    /** The sidebar. */
    sidebar: "light-dark(#f9f9f9, #171717)",
    /** The text. */
    ink: "light-dark(#0d0d0d, #ececec)",
    /** Secondary text. */
    muted: "light-dark(#5d5d5d, #b4b4b4)",
    /** Card borders. */
    border: "light-dark(#e5e5e5, #3a3a3a)",
    /** A failed task's note. */
    error: "light-dark(#d1342f, #ff7b72)",
    /** The brand green. */
    green: "#10a37f",
});

/** The colours the rented tools share or borrow from their own brand marks. */
export const marks = style.defineConsts({
    /** A failed run's annotation field. */
    dangerField: "light-dark(#ffebe9, #3c1618)",
    /** A run's passed green. */
    passed: "light-dark(#1a7f37, #3fb950)",
    /** A skipped run's grey. */
    skipped: "light-dark(#818b98, #6e7681)",
    /** White text on a brand fill. */
    white: "#ffffff",
    /** The code host's failed run red. */
    danger: "light-dark(#d1242f, #f85149)",
});
