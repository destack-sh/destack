import * as style from "@destack/style";

/** The values a figure passes down to the cards and bands inside it. */
export const cardVariables = style.defineVars({
    /** The opacity of the cards' registration marks, one while the figure is open. */
    marks: "0",
    /** The delay before a band's cards fall into place. */
    cascade: "0ms",
    /** The opacity of a card's ink. */
    ink: "1",
    /** The duration of a card's ink fade. */
    inkTime: "0ms",
    /** The delay before a card's ink fades. */
    inkDelay: "0ms",
});

/** The paints of the duct tape across the figure's gaps. */
export const tape = style.defineConsts({
    /** The tape's silver strip. */
    strip: "#cdd3d6",
    /** The shine along the tape. */
    shine: "#f1f3f4",
    /** The label written on the tape. */
    ink: "#0b1a20",
});
