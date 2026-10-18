import * as style from "@destack/style";

/** The colours of the spreadsheet in a browser, light and dark as it ships. */
export const googleSheets = style.defineConsts({
    /** The app's chrome. */
    chrome: "light-dark(#f9fbfd, #1b1b1b)",
    /** The grid's cells. */
    cell: "light-dark(#ffffff, #1e1f20)",
    /** The grid's column and row headers. */
    header: "light-dark(#f8f9fa, #282a2c)",
    /** The text. */
    ink: "light-dark(#1f1f1f, #e3e3e3)",
    /** Secondary text and the toolbar's glyphs. */
    muted: "light-dark(#444746, #c4c7c5)",
    /** The toolbar's field. */
    bar: "light-dark(#edf2fa, #282a2c)",
    /** The grid's lines. */
    line: "light-dark(#e1e1e1, #3c4043)",
    /** The selected cell's edge and the share button's text. */
    blue: "light-dark(#1a73e8, #8ab4f8)",
    /** The selected column's and row's header. */
    selected: "light-dark(#d3e3fd, #004a77)",
    /** The share button. */
    share: "light-dark(#c2e7ff, #004a77)",
    /** The share button's text. */
    shareInk: "light-dark(#001d35, #c2e7ff)",
    /** A call-booked chip. */
    green: "light-dark(#d4edbc, #1e3a1a)",
    /** A replied chip. */
    sky: "light-dark(#bfe1f6, #0f3550)",
    /** A pitched chip. */
    gray: "light-dark(#e6e6e6, #3c4043)",
    /** The chosen sheet tab. */
    tabOn: "light-dark(#e1e9f7, #283a52)",
    /** Text on a filled accent. */
    onAccent: "#ffffff",
    /** The typeface the app sets, or the nearest one a machine has. */
    face: '"Google Sans", Roboto, -apple-system, BlinkMacSystemFont, "Segoe UI", sans-serif',
    /** The typeface of the grid. */
    grid: 'Arial, "Helvetica Neue", Helvetica, sans-serif',
});
