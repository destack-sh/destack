import * as style from "@destack/style";

/** The colours of the automation tool's editor. */
export const zapier = style.defineConsts({
    /** The page background. */
    canvas: "light-dark(#fffdf9, #1a1918)",
    /** The text. */
    ink: "light-dark(#201515, #f2efeb)",
    /** Secondary text. */
    muted: "light-dark(#6b6460, #a39d98)",
    /** The brand orange. */
    orange: "#ff4f00",
    /** A successful run. */
    success: "light-dark(#0b7a3e, #4ade80)",
    /** A successful run's badge. */
    successField: "light-dark(#e3f6ea, #12301f)",
    /** A failed run. */
    error: "light-dark(#c4320a, #ff8a65)",
    /** A failed run's badge. */
    errorField: "light-dark(#fdeae4, #3a1a12)",
    /** The dots of the editor's canvas. */
    dot: "light-dark(#dcd7cf, #3a3632)",
    /** A step card. */
    card: "light-dark(#ffffff, #262422)",
    /** A step card's border. */
    border: "light-dark(#e2ddd5, #3d3935)",
    /** A skipped step's text. */
    skipped: "light-dark(#a8a29c, #6e6863)",
});

/** The colours of the chat assistant. */
export const chatgpt = style.defineConsts({
    /** The page background. */
    canvas: "light-dark(#ffffff, #212121)",
    /** The sidebar. */
    sidebar: "light-dark(#f9f9f9, #171717)",
    /** The text. */
    ink: "light-dark(#0d0d0d, #ececec)",
    /** Secondary text. */
    muted: "light-dark(#5d5d5d, #b4b4b4)",
    /** Card borders. */
    border: "light-dark(#e5e5e5, #3a3a3a)",
    /** A failed task's note. */
    error: "light-dark(#d1342f, #ff7b72)",
    /** The brand green. */
    green: "#10a37f",
});

/** The colours the rented tools share or borrow from their own brand marks. */
export const marks = style.defineConsts({
    /** A rule between rows of a tool's lists. */
    border: "light-dark(#e6e8eb, #34373c)",
    /** A selected navigation item's field. */
    selected: "light-dark(#e8f1ff, #1c2c44)",
    /** A search field on a dark bar. */
    glass: "rgba(255, 255, 255, 0.16)",
    /** The mail tool's red mark. */
    gmailRed: "#ea4335",
    /** Your own avatar's teal. */
    teal: "#2f7d8c",
    /** A failed run's annotation field. */
    dangerField: "light-dark(#ffebe9, #3c1618)",
    /** A tracker's started status amber. */
    amber: "#f2c94c",
    /** A tracker's review status green. */
    green: "#4cb782",
    /** A run's passed green. */
    passed: "light-dark(#1a7f37, #3fb950)",
    /** A skipped run's grey. */
    skipped: "light-dark(#818b98, #6e7681)",
    /** The chat tool's step accent. */
    berry: "#611f69",
    /** The chat tool's webhook trigger. */
    trigger: "light-dark(#e8f5fa, #1d3a4a)",
    /** The docs tool's board column fields. */
    column: "light-dark(#f7f6f3, #202020)",
    /** The docs tool's not started status. */
    notStarted: "light-dark(#e3e2e0, #373737)",
    /** The docs tool's in progress status. */
    inProgress: "light-dark(#d3e5ef, #28456c)",
    /** The docs tool's done status. */
    done: "light-dark(#dbeddb, #2b593f)",
    /** The docs tool's automation panel shadow. */
    lift: "rgba(15, 15, 15, 0.16)",
    /** White text on a brand fill. */
    white: "#ffffff",
    /** A card's soft drop shadow. */
    shadow: "rgba(15, 15, 15, 0.08)",
    /** The task tool's completed status violet. */
    violet: "#5e6ad2",
    /** The docs tool's secondary grey. */
    grey: "light-dark(#9b9a97, #7f7f7c)",
    /** The code host's failed run red. */
    danger: "light-dark(#d1242f, #f85149)",
});
