import * as style from "@destack/style";

/** The colours of the automation editor, light and dark as it ships. */
export const zapier = style.defineConsts({
    /** The header and the side panel. */
    surface: "light-dark(#ffffff, #201f1e)",
    /** The canvas behind the steps. */
    canvas: "light-dark(#f6f5f4, #161514)",
    /** The canvas's dots. */
    dots: "light-dark(#dcd8d3, #33302d)",
    /** The text. */
    ink: "light-dark(#2b2358, #f0eeec)",
    /** Secondary text and icons. */
    muted: "light-dark(#686377, #a8a39d)",
    /** Rules and card edges. */
    line: "light-dark(#d7d3cf, #3a3734)",
    /** The brand orange. */
    orange: "#ff4f00",
    /** The chosen step's edge and the chosen tab. */
    purple: "#503ebd",
    /** A step's number badge. */
    badge: "light-dark(#ece9f8, #2f2a4a)",
    /** The on state and a done step. */
    on: "#1aa55c",
    /** A card's lift off the canvas. */
    lift: "0 1px 3px rgb(43 35 88 / 0.12)",
    /** Text on a filled accent. */
    onAccent: "#ffffff",
    /** The typeface the app sets, or the nearest one a machine has. */
    face: 'Inter, -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, sans-serif',
});
