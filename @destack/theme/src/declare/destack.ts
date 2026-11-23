import { TYPEFACES } from "../theme/index.ts";
import { defineTheme } from "./theme.ts";

/** Destack's type: IBM Plex for interface and code text. */
export const DESTACK_FONTS = {
    text: TYPEFACES.plex,
    code: '"IBM Plex Mono", "ui-monospace", "SFMono-Regular", "Menlo", "monospace"',
} as const;

/** The tracking of large headings in ems, tightened so they read as one shape. */
const HEADING_TRACKING = -0.02;

/** The headings Destack sets lighter and tighter than the type scale, for IBM Plex. */
export const DESTACK_TEXT = {
    "9xl": { tracking: HEADING_TRACKING },
    "8xl": { tracking: HEADING_TRACKING },
    "7xl": { tracking: HEADING_TRACKING },
    "6xl": { tracking: HEADING_TRACKING },
    "5xl": { tracking: HEADING_TRACKING },
    "4xl": { tracking: HEADING_TRACKING },
    "3xl": { tracking: HEADING_TRACKING },
    "2xl": { weight: "medium", tracking: HEADING_TRACKING },
    xl: { weight: "medium" },
    lg: { weight: "medium" },
} as const;

/** Destack's look for every package's views: warm white by day, night water by night. */
export const destackTheme = defineTheme({
    name: "destack",
    base: "#8d8d89",
    accent: "#ff792e",
    radius: "medium",
    ground: {
        light: { background: "#fbfaf7", foreground: "#12313c", muted: "#f2efe9" },
        dark: { background: "#0b2029", foreground: "#eeeae1", muted: "#122a33" },
    },
    fonts: DESTACK_FONTS,
    text: DESTACK_TEXT,
});
