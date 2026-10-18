import * as style from "@destack/style";

/** The colours of the code host in its light and dark themes. */
export const github = style.defineConsts({
    /** The top bar and tab strip. */
    inset: "light-dark(#f6f8fa, #010409)",
    /** The page background. */
    canvas: "light-dark(#ffffff, #0d1117)",
    /** The latest commit row. */
    overlay: "light-dark(#f6f8fa, #151b23)",
    /** The topic tag background. */
    accentMuted: "light-dark(#ddf4ff, #121d2f)",
    /** Button and branch fills. */
    subtle: "light-dark(#f6f8fa, #212830)",
    /** Counter fills. */
    neutral: "light-dark(#e6eaef, #2f3742)",
    /** Borders. */
    border: "light-dark(#d1d9e0, #3d444d)",
    /** The code button. */
    success: "light-dark(#1f883d, #238636)",
    /** Secondary text. */
    muted: "light-dark(#59636e, #9198a1)",
    /** The topic tag text. */
    accent: "light-dark(#0969da, #4493f8)",
    /** The selected tab underline. */
    attention: "light-dark(#fd8c73, #f78166)",
    /** The text. */
    foreground: "light-dark(#1f2328, #e6edf3)",
    /** The typeface the site sets, or the nearest one a machine has. */
    face: '-apple-system, BlinkMacSystemFont, "Segoe UI", Helvetica, Arial, sans-serif, "Noto Sans"',
});
