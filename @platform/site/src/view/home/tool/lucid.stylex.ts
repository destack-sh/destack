import * as style from "@destack/style";

/** The colours of the brainstorming whiteboard, light and dark as it ships. */
export const lucid = style.defineConsts({
    /** The canvas. */
    canvas: "light-dark(#ffffff, #1b1f2a)",
    /** The canvas's dots. */
    dots: "light-dark(#d9dde6, #343a4a)",
    /** The header, the toolbar and a frame's title. */
    panel: "light-dark(#ffffff, #252a37)",
    /** The text. */
    ink: "light-dark(#282c33, #eef0f4)",
    /** Secondary text and icons. */
    muted: "light-dark(#6f7682, #a5acb9)",
    /** Rules and a frame's edge. */
    line: "light-dark(#dfe3e8, #3a4152)",
    /** The brand orange of the share button. */
    orange: "#f96b13",
    /** A yellow sticky note. */
    yellow: "#fff1a8",
    /** A green sticky note. */
    green: "#c7f0c2",
    /** A blue sticky note. */
    blue: "#cde3ff",
    /** A purple sticky note. */
    purple: "#e3d6ff",
    /** A sticky note's text, dark on every colour. */
    stickyInk: "#282c33",
    /** A sticky note's lift. */
    stickyLift: "0 1px 3px rgb(40 44 51 / 0.18)",
    /** A floating panel's lift. */
    lift: "0 2px 8px rgb(40 44 51 / 0.12)",
    /** Text on a filled accent. */
    onAccent: "#ffffff",
    /** The typeface the app sets, or the nearest one a machine has. */
    face: '"Open Sans", -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, sans-serif',
});
