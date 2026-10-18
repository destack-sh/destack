import * as style from "@destack/style";

/** The colours of the design whiteboard, light and dark as it ships. */
export const figjam = style.defineConsts({
    /** The canvas. */
    canvas: "light-dark(#f5f5f5, #1e1e1e)",
    /** A floating bar and a section's body. */
    panel: "light-dark(#ffffff, #2c2c2c)",
    /** The file menu's pill. */
    pill: "#1e1e1e",
    /** The text. */
    ink: "light-dark(#1e1e1e, #f5f5f5)",
    /** Secondary text and icons. */
    muted: "light-dark(#757575, #b3b3b3)",
    /** The share button. */
    blue: "#0d99ff",
    /** A section's purple title tab. */
    violet: "#9747ff",
    /** A section's fill. */
    sectionFill: "light-dark(#f3ebff, #2d2440)",
    /** A yellow sticky. */
    yellow: "#ffe299",
    /** A green sticky. */
    green: "#b3efbd",
    /** A blue sticky. */
    sky: "#bde3ff",
    /** A pink sticky. */
    pink: "#ffc7c2",
    /** A sticky's text, dark on every colour. */
    stickyInk: "#1e1e1e",
    /** A sticky's lift. */
    stickyLift: "0 2px 4px rgb(0 0 0 / 0.1)",
    /** A floating bar's lift. */
    lift: "0 2px 12px rgb(0 0 0 / 0.12)",
    /** The logo's four colours. */
    logo: "conic-gradient(#f24e1e 0 25%, #a259ff 0 50%, #1abcfe 0 75%, #0acf83 0)",
    /** Text on a filled accent. */
    onAccent: "#ffffff",
    /** The typeface the app sets, or the nearest one a machine has. */
    face: 'Inter, -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, sans-serif',
});
