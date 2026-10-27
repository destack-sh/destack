import * as style from "@destack/style";

/** The site's five font sizes, from figure labels up to section definitions. */
export const typeScale = style.defineConsts({
    /** The size of uppercase code labels, such as figure numbers and column heads. */
    label: "0.6875rem",

    /** The size of figure contents, notes and menus. */
    small: "0.8125rem",

    /** The size of running text, captions and navigation. */
    body: "0.9375rem",

    /** The size of the sentence under a definition, and of page descriptions. */
    lead: "clamp(1rem, 1.3vw, 1.1875rem)",

    /** The size of definitions, page titles and prices. */
    display: "clamp(1.75rem, 3vw, 2.375rem)",

    /** The size of the footer's closing definition, between display and the headword. */
    closing: "clamp(2.25rem, 3.4vw, 3.25rem)",
});

/** The site's two letter spacings, for uppercase labels and display text. */
export const tracking = style.defineConsts({
    /** The spacing of uppercase code labels. */
    label: "0.1em",

    /** The spacing of display text. */
    display: "-0.02em",
});
