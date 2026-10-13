import { DEFAULT_PREFERENCES } from "@destack/theme";
import { defineTheme } from "@destack/theme/declare";
import { mono, sans } from "./font.ts";

/** White by day and night water by night, with signal orange for primary actions. */
export const theme = defineTheme({
    name: "site",
    base: "sand",
    accent: "#ff792e",
    roles: {
        background: { light: "#ffffff", dark: "#0b2029" },
        foreground: { light: "#12313c", dark: "#f1eadb" },
        muted: { light: "#f3f2ef", dark: "#122a33" },
        border: { from: "foreground", lightness: { light: 0, dark: 0 }, alpha: 0.18 },
    },
    fonts: { text: sans.stack, code: mono.stack },
    text: {
        title1: { weight: "medium" },
        title2: { weight: "medium" },
        title3: { weight: "medium" },
    },
});

/** The dark theme root of surfaces that stay night in both appearances, like the goo cells. */
export const night = theme.variables("dark", DEFAULT_PREFERENCES);
