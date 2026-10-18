import * as style from "@destack/style";

/** The colours of a waitlist the in-browser app builder published, in its usual dark style. */
export const bolt = style.defineConsts({
    /** The page. */
    surface: "#0b0d12",
    /** A card. */
    card: "#141821",
    /** The text. */
    ink: "#f2f4f8",
    /** Secondary text. */
    muted: "#99a1b3",
    /** Rules and card edges. */
    line: "#232937",
    /** The accent of the button and the glow. */
    indigo: "#6366f1",
    /** The glow behind the hero. */
    glow: "radial-gradient(60% 50% at 50% 0%, rgb(99 102 241 / 0.35), transparent 70%)",
    /** Text on a filled accent. */
    onAccent: "#ffffff",
    /** The typeface the generated app sets, or the nearest one a machine has. */
    face: 'Inter, -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, sans-serif',
});
