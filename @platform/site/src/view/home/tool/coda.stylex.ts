import * as style from "@destack/style";

/** The colours of the doc with tables, light and dark as it ships. */
export const coda = style.defineConsts({
    /** The doc. */
    surface: "light-dark(#ffffff, #1e1e20)",
    /** The page list. */
    side: "light-dark(#f8f8f7, #26262a)",
    /** The text. */
    ink: "light-dark(#1b1b1d, #eeeeef)",
    /** Secondary text and icons. */
    muted: "light-dark(#6b6b70, #a5a5ab)",
    /** Rules and the table's lines. */
    line: "light-dark(#e8e8e6, #3a3a3f)",
    /** The brand red of the share button. */
    red: "#ee5a29",
    /** The chosen page. */
    selected: "light-dark(#eceae6, #34343a)",
    /** A call-booked chip. */
    green: "light-dark(#d7f0dd, #1d3a25)",
    /** A replied chip. */
    blue: "light-dark(#dbe7fb, #1d2e4d)",
    /** A pitched chip. */
    yellow: "light-dark(#fbefcf, #463b14)",
    /** Text on a filled accent. */
    onAccent: "#ffffff",
    /** The typeface the app sets, or the nearest one a machine has. */
    face: 'Inter, -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, sans-serif',
});
