import * as style from "@destack/style";

/** The colours of the card board, light and dark as it ships. */
export const trello = style.defineConsts({
    /** The board's backdrop. */
    backdrop: "linear-gradient(135deg, #0c66e4 0%, #6e5dc6 100%)",
    /** The app header over the backdrop. */
    header: "rgb(0 0 0 / 0.32)",
    /** The board header over the backdrop. */
    strip: "rgb(0 0 0 / 0.24)",
    /** A quiet button over the backdrop. */
    glass: "rgb(255 255 255 / 0.2)",
    /** A list. */
    list: "light-dark(#f1f2f4, #101204)",
    /** A card. */
    card: "light-dark(#ffffff, #22272b)",
    /** The text. */
    ink: "light-dark(#172b4d, #b6c2cf)",
    /** Secondary text and badges. */
    muted: "light-dark(#44546f, #9fadbc)",
    /** The launch label. */
    green: "#4bce97",
    /** The review label. */
    yellow: "#f5cd47",
    /** The waitlist label. */
    purple: "#9f8fef",
    /** A done card's due badge. */
    done: "#1f845a",
    /** A due-soon card's due badge. */
    soon: "light-dark(#f5cd47, #7f5f01)",
    /** The share button. */
    button: "light-dark(#dcdfe4, #2c333a)",
    /** The share button's text. */
    buttonInk: "light-dark(#172b4d, #b6c2cf)",
    /** A card's lift off its list. */
    lift: "0 1px 1px rgb(9 30 66 / 0.25), 0 0 1px rgb(9 30 66 / 0.31)",
    /** Text on the backdrop. */
    onBackdrop: "#ffffff",
    /** The typeface the app sets, or the nearest one a machine has. */
    face: '-apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, "Noto Sans", Ubuntu, sans-serif',
});
