import * as style from "@destack/style";

/** The colours of a waitlist the prompt-built app generator published, light and dark. */
export const lovable = style.defineConsts({
    /** The page. */
    surface: "light-dark(#fffaf7, #110d0c)",
    /** A card. */
    card: "light-dark(#ffffff, #1c1716)",
    /** The text. */
    ink: "light-dark(#1c1210, #faf3f0)",
    /** Secondary text. */
    muted: "light-dark(#6f625e, #b4a6a1)",
    /** Rules and card edges. */
    line: "light-dark(#f0e3dd, #2f2522)",
    /** The hero's warm gradient. */
    glow: "linear-gradient(135deg, #ff6b4a 0%, #ff3d8b 50%, #7b5cff 100%)",
    /** The badge. */
    badge: "#111111",
    /** Text on the gradient and the badge. */
    onAccent: "#ffffff",
    /** The typeface the generated app sets, or the nearest one a machine has. */
    face: 'Inter, -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, sans-serif',
});
