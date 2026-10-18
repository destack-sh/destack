import * as style from "@destack/style";

/** The colours of the spreadsheet database, light and dark as it ships. */
export const airtable = style.defineConsts({
    /** The page and the grid's cells. */
    surface: "light-dark(#ffffff, #1f1f1f)",
    /** The table tabs strip, tinted by the base's colour. */
    tabs: "light-dark(#e3f0ff, #13253d)",
    /** The grid's header row and footer. */
    header: "light-dark(#f4f4f4, #262626)",
    /** The text. */
    ink: "light-dark(#1d1f25, #e8e8e8)",
    /** Secondary text and icons. */
    muted: "light-dark(#616670, #a3a3a3)",
    /** The grid's lines. */
    line: "light-dark(#dfe2e4, #3a3a3a)",
    /** The base's colour, on its icon and the share button. */
    base: "#166ee1",
    /** A scheduled status pill. */
    green: "light-dark(#d1f7c4, #1d3a19)",
    /** A replied status pill. */
    blue: "light-dark(#cfdfff, #182b4f)",
    /** A pitched status pill. */
    gray: "light-dark(#eeeeee, #333333)",
    /** A linked record's chip. */
    link: "light-dark(#e1efff, #182b4f)",
    /** A linked record chip's edge. */
    linkEdge: "light-dark(#c2dcff, #2b4a7a)",
    /** Text on a filled accent. */
    onAccent: "#ffffff",
    /** The typeface the app sets, or the nearest one a machine has. */
    face: '-apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, "Helvetica Neue", sans-serif',
});
