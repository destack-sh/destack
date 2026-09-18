import * as stylex from "@destack/style";

/// Shared publication constants compiled to CSS variables.
export const tokens = stylex.defineConsts({
    code: "#1b3037",
    cream: "var(--publication-page)",
    creamDeep: "var(--publication-page-muted)",
    diagnostic: "#c22b12",
    displayFont: '"IBM Plex Sans Condensed", "IBM Plex Sans Variable", sans-serif',
    gutterLeft: "var(--site-gutter-left)",
    gutterRight: "var(--site-gutter-right)",
    hairline: "var(--publication-hairline)",
    night: "#12313c",
    orange: "var(--publication-accent)",
    posterFont: '"Limelight", "Futura", "Arial Black", sans-serif',
    orangeLight: "#e77443",
    publicationRow: "2.5rem",
    publicationSpace: "0.5rem",
    rust: "var(--publication-rust)",
    siteControlHeight: "2.5rem",
    siteFontSize: "0.95rem",
    siteWidth: "var(--site-width)",
    stroke: "var(--publication-stroke)",
    textStroke: "1.5px",
});
