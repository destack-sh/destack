import * as style from "@destack/style";

/** The colours of the notes workspace. */
export const notion = style.defineConsts({
    /** The page background. */
    surface: "light-dark(#ffffff, #191919)",
    /** The dark button, light on dark pages. */
    charcoal: "light-dark(#2f2f2f, #f0efed)",
    /** The text. */
    ink: "light-dark(#37352f, #d4d4d2)",
    /** The status tag text. */
    navy: "light-dark(#183347, #d3e5ef)",
    /** The status dot. */
    blue: "#337ea9",
    /** Checked boxes and the publish button. */
    link: "#2383e2",
    /** The text on the publish button. */
    onLink: "#ffffff",
    /** The status tag. */
    sky: "light-dark(#d3e5ef, #28456c)",
    /** The callout. */
    sand: "light-dark(#f1f1ef, #2a2a2a)",
    /** The cover's light paint. */
    paper: "#e8d5b5",
    /** The cover's middle paint. */
    tan: "#d9a679",
    /** The cover's dark paint. */
    clay: "#b46d4e",
    /** Secondary text, icons and column heads. */
    muted: "light-dark(#787774, #9b9b9b)",
    /** The table's lines. */
    line: "light-dark(#e9e9e7, #2f2f2f)",
    /** A call-booked tag. */
    green: "light-dark(#dbeddb, #2b593f)",
    /** A call-booked tag's text. */
    greenInk: "light-dark(#1c3829, #dbeddb)",
    /** A pitched tag. */
    yellow: "light-dark(#fdecc8, #89632a)",
    /** A pitched tag's text. */
    yellowInk: "light-dark(#402c1b, #fdecc8)",
    /** The typeface the app sets in databases, or the nearest one a machine has. */
    sans: 'ui-sans-serif, -apple-system, BlinkMacSystemFont, "Segoe UI", Helvetica, sans-serif',
    /** The typeface the site sets, or the nearest one a machine has. */
    face: '"Lyon-Text", Georgia, ui-serif, "Times New Roman", serif',
});
