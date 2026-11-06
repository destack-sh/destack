import { DEFAULT_DISPLAY } from "@destack/theme";
import { defineTheme } from "@destack/theme/declare";
import { mono, sans } from "./font.ts";

/** Paper cream by day and night water by night, with signal orange for primary actions. */
export const theme = defineTheme({
    name: "site",
    base: "sand",
    radius: "none",
    accent: "#ff792e",
    roles: {
        background: { light: "#f8f4ec", dark: "#0b2029" },
        foreground: { light: "#12313c", dark: "#f1eadb" },
        muted: { light: "#efe9dd", dark: "#122a33" },
        border: { from: "foreground", lightness: { light: 0, dark: 0 }, alpha: 0.18 },
    },
    fonts: { text: sans.stack, code: mono.stack },
    text: {
        title1: { weight: "medium" },
        title2: { weight: "medium" },
        title3: { weight: "medium" },
    },
});

/** The dark theme root of surfaces that stay night in both appearances, like the sky bands. */
export const night = theme.variables({ ...DEFAULT_DISPLAY, appearance: "dark" });
