import * as style from "@destack/style";

/** The colours of a waitlist the UI generator made and the platform hosts, light and dark. */
export const v0 = style.defineConsts({
    /** The page. */
    surface: "light-dark(#ffffff, #0a0a0a)",
    /** A muted fill. */
    muted: "light-dark(#f5f5f5, #171717)",
    /** The text. */
    ink: "light-dark(#0a0a0a, #fafafa)",
    /** Secondary text. */
    quiet: "light-dark(#737373, #a3a3a3)",
    /** Rules and card edges. */
    line: "light-dark(#e5e5e5, #262626)",
    /** The typeface the generated app sets, or the nearest one a machine has. */
    face: 'Geist, Inter, -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, sans-serif',
});
