import { DEFAULT_PREFERENCES } from "@destack/theme";
import { defineTheme } from "@destack/theme/declare";
import { mono, sans } from "./font.ts";

/** Paper by day and night water by night, with the signal orange for primary actions. */
export const theme = defineTheme({
    name: "site",
    base: "sand",
    accent: "#ff792e",
    roles: {
        background: { light: "#f8f5ee", dark: "#0b2029" },
        foreground: { light: "#12313c", dark: "#f1eadb" },
        muted: { light: "#eeebe3", dark: "#122a33" },
        border: { from: "foreground", lightness: { light: 0, dark: 0 }, alpha: 0.18 },
    },
    fonts: { text: sans.stack, code: mono.stack },
    text: {
        title1: { weight: "medium" },
        title2: { weight: "medium" },
        title3: { weight: "medium" },
    },
});

/** The dark theme root of the surfaces that stay night in both appearances, such as the starry goo cells. */
export const night = theme.variables("dark", DEFAULT_PREFERENCES);

/** The light theme root of the surfaces that stay day in both appearances, such as a mocked app's pane. */
export const day = theme.variables("light", DEFAULT_PREFERENCES);
