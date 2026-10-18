import * as style from "@destack/style";

/** The colours of the team wiki, light and dark as it ships. */
export const confluence = style.defineConsts({
    /** The page and the top bar. */
    surface: "light-dark(#ffffff, #1f1f21)",
    /** The sidebar. */
    side: "light-dark(#f8f8f8, #18191a)",
    /** The text. */
    ink: "light-dark(#292a2e, #cecfd2)",
    /** Secondary text and icons. */
    muted: "light-dark(#505258, #a9abaf)",
    /** Faint text. */
    faint: "light-dark(#6b6e76, #96999e)",
    /** Rules and table edges. */
    line: "light-dark(#dddee1, #38393c)",
    /** The brand blue of buttons and links. */
    blue: "light-dark(#1868db, #669df1)",
    /** The chosen page in the tree, a mention and a table's header cells. */
    selected: "light-dark(#e9f2fe, #1c2b42)",
    /** A table's header cells. */
    header: "light-dark(#f8f8f8, #242528)",
    /** The info panel's fill. */
    panel: "light-dark(#e9f2fe, #1c2b42)",
    /** The info panel's mark. */
    panelMark: "light-dark(#1868db, #669df1)",
    /** The in-progress lozenge. */
    lozenge: "light-dark(#cfe1fd, #123263)",
    /** The in-progress lozenge's text. */
    lozengeInk: "light-dark(#1558bc, #8fb8f6)",
    /** A date lozenge. */
    date: "light-dark(#f0f1f2, #2b2c2f)",
    /** Text on a filled accent. */
    onAccent: "#ffffff",
    /** The typeface the app sets, or the nearest one a machine has. */
    face: '"Atlassian Sans", ui-sans-serif, -apple-system, BlinkMacSystemFont, "Segoe UI", Ubuntu, sans-serif',
});
