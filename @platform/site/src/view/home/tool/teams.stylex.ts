import * as style from "@destack/style";

/** The colours of the office chat app, light and dark as it ships. */
export const teams = style.defineConsts({
    /** The title bar and the app bar. */
    frame: "light-dark(#ebebeb, #0a0a0a)",
    /** The teams pane. */
    pane: "light-dark(#f5f5f5, #1f1f1f)",
    /** The channel's conversation behind its posts. */
    canvas: "light-dark(#f5f5f5, #1f1f1f)",
    /** The channel header, a post's card and the search field. */
    surface: "light-dark(#ffffff, #292929)",
    /** The text. */
    ink: "light-dark(#242424, #ffffff)",
    /** Secondary text. */
    muted: "light-dark(#616161, #adadad)",
    /** Rules and card edges. */
    line: "light-dark(#e0e0e0, #3d3d3d)",
    /** The brand purple. */
    brand: "light-dark(#5b5fc7, #7f85f5)",
    /** The brand purple's quiet tint, behind the chosen app and reactions. */
    tint: "light-dark(#e8ebfa, #2f2f4a)",
    /** The available dot. */
    available: "#13a10e",
    /** A post card's lift. */
    lift: "0 1px 2px rgb(0 0 0 / 0.14), 0 0 2px rgb(0 0 0 / 0.12)",
    /** The rule inside the split Meet button. */
    split: "rgb(255 255 255 / 0.35)",
    /** Text on a filled accent. */
    onAccent: "#ffffff",
    /** The typeface the app sets, or the nearest one a machine has. */
    face: '"Segoe UI Variable Text", "Segoe UI", -apple-system, BlinkMacSystemFont, Roboto, sans-serif',
});
