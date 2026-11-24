import * as style from "@destack/style";

/** The backdrop of a dialog. */
export const scrim = style.defineVars({
    /** Its opacity, which follows the dialog. */
    opacity: "0",
    /** The share of its opacity a drag of the dialog leaves. */
    share: "1",
    /** How long it fades for. */
    fade: "0s",
});
