import * as style from "@destack/style";

/** The colours of the work tracker, light and dark as it ships. */
export const asana = style.defineConsts({
    /** The top bar and the sidebar. */
    chrome: "#2e2e30",
    /** The top bar's search field. */
    chromeField: "#565557",
    /** The chrome's text. */
    chromeInk: "#f5f4f3",
    /** The chrome's quiet text. */
    chromeMuted: "#a2a0a2",
    /** The chosen sidebar link. */
    chromeOn: "rgb(255 255 255 / 0.12)",
    /** The project. */
    surface: "light-dark(#ffffff, #1e1f21)",
    /** The text. */
    ink: "light-dark(#1e1f21, #f5f4f3)",
    /** Secondary text. */
    muted: "light-dark(#6d6e6f, #a2a0a2)",
    /** Rules and cell edges. */
    line: "light-dark(#edeae9, #424244)",
    /** A done task's check. */
    done: "#58a182",
    /** The add task button. */
    blue: "#4573d2",
    /** The project's colour. */
    coral: "#f06a6a",
    /** The on-track status. */
    onTrack: "#58a182",
    /** A high priority's pill. */
    high: "light-dark(#fcd9cf, #5c2c22)",
    /** A medium priority's pill. */
    medium: "light-dark(#fbe7b5, #4e3f12)",
    /** A low priority's pill. */
    low: "light-dark(#c7e9f0, #1d3c44)",
    /** A pill's text. */
    pillInk: "light-dark(#1e1f21, #f5f4f3)",
    /** Text on a filled accent. */
    onAccent: "#ffffff",
    /** The typeface the app sets, or the nearest one a machine has. */
    face: '"TT Norms Pro", -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, sans-serif',
});
