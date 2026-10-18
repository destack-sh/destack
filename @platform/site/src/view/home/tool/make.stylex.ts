import * as style from "@destack/style";

/** The colours of the visual automation builder, light and dark as it ships. */
export const make = style.defineConsts({
    /** The side navigation. */
    nav: "#2c0057",
    /** The side navigation's text. */
    navInk: "#d8c6f0",
    /** The canvas. */
    canvas: "light-dark(#f7f7f8, #17151c)",
    /** The canvas's dots. */
    dots: "light-dark(#dedde3, #2f2c38)",
    /** The header and the bottom bar. */
    panel: "light-dark(#ffffff, #221f29)",
    /** The text. */
    ink: "light-dark(#1d0b33, #f1edf7)",
    /** Secondary text and icons. */
    muted: "light-dark(#6b6177, #aaa2b8)",
    /** Rules. */
    line: "light-dark(#e3e0e8, #36323f)",
    /** The brand purple. */
    purple: "#6d00cc",
    /** The webhook module's colour. */
    webhook: "#c73a63",
    /** The links between modules. */
    link: "light-dark(#c7bfd3, #4a4458)",
    /** The scheduling switch's on state. */
    on: "#2fbf71",
    /** A module's lift. */
    lift: "0 2px 6px rgb(29 11 51 / 0.15)",
    /** Text on a filled accent. */
    onAccent: "#ffffff",
    /** The typeface the app sets, or the nearest one a machine has. */
    face: 'Inter, -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, sans-serif',
});
