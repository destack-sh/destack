import * as style from "@destack/style";

/** The colours of the workspace chat app, light and dark as it ships. */
export const googleChat = style.defineConsts({
    /** The app's frame around the conversation. */
    frame: "light-dark(#f6f8fc, #1b1b1b)",
    /** The conversation's card. */
    surface: "light-dark(#ffffff, #1f1f1f)",
    /** The text. */
    ink: "light-dark(#1f1f1f, #e3e3e3)",
    /** Secondary text and icons. */
    muted: "light-dark(#444746, #c4c7c5)",
    /** Rules and outlines. */
    line: "light-dark(#c4c7c5, #444746)",
    /** The search field and the composer. */
    field: "light-dark(#e9eef6, #282a2c)",
    /** The chosen space and the new chat button. */
    selected: "light-dark(#d3e3fd, #004a77)",
    /** The new chat button. */
    button: "light-dark(#c2e7ff, #004a77)",
    /** Links and thread counts. */
    link: "light-dark(#0b57d0, #a8c7fa)",
    /** The active status dot. */
    active: "#1e8e3e",
    /** Text on a filled accent. */
    onAccent: "#ffffff",
    /** The typeface the app sets, or the nearest one a machine has. */
    face: '"Google Sans Text", "Google Sans", Roboto, -apple-system, BlinkMacSystemFont, sans-serif',
    /** The typeface of the app's name and headings. */
    display: '"Google Sans", "Product Sans", Roboto, -apple-system, BlinkMacSystemFont, sans-serif',
});
