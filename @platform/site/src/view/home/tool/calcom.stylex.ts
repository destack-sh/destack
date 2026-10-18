import * as style from "@destack/style";

/** The colours of the open scheduling page, light and dark as it ships. */
export const calcom = style.defineConsts({
    /** The page behind the booker. */
    page: "light-dark(#f3f4f6, #101010)",
    /** The booker's card. */
    card: "light-dark(#ffffff, #1c1c1c)",
    /** The text. */
    ink: "light-dark(#111827, #f3f4f6)",
    /** Secondary text and icons. */
    muted: "light-dark(#6b7280, #9ca3af)",
    /** Rules and the card's edge. */
    line: "light-dark(#e5e7eb, #2b2b2b)",
    /** An open date's fill. */
    open: "light-dark(#e5e7eb, #2b2b2b)",
    /** The chosen date and the toggle's chosen side. */
    chosen: "light-dark(#111827, #f3f4f6)",
    /** Text on the chosen date. */
    onChosen: "light-dark(#ffffff, #111827)",
    /** The availability dot beside a time. */
    available: "#22c55e",
    /** The typeface the app sets, or the nearest one a machine has. */
    face: '"Cal Sans", Inter, -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, sans-serif',
    /** The typeface of the body text. */
    text: 'Inter, -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, sans-serif',
});
