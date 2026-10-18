import * as style from "@destack/style";

/** The colours of the visual collaboration board, light and dark as it ships. */
export const mural = style.defineConsts({
    /** The canvas. */
    canvas: "light-dark(#f4f4f6, #1d1d21)",
    /** The top bar, the toolbar and an area's body. */
    panel: "light-dark(#ffffff, #2a2a30)",
    /** The text. */
    ink: "light-dark(#1d1d1f, #f2f2f4)",
    /** Secondary text and icons. */
    muted: "light-dark(#6e6e78, #a7a7b2)",
    /** Rules and an area's edge. */
    line: "light-dark(#e2e2e8, #3a3a42)",
    /** The brand pink of the logo and the share button. */
    pink: "#ff0b47",
    /** An area's title bar. */
    areaHead: "light-dark(#e9e4ff, #2e2848)",
    /** A yellow sticky note. */
    yellow: "#fcfe7d",
    /** A green sticky note. */
    green: "#c4f8b0",
    /** A blue sticky note. */
    blue: "#b4f2ff",
    /** A pink sticky note. */
    rose: "#ffc0e0",
    /** A sticky note's text, dark on every colour. */
    stickyInk: "#1d1d1f",
    /** A sticky note's lift. */
    stickyLift: "0 1px 3px rgb(0 0 0 / 0.15)",
    /** A floating panel's lift. */
    lift: "0 2px 8px rgb(0 0 0 / 0.1)",
    /** Text on a filled accent. */
    onAccent: "#ffffff",
    /** The typeface the app sets, or the nearest one a machine has. */
    face: 'Proxima Nova, -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, sans-serif',
});
