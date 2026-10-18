import * as style from "@destack/style";

/** The colours of the workflow automation editor, light and dark as it ships. */
export const n8n = style.defineConsts({
    /** The canvas. */
    canvas: "light-dark(#f5f5f5, #1f1f1f)",
    /** The canvas's dots. */
    dots: "light-dark(#d6d6d6, #3a3a3a)",
    /** The header, the sidebar and a node. */
    panel: "light-dark(#ffffff, #2a2a2a)",
    /** The text. */
    ink: "light-dark(#2d2e2e, #f0f0f0)",
    /** Secondary text and icons. */
    muted: "light-dark(#7d7d87, #a8a8b0)",
    /** Rules and a node's edge. */
    line: "light-dark(#dbdfe7, #444444)",
    /** The brand coral of the run button and the active switch. */
    coral: "#ff6d5a",
    /** The links between nodes, and a node that ran. */
    green: "#29a568",
    /** The webhook node's colour. */
    webhook: "#5c5cbf",
    /** A node's lift. */
    lift: "0 2px 6px rgb(0 0 0 / 0.08)",
    /** Text on a filled accent. */
    onAccent: "#ffffff",
    /** The typeface the app sets, or the nearest one a machine has. */
    face: '"Open Sans", -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, sans-serif',
});
