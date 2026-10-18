import * as style from "@destack/style";

/** The colours of the notes app in its default dark theme. */
export const obsidian = style.defineConsts({
    /** The editor. */
    editor: "#1e1e1e",
    /** The sidebars, the ribbon and the tab strip. */
    side: "#262626",
    /** The chosen tab, joined to the editor. */
    tab: "#1e1e1e",
    /** The text. */
    ink: "#dadada",
    /** Headings. */
    heading: "#e8e8e8",
    /** Secondary text and icons. */
    muted: "#b3b3b3",
    /** Faint text, such as property names and the status bar. */
    faint: "#666666",
    /** Rules. */
    line: "#363636",
    /** The accent of checkboxes, links, tags and the chosen file. */
    accent: "#a68af9",
    /** A tag's fill. */
    tagFill: "rgb(138 92 245 / 0.15)",
    /** The chosen file's row. */
    active: "#363636",
    /** The tip callout's colour. */
    tip: "#53dfdd",
    /** The tip callout's fill. */
    tipFill: "rgb(83 223 221 / 0.1)",
    /** Text on a filled accent. */
    onAccent: "#1e1e1e",
    /** The typeface the app sets, or the nearest one a machine has. */
    face: 'Inter, ui-sans-serif, -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, sans-serif',
});
