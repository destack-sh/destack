import * as stylex from "@destack/style";

/// The site lattice and shared constants, compiled to CSS variables defined in `site.css`.
export const tokens = stylex.defineConsts({
    /// The frame width shared by every page.
    siteWidth: "var(--site-width)",
    /// One of the twelve frame columns; also the square module height.
    column: "var(--site-column)",
    /// The height of the header, footer, and toolbar rows.
    bar: "var(--site-bar)",
    /// The homepage row height, sized so four rows, six stage rows, and two bars fill the window.
    row: "var(--site-row)",
    /// The taller row height of the homepage stack figure.
    stage: "var(--site-stage)",
    /// The breadboard hole pitch across: the stack figure's drawing is 88 cells wide.
    cell: "var(--site-cell)",
    /// The breadboard hole pitch down: nine cells to a stage row.
    cellRow: "var(--site-cell-row)",
    /// The text inset inside a lattice cell.
    inset: "var(--site-inset)",
    /// The lattice rule width.
    hairline: "1px",
    /// The bright fill used for primary actions in both themes.
    signal: "#ff792e",
    /// The ink drawn on signal fills.
    signalInk: "#12313c",
    /// The deep space field behind illustrations in both themes.
    space: "#0d2233",
    /// The cream drawn on space and night fields in both themes.
    cream: "#f1eadb",
    /// The night field behind code and reveals in both themes.
    night: "#0b2029",
    /// The poster typeface reserved for the wordmark.
    posterFont: '"Limelight", "Futura", "Arial Black", sans-serif',
    /// The mono typeface for commands, callouts, and captions.
    monoFont: '"IBM Plex Mono", ui-monospace, SFMono-Regular, Menlo, monospace',
    /// The muted page tone used behind search results.
    pageMuted: "var(--publication-page-muted)",
});
