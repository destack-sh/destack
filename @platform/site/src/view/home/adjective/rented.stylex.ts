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

/** The colours of the notes app's database, beside the page colours fig. 1 shares. */
export const notionTable = style.defineConsts({
    /** The sidebar. */
    sidebar: "light-dark(#f8f8f7, #202020)",
    /** Secondary text. */
    muted: "light-dark(#787774, #9b9b9b)",
    /** Rules and borders. */
    border: "light-dark(#e9e9e7, #2f2f2f)",
    /** A hovered or open item, and a grey tag. */
    hover: "light-dark(#efefed, #2c2c2c)",
    /** The open toolbar button. */
    select: "light-dark(#e7f3f8, #1f3a4a)",
    /** A green tag's field. */
    green: "light-dark(#dbeddb, #2b593f)",
    /** A green tag's text. */
    greenInk: "light-dark(#1c3829, #d0e7d6)",
});
