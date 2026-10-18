import * as style from "@destack/style";

/** The colours of the issue tracker's board, light and dark as it ships. */
export const jira = style.defineConsts({
    /** The page and the top bar. */
    surface: "light-dark(#ffffff, #1f1f21)",
    /** The sidebar. */
    side: "light-dark(#f8f8f8, #18191a)",
    /** A board column. */
    column: "light-dark(#f8f8f8, #242528)",
    /** A card. */
    card: "light-dark(#ffffff, #2b2c2f)",
    /** The text. */
    ink: "light-dark(#292a2e, #cecfd2)",
    /** Secondary text and icons. */
    muted: "light-dark(#505258, #a9abaf)",
    /** Faint text. */
    faint: "light-dark(#6b6e76, #96999e)",
    /** Rules and card edges. */
    line: "light-dark(#dddee1, #38393c)",
    /** The brand blue. */
    blue: "light-dark(#1868db, #669df1)",
    /** The chosen view in the sidebar. */
    selected: "light-dark(#e9f2fe, #1c2b42)",
    /** A story's mark. */
    story: "#6a9a23",
    /** A task's mark. */
    task: "#4688ec",
    /** A high priority's arrow. */
    priority: "#e06c00",
    /** The launch epic's lozenge. */
    epic: "light-dark(#f3f0ff, #2b2451)",
    /** The launch epic's text. */
    epicInk: "light-dark(#5e4db2, #b8acf6)",
    /** A card's lift off its column. */
    lift: "0 1px 1px rgb(30 31 33 / 0.25), 0 0 1px rgb(30 31 33 / 0.31)",
    /** Text on a filled accent. */
    onAccent: "#ffffff",
    /** The typeface the app sets, or the nearest one a machine has. */
    face: '"Atlassian Sans", ui-sans-serif, -apple-system, BlinkMacSystemFont, "Segoe UI", Ubuntu, sans-serif',
});
