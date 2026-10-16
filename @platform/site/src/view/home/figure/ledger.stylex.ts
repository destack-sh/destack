import * as style from "@destack/style";

/** The colours a ledger passes down to its blocks and their pinned names. */
export const ledgerVariables = style.defineVars({
    /** The colour behind the ledger: its window's card, or the page. */
    surface: "transparent",
    /** A block's fill. */
    fill: "transparent",
    /** A block's tint, drawn as its edge. */
    tint: "transparent",
});
