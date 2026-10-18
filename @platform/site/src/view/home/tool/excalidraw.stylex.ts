import * as style from "@destack/style";

/** The colours of the hand-drawn whiteboard, light and dark as it ships. */
export const excalidraw = style.defineConsts({
    /** The canvas. */
    canvas: "light-dark(#ffffff, #121212)",
    /** A floating island. */
    island: "light-dark(#ffffff, #232329)",
    /** The text and the strokes. */
    ink: "light-dark(#1e1e1e, #e3e3e8)",
    /** Secondary text and icons. */
    muted: "light-dark(#5c5c63, #a0a0aa)",
    /** The brand violet of the chosen tool and the share button. */
    violet: "#6965db",
    /** The chosen tool's fill. */
    chosen: "light-dark(#e0dfff, #403e6a)",
    /** A yellow shape's fill. */
    yellow: "light-dark(#ffec99, #5a4d1b)",
    /** A green shape's fill. */
    green: "light-dark(#b2f2bb, #1f4a29)",
    /** A blue shape's fill. */
    blue: "light-dark(#a5d8ff, #1b3e5a)",
    /** A floating island's lift. */
    lift: "0 0 0 1px rgb(0 0 0 / 0.06), 0 2px 6px rgb(0 0 0 / 0.08)",
    /** Text on a filled accent. */
    onAccent: "#ffffff",
    /** The typeface of the drawing, or the nearest one a machine has. */
    hand: 'Excalifont, Virgil, "Comic Sans MS", "Segoe Print", "Bradley Hand", cursive',
    /** The typeface of the interface. */
    face: 'Assistant, -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, sans-serif',
});
