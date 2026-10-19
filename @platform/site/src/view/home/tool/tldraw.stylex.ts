import * as style from "@destack/style";

/** The colours of the infinite canvas, light and dark as it ships. */
export const tldraw = style.defineConsts({
    /** The canvas. */
    canvas: "light-dark(#f9fafb, #101011)",
    /** A floating panel. */
    panel: "light-dark(#ffffff, #212125)",
    /** The text and the strokes. */
    ink: "light-dark(#1d1d1d, #f2f2f2)",
    /** Secondary text, icons and frame labels. */
    muted: "light-dark(#6b6b6b, #9b9b9f)",
    /** A frame's edge. */
    frame: "light-dark(#c8c8cc, #4a4a50)",
    /** The brand blue of the chosen tool and the share button. */
    blue: "#2f80ed",
    /** A hovered or quiet control's fill. */
    quiet: "light-dark(#f2f3f5, #2c2c31)",
    /** The text on a note. */
    noteInk: "light-dark(#1d1d1d, #f2f2f2)",
    /** A yellow note's fill. */
    yellow: "light-dark(#fed49a, #7a5a1e)",
    /** A green note's fill. */
    green: "light-dark(#a7e3b0, #24512e)",
    /** A blue note's fill. */
    sky: "light-dark(#a9d4f5, #1f4566)",
    /** A floating panel's lift. */
    lift: "0 0 0 1px rgb(0 0 0 / 0.06), 0 1px 3px rgb(0 0 0 / 0.08), 0 8px 24px rgb(0 0 0 / 0.06)",
    /** A note's lift off the canvas. */
    noteLift: "0 1px 2px rgb(0 0 0 / 0.16), 0 4px 10px rgb(0 0 0 / 0.08)",
    /** Text on a filled accent. */
    onAccent: "#ffffff",
    /** The typeface of drawn text, or the nearest one a machine has. */
    hand: '"Shantell Sans", "Comic Sans MS", "Segoe Print", "Bradley Hand", cursive',
    /** The typeface of the interface. */
    face: 'Inter, -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, sans-serif',
});
