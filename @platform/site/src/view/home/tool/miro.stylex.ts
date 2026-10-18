import * as style from "@destack/style";

/** The colours of the online whiteboard, light and dark as it ships. */
export const miro = style.defineConsts({
    /** The canvas. */
    canvas: "light-dark(#f2f2f2, #1e1e1e)",
    /** The canvas's dots. */
    dots: "light-dark(#d6d6d6, #3a3a3a)",
    /** A floating panel and a frame. */
    panel: "light-dark(#ffffff, #2b2b2b)",
    /** The text. */
    ink: "light-dark(#050038, #f2f2f2)",
    /** Secondary text and icons. */
    muted: "light-dark(#5f5c80, #a8a8b8)",
    /** A frame's edge. */
    line: "light-dark(#e0e0e0, #3a3a3a)",
    /** The brand yellow of the logo. */
    yellow: "#ffd02f",
    /** The share button. */
    blue: "#4262ff",
    /** A yellow sticky note. */
    stickyYellow: "#fff9b1",
    /** A green sticky note. */
    stickyGreen: "#d5f692",
    /** A blue sticky note. */
    stickyBlue: "#a6ccf5",
    /** A pink sticky note. */
    stickyPink: "#ffcee0",
    /** A sticky note's text, dark on every colour. */
    stickyInk: "#1a1a1a",
    /** A sticky note's lift off its frame. */
    stickyLift: "0 2px 4px rgb(0 0 0 / 0.12)",
    /** A panel's lift off the canvas. */
    lift: "0 2px 8px rgb(5 0 56 / 0.12)",
    /** Text on a filled accent. */
    onAccent: "#ffffff",
    /** The typeface the app sets, or the nearest one a machine has. */
    face: '"Roobert PRO", "Open Sans", -apple-system, BlinkMacSystemFont, "Segoe UI", sans-serif',
});
