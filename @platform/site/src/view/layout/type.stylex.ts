import * as style from "@destack/style";

/** The site's font sizes, from figure labels up to the closing definition. */
export const typeScale = style.defineConsts({
    /** The size of uppercase code labels, such as figure numbers and column heads. */
    label: "0.6875rem",

    /** The size of figure contents, notes and menus. */
    small: "0.8125rem",

    /** The size of running text, captions and navigation. */
    body: "0.9375rem",

    /** The size of an entry's headword, leading the line above its definition. */
    headword: "clamp(1.375rem, 2vw, 1.75rem)",

    /** The size of the sentence under a definition, and of page descriptions. */
    lead: "clamp(1rem, 1.3vw, 1.1875rem)",

    /** The size of definitions and page titles. */
    display: "clamp(2rem, 3.3vw, 3rem)",

    /** The size of prices and the phone menu's links, a step under the definitions. */
    title: "clamp(1.75rem, 3vw, 2.375rem)",

    /** The size of the footer's closing definition, on a row of its own above the headword. */
    closing: "clamp(2.5rem, 5.8vw, 5rem)",
});

/** The site's two letter spacings, for uppercase labels and display text. */
export const tracking = style.defineConsts({
    /** The spacing of uppercase code labels. */
    label: "0.1em",

    /** The spacing of display text. */
    display: "-0.02em",
});
