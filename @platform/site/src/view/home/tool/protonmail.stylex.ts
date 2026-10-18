import * as style from "@destack/style";

/** The colours of the private mail app, light and dark as it ships. */
export const protonMail = style.defineConsts({
    /** The sidebar and the header. */
    chrome: "light-dark(#f5f4f2, #16141c)",
    /** The message list and the composer. */
    surface: "light-dark(#ffffff, #1c1b24)",
    /** A read message's row. */
    read: "light-dark(#f5f4f2, #16141c)",
    /** The text. */
    ink: "light-dark(#0c0c14, #ffffff)",
    /** Secondary text and icons. */
    muted: "light-dark(#5c5958, #a7a4b5)",
    /** Rules and field edges. */
    line: "light-dark(#eae7e4, #2e2b3a)",
    /** The brand purple. */
    purple: "#6d4aff",
    /** The chosen folder and a recipient. */
    selected: "light-dark(#e6e1fd, #2f2a4a)",
    /** The quiet field behind search. */
    field: "light-dark(#eae7e4, #2e2b3a)",
    /** The error text. */
    danger: "light-dark(#dc3251, #f5385a)",
    /** The error banner's fill. */
    dangerTint: "light-dark(#fdeef1, #3a1520)",
    /** The composer's lift off the list. */
    lift: "0 8px 32px rgb(12 12 20 / 0.24)",
    /** Text on a filled accent. */
    onAccent: "#ffffff",
    /** The typeface the app sets, or the nearest one a machine has. */
    face: 'Inter, -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, sans-serif',
});
