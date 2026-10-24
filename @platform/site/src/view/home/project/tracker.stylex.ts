import * as style from "@destack/style";

/** The colours of the launch tracker running on your laptop. */
export const tracker = style.defineConsts({
    /** The page background. */
    paper: "light-dark(#fafafa, #0d1117)",
    /** The text. */
    ink: "light-dark(#1f2328, #e6edf3)",
    /** The quiet text: dates and notes. */
    muted: "light-dark(#656d76, #8d96a0)",
    /** The rules between milestones and around the list. */
    rule: "light-dark(#d0d7de, #30363d)",
    /** The empty checkbox. */
    box: "light-dark(#8c959f, #6e7681)",
    /** The checked box. */
    done: "#1f883d",
    /** The countdown chip's fill. */
    chip: "light-dark(#fff3cd, #3b2e00)",
    /** The countdown chip's text. */
    chipInk: "light-dark(#7a5b00, #e3b341)",
    /** The dev server badge's fill. */
    badge: "#1f2328",
    /** The dev server badge's text. */
    badgeInk: "#ffffff",
});
