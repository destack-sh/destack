import * as style from "@destack/style";

/** The colours of the messenger in a browser, light and dark as it ships. */
export const whatsapp = style.defineConsts({
    /** The navigation rail, the panes' headers and the composer bar. */
    bar: "light-dark(#f0f2f5, #202c33)",
    /** The chat list. */
    list: "light-dark(#ffffff, #111b21)",
    /** The conversation's wallpaper. */
    wallpaper: "light-dark(#efeae2, #0b141a)",
    /** The wallpaper's doodles. */
    doodles:
        "url(\"data:image/svg+xml,%3Csvg xmlns='http://www.w3.org/2000/svg' width='120' height='120' fill='none' stroke='%23000' stroke-opacity='.045' stroke-width='1.5'%3E%3Ccircle cx='20' cy='22' r='7'/%3E%3Cpath d='M60 14l4 8 9 1-6 6 2 9-9-4-8 4 2-9-6-6 9-1z'/%3E%3Cpath d='M98 20c4-6 12-2 8 5l-8 8-8-8c-4-7 4-11 8-5z'/%3E%3Crect x='14' y='70' width='16' height='12' rx='3'/%3E%3Cpath d='M58 80c6-10 16-10 22 0'/%3E%3Ccircle cx='100' cy='78' r='4'/%3E%3Cpath d='M40 104l8-8 8 8M86 108h16'/%3E%3C/svg%3E\")",
    /** Your bubbles. */
    outgoing: "light-dark(#d9fdd3, #005c4b)",
    /** Their bubbles and the date chip. */
    incoming: "light-dark(#ffffff, #202c33)",
    /** The text. */
    ink: "light-dark(#111b21, #e9edef)",
    /** Secondary text and icons. */
    muted: "light-dark(#667781, #8696a0)",
    /** Rules. */
    line: "light-dark(#e9edef, #222d34)",
    /** The chosen chat. */
    selected: "light-dark(#f0f2f5, #2a3942)",
    /** The brand green of badges and unread times. */
    green: "#25d366",
    /** The chosen filter's text. */
    filterInk: "light-dark(#008069, #00a884)",
    /** The chosen filter's fill. */
    filter: "light-dark(#d9fdd3, #103529)",
    /** The read ticks. */
    tick: "#53bdeb",
    /** A bubble's lift off the wallpaper. */
    lift: "0 1px 0.5px rgb(11 20 26 / 0.13)",
    /** Text on a filled accent. */
    onAccent: "#ffffff",
    /** The typeface the app sets, or the nearest one a machine has. */
    face: '"Segoe UI", "Helvetica Neue", Helvetica, "Lucida Grande", Arial, sans-serif',
});
