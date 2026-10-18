import * as style from "@destack/style";

/** The colours of the desktop mail client, light and dark as it ships. */
export const thunderbird = style.defineConsts({
    /** The spaces bar and the unified toolbar. */
    chrome: "light-dark(#e6e6eb, #18181b)",
    /** The folder pane. */
    folders: "light-dark(#f0f0f4, #1f1e25)",
    /** The message list and the dialog. */
    surface: "light-dark(#ffffff, #2b2a33)",
    /** The text. */
    ink: "light-dark(#15141a, #fbfbfe)",
    /** Secondary text and icons. */
    muted: "light-dark(#5b5b66, #a8a8b3)",
    /** Rules and card edges. */
    line: "light-dark(#d7d7db, #3a3944)",
    /** The brand blue of buttons, badges and the chosen message. */
    blue: "light-dark(#0061e0, #0df)",
    /** The chosen folder and card. */
    selected: "light-dark(#d9e6fb, #0e3a63)",
    /** The unread dot. */
    unread: "light-dark(#2ac3a2, #54ffbd)",
    /** The error dialog's mark. */
    error: "light-dark(#d70022, #ff848b)",
    /** The quiet fill of the search field. */
    field: "light-dark(#ffffff, #2b2a33)",
    /** A dialog's lift off the window. */
    lift: "0 12px 32px rgb(0 0 0 / 0.25), 0 0 0 1px rgb(0 0 0 / 0.06)",
    /** Text on a filled accent. */
    onAccent: "#ffffff",
    /** The typeface the app sets, or the nearest one a machine has. */
    face: '-apple-system, BlinkMacSystemFont, "Segoe UI", Ubuntu, Cantarell, sans-serif',
});
