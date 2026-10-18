import * as style from "@destack/style";

/** The colours of the spreadsheet in a browser, light and dark as it ships. */
export const excel = style.defineConsts({
    /** The title bar and the chrome around the grid. */
    chrome: "light-dark(#f3f3f3, #1f1f1f)",
    /** The ribbon and the grid's cells. */
    surface: "light-dark(#ffffff, #292929)",
    /** The grid's column and row headers. */
    header: "light-dark(#f3f3f3, #333333)",
    /** The text. */
    ink: "light-dark(#242424, #e8e8e8)",
    /** Secondary text and icons. */
    muted: "light-dark(#616161, #adadad)",
    /** Rules and grid lines. */
    line: "light-dark(#e1e1e1, #3d3d3d)",
    /** The app's green, on the selection and the chosen tab. */
    green: "light-dark(#107c41, #3fae6f)",
    /** The selected column's and row's header. */
    selected: "light-dark(#d2e8dc, #1c3a2a)",
    /** The table's header row. */
    tableHead: "#156082",
    /** The table's banded rows. */
    band: "light-dark(#c0e6f5, #173748)",
    /** The share button. */
    share: "#0f6cbd",
    /** The ribbon's lift. */
    lift: "0 1px 2px rgb(0 0 0 / 0.12)",
    /** Text on a filled accent. */
    onAccent: "#ffffff",
    /** The typeface the app sets, or the nearest one a machine has. */
    face: '"Segoe UI Variable Text", "Segoe UI", -apple-system, BlinkMacSystemFont, Roboto, sans-serif',
    /** The typeface of the grid. */
    grid: '"Aptos Narrow", Aptos, Calibri, "Segoe UI", -apple-system, Helvetica, sans-serif',
});
