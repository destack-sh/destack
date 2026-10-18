import * as style from "@destack/style";

/** The colours of the all-in-one work app, light and dark as it ships. */
export const clickup = style.defineConsts({
    /** The global top bar. */
    bar: "#2a2e34",
    /** The top bar's search field. */
    barField: "#3f444b",
    /** The top bar's quiet text. */
    barMuted: "#adb3bd",
    /** The page. */
    surface: "light-dark(#ffffff, #1e1e1f)",
    /** The sidebar. */
    side: "light-dark(#fafbfc, #252526)",
    /** The text. */
    ink: "light-dark(#292d34, #e6e6e6)",
    /** Secondary text and icons. */
    muted: "light-dark(#656f7d, #a3a3a3)",
    /** Rules. */
    line: "light-dark(#e8eaed, #383838)",
    /** The brand purple. */
    purple: "#7b68ee",
    /** The chosen list in the sidebar. */
    selected: "light-dark(#f0eefd, #2f2a4f)",
    /** The urgent priority's flag. */
    urgent: "#f50000",
    /** The high priority's flag. */
    high: "#ffcc00",
    /** The normal priority's flag. */
    normal: "#6fddff",
    /** The low priority's flag. */
    low: "#d8d8d8",
    /** Text on a filled accent. */
    onAccent: "#ffffff",
    /** The typeface the app sets, or the nearest one a machine has. */
    face: '"Plus Jakarta Sans", -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, sans-serif',
});
