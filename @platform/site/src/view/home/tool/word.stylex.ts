import * as style from "@destack/style";

/** The colours of the word processor in a browser, light and dark as it ships. */
export const word = style.defineConsts({
    /** The title bar and the desk the page lies on. */
    chrome: "light-dark(#f3f3f3, #1f1f1f)",
    /** The ribbon. */
    ribbon: "light-dark(#ffffff, #292929)",
    /** The page. */
    paper: "light-dark(#ffffff, #1f1f1f)",
    /** The text. */
    ink: "light-dark(#242424, #e8e8e8)",
    /** Secondary text and icons. */
    muted: "light-dark(#616161, #adadad)",
    /** Rules. */
    line: "light-dark(#e0e0e0, #3d3d3d)",
    /** The app's blue, on the chosen tab and the share button. */
    blue: "light-dark(#185abd, #6ea8fe)",
    /** The default heading colour of the document's theme. */
    heading: "light-dark(#0f4761, #8fc2e0)",
    /** The search field. */
    field: "light-dark(#ffffff, #141414)",
    /** The ribbon's lift. */
    ribbonLift: "0 1px 2px rgb(0 0 0 / 0.12)",
    /** The page's lift off the desk. */
    paperLift: "0 0 0 1px rgb(0 0 0 / 0.06), 0 2px 6px rgb(0 0 0 / 0.08)",
    /** Text on a filled accent. */
    onAccent: "#ffffff",
    /** The typeface the app sets, or the nearest one a machine has. */
    face: '"Segoe UI Variable Text", "Segoe UI", -apple-system, BlinkMacSystemFont, Roboto, sans-serif',
    /** The typeface of the page's text. */
    text: 'Aptos, Calibri, "Segoe UI", -apple-system, Helvetica, sans-serif',
    /** The typeface of the page's title. */
    display: '"Aptos Display", Aptos, Calibri, "Segoe UI", -apple-system, Helvetica, sans-serif',
});
