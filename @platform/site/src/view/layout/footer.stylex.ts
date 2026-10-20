import * as style from "@destack/style";

/** The footer command's caret, shown while the command is hovered or focused. */
export const caret = style.defineVars({
    /** The caret's opacity. */
    opacity: "0",
    /** Whether the caret blinks: only while it shows, so a hidden caret costs no frames. */
    play: "paused",
});
