import * as style from "@destack/style";

/** The colours of the office mail app in a browser, light and dark as it ships. */
export const outlook = style.defineConsts({
    /** The header bar, the same grey as the frame in the new Outlook. */
    header: "light-dark(#f0f0f0, #0a0a0a)",
    /** The app's frame, behind its cards. */
    frame: "light-dark(#f0f0f0, #0a0a0a)",
    /** The ribbon, the message list and the reading pane. */
    surface: "light-dark(#ffffff, #292929)",
    /** The text. */
    ink: "light-dark(#242424, #ffffff)",
    /** Secondary text and icons. */
    muted: "light-dark(#616161, #adadad)",
    /** Rules. */
    line: "light-dark(#e0e0e0, #3d3d3d)",
    /** The brand blue of links, unread marks and buttons. */
    blue: "light-dark(#0f6cbd, #479ef5)",
    /** The chosen folder and message. */
    selected: "light-dark(#ebf3fc, #082338)",
    /** The header's search field. */
    search: "light-dark(#ffffff, #1f1f1f)",
    /** The edge of the header's search field. */
    searchLine: "light-dark(#d1d1d1, #3d3d3d)",
    /** The divider inside the New mail button. */
    split: "rgb(255 255 255 / 0.45)",
    /** The red of Report and Flag. */
    red: "light-dark(#c50f1f, #f1707b)",
    /** The purple of Reply, Reply all and Forward. */
    purple: "light-dark(#7160e8, #a79cf1)",
    /** The amber of Quick steps. */
    amber: "light-dark(#e3a21a, #f2c661)",
    /** The green of Archive. */
    green: "light-dark(#107c10, #54b054)",
    /** The warning bar. */
    warning: "light-dark(#fdf3f4, #3b1d21)",
    /** The warning bar's text. */
    warningInk: "light-dark(#b10e1c, #f1707b)",
    /** Text on a filled accent. */
    onAccent: "#ffffff",
    /** The typeface the app sets, or the nearest one a machine has. */
    face: '"Segoe UI Variable Text", "Segoe UI", -apple-system, BlinkMacSystemFont, Roboto, sans-serif',
});
