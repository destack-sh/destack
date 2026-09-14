import * as stylex from "@stylexjs/stylex";

/// Shared publication constants compiled to CSS variables.
export const tokens = stylex.defineVars({
    accent: "var(--publication-accent)",
    code: "#1b3037",
    cream: "var(--publication-page)",
    creamDeep: "var(--publication-page-muted)",
    diagnostic: "#c22b12",
    displayFont:
        '"IBM Plex Sans Condensed", "IBM Plex Sans Variable", sans-serif',
    gutterLeft: "calc(1.5rem + env(safe-area-inset-left))",
    gutterRight: "calc(1.5rem + env(safe-area-inset-right))",
    hairline: "var(--publication-hairline)",
    ink: "var(--publication-ink)",
    line: "var(--publication-line)",
    monoFont: '"IBM Plex Mono", ui-monospace, SFMono-Regular, Menlo, monospace',
    night: "#12313c",
    orange: "var(--publication-accent)",
    posterFont: '"Limelight", "Futura", "Arial Black", sans-serif',
    orangeLight: "#e77443",
    page: "var(--publication-page)",
    publicationRow: "2.5rem",
    publicationSpace: "0.5rem",
    rust: "var(--publication-rust)",
    siteControlHeight: "2.5rem",
    siteFontSize: "0.95rem",
    siteWidth: "72rem",
    soft: "var(--publication-soft)",
    stroke: "var(--publication-stroke)",
    text: "var(--publication-ink)",
    textFont: '"IBM Plex Sans Variable", Helvetica, Arial, sans-serif',
    textStroke: "1.5px",
});
