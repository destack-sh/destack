import * as style from "@destack/style";

/** The colours of the applet service, light and dark as it ships. */
export const ifttt = style.defineConsts({
    /** The page. */
    surface: "light-dark(#ffffff, #141414)",
    /** The text. */
    ink: "light-dark(#222222, #f2f2f2)",
    /** Secondary text. */
    muted: "light-dark(#666666, #a3a3a3)",
    /** The steps' card. */
    card: "light-dark(#f2f2f2, #222222)",
    /** A step's ingredient tag. */
    tag: "light-dark(#e6e6e6, #333333)",
    /** The Webhooks service blue the applet's header wears, its trigger's colour. */
    webhooks: "#0099ff",
    /** The Slack service tile. */
    slack: "#4a154b",
    /** The Gmail service tile. */
    gmail: "#ea4335",
    /** The edge of the search field on the header band. */
    edge: "rgb(255 255 255 / 0.6)",
    /** The connect switch. */
    switch: "#222222",
    /** Text on a filled colour. */
    onAccent: "#ffffff",
    /** The typeface the site sets, Museo Sans Rounded, or the nearest one a machine has. */
    face: '"Museo Sans Rounded", Nunito, "Avenir Next", Avenir, -apple-system, sans-serif',
});
