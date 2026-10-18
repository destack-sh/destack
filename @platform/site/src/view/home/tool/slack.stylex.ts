import * as style from "@destack/style";

/** The colours of the chat workspace. */
export const slack = style.defineConsts({
    /** The page background. */
    surface: "light-dark(#ffffff, #1a1d21)",
    /** The message text. */
    ink: "light-dark(#1d1c1d, #d1d2d3)",
    /** The top bar and the rail. */
    aubergine: "#350d36",
    /** The channel sidebar. */
    sidebar: "#3f0e40",
    /** The selected channel and thread links. */
    link: "light-dark(#1264a3, #1d9bd1)",
    /** Secondary text. */
    muted: "light-dark(#616061, #ababad)",
    /** The border of a mention chip. */
    highlight: "#1d9bd1",
    /** The composer border. */
    border: "light-dark(#bbbbbb, #565856)",
    /** The member list border. */
    line: "light-dark(#dddddd, #35373b)",
    /** Tab rules and app icon fills. */
    rule: "light-dark(#e8e8e8, #35373b)",
    /** The guest badge. */
    notice: "light-dark(#fff8e1, #3b3214)",
    /** The guest badge text. */
    noticeInk: "light-dark(#5c4a00, #f2c744)",
    /** The mention chip. */
    mention: "light-dark(#e8f5fa, #1d3a4a)",
    /** The typeface the site sets, or the nearest one a machine has. */
    face: 'Lato, "Slack-Lato", -apple-system, BlinkMacSystemFont, "Segoe UI", Helvetica, Arial, sans-serif',
});
