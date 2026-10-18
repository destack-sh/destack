import * as style from "@destack/style";

/** The colours of the fast mail client, light and dark as it ships. */
export const superhuman = style.defineConsts({
    /** The inbox. */
    surface: "light-dark(#ffffff, #141414)",
    /** The compose window. */
    panel: "light-dark(#ffffff, #1f1f1f)",
    /** The text. */
    ink: "light-dark(#1b1b1f, #ececf0)",
    /** Secondary text. */
    muted: "light-dark(#8a8a93, #8f8f99)",
    /** Rules. */
    line: "light-dark(#efeff2, #262629)",
    /** The brand's aubergine, on the logo and the chosen split. */
    plum: "#3d1832",
    /** The chosen split's text and underline. */
    split: "light-dark(#4b1e3e, #f0c4e1)",
    /** The chosen row. */
    selected: "light-dark(#f6f2f6, #241c22)",
    /** The error toast. */
    toast: "#1b1b1f",
    /** The error toast's mark. */
    danger: "#ff6b6b",
    /** The compose window's lift. */
    lift: "0 12px 40px rgb(0 0 0 / 0.18), 0 0 0 1px rgb(0 0 0 / 0.04)",
    /** Text on a filled accent. */
    onAccent: "#ffffff",
    /** The typeface the app sets, or the nearest one a machine has. */
    face: 'Inter, -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, sans-serif',
});
