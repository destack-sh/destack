import { defineTheme } from "./theme.ts";

/** Destack's look, which every package's views take: paper by day, night water by night, signal orange. */
export const destackTheme = defineTheme({
    name: "destack",
    base: "#8d8d89",
    accent: "#ff792e",
    radius: "none",
    roles: {
        background: { light: "#f7f4ef", dark: "#0b2029" },
        foreground: { light: "#12313c", dark: "#eeeae1" },
        muted: { light: "#ede9e2", dark: "#122a33" },
        border: { from: "foreground", lightness: { light: 0, dark: 0 }, alpha: 0.18 },
    },
    fonts: {
        text: '"IBM Plex Sans", "Helvetica", "Arial", "sans-serif"',
        code: '"IBM Plex Mono", "ui-monospace", "SFMono-Regular", "Menlo", "monospace"',
    },
    text: {
        "2xl": { weight: "medium" },
        xl: { weight: "medium" },
        lg: { weight: "medium" },
    },
});
