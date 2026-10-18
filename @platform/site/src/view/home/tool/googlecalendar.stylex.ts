import * as style from "@destack/style";

/** The colours of the calendar in a browser, light and dark as it ships. */
export const googleCalendar = style.defineConsts({
    /** The app's chrome. */
    chrome: "light-dark(#f8fafd, #1b1b1b)",
    /** The week's grid. */
    surface: "light-dark(#ffffff, #1f1f1f)",
    /** The text. */
    ink: "light-dark(#1f1f1f, #e3e3e3)",
    /** Secondary text and icons. */
    muted: "light-dark(#444746, #c4c7c5)",
    /** The grid's lines. */
    line: "light-dark(#dde3ea, #3c4043)",
    /** Today's date and the now line. */
    blue: "light-dark(#0b57d0, #a8c7fa)",
    /** The create button. */
    create: "light-dark(#ffffff, #2d2f31)",
    /** The create button's lift. */
    lift: "0 1px 3px rgb(60 64 67 / 0.3), 0 4px 8px 3px rgb(60 64 67 / 0.15)",
    /** An event in your own calendar's colour. */
    event: "#039be5",
    /** An event from your apps. */
    sage: "#33b679",
    /** An appointment schedule's slot. */
    slot: "light-dark(#e8f0fe, #1f2f45)",
    /** An appointment schedule slot's edge. */
    slotEdge: "light-dark(#4285f4, #8ab4f8)",
    /** Text on a filled event. */
    onAccent: "#ffffff",
    /** The typeface the app sets, or the nearest one a machine has. */
    face: '"Google Sans", Roboto, -apple-system, BlinkMacSystemFont, "Segoe UI", sans-serif',
});
