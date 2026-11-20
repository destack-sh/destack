import * as style from "@destack/style";

/** The colours of the Monday page an agent made on its own, in the stock look agents reach for. */
export const monday = style.defineConsts({
    /** The page. */
    surface: "light-dark(#f8fafc, #0f172a)",
    /** A card. */
    card: "light-dark(#ffffff, #1e293b)",
    /** The text. */
    ink: "light-dark(#0f172a, #f1f5f9)",
    /** Secondary text. */
    muted: "light-dark(#64748b, #94a3b8)",
    /** Rules and card edges. */
    line: "light-dark(#e2e8f0, #334155)",
    /** A card's shadow. */
    shadow: "0 1px 3px rgb(0 0 0 / 0.08), 0 1px 2px rgb(0 0 0 / 0.04)",
    /** Text on the button. */
    onAccent: "#ffffff",
    /** The button. */
    button: "#2563eb",
    /** An open promise's pill. */
    openPill: "light-dark(#fef3c7, #422006)",
    /** An open promise's pill text. */
    openInk: "light-dark(#92400e, #fcd34d)",
    /** A waiting promise's pill. */
    waitingPill: "light-dark(#e0e7ff, #1e1b4b)",
    /** A waiting promise's pill text. */
    waitingInk: "light-dark(#3730a3, #a5b4fc)",
    /** A free slot's chip. */
    slot: "light-dark(#dbeafe, #172554)",
    /** A free slot's chip text. */
    slotInk: "light-dark(#1d4ed8, #93c5fd)",
    /** The typeface the generated app sets, or the nearest one a machine has. */
    face: 'ui-sans-serif, system-ui, -apple-system, "Segoe UI", Roboto, sans-serif',
});
