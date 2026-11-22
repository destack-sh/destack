import { DESTACK_FONTS, DESTACK_TEXT, destackTheme } from "./destack.ts";
import { defineTheme } from "./theme.ts";

/** Mercury: neutral gray with ink for an accent, the theme with no hue. */
export const mercuryTheme = defineTheme({
    name: "mercury",
    base: "gray",
    accent: "gray",
    ground: {
        light: { background: "#fbfbfa", foreground: "#1c1d1e", muted: "#f1f1ef" },
        dark: { background: "#111213", foreground: "#ededec", muted: "#1b1c1e" },
    },
    roles: { solid: { light: "#1f2021", dark: "#eeeeec" } },
    fonts: DESTACK_FONTS,
    text: DESTACK_TEXT,
});

/** Venus: warm mauve with a rose accent. */
export const venusTheme = defineTheme({
    name: "venus",
    base: "mauve",
    accent: "crimson",
    ground: {
        light: { background: "#fcf9fa", foreground: "#3a1c2a", muted: "#f5eef1" },
        dark: { background: "#1d1218", foreground: "#f1e6ea", muted: "#2a1a22" },
    },
    fonts: DESTACK_FONTS,
    text: DESTACK_TEXT,
});

/** Earth: sage with a green accent, its success states in teal to stay apart. */
export const earthTheme = defineTheme({
    name: "earth",
    base: "sage",
    accent: "grass",
    status: { success: "teal" },
    ground: {
        light: { background: "#f8faf6", foreground: "#17301f", muted: "#edf2ea" },
        dark: { background: "#0f1d17", foreground: "#e6eee6", muted: "#172a21" },
    },
    fonts: DESTACK_FONTS,
    text: DESTACK_TEXT,
});

/** Mars: sand with a rust accent, its destructive actions in crimson to stay apart. */
export const marsTheme = defineTheme({
    name: "mars",
    base: "sand",
    accent: "tomato",
    status: { destructive: "crimson" },
    ground: {
        light: { background: "#fcf8f4", foreground: "#3b1a12", muted: "#f5ede6" },
        dark: { background: "#20120e", foreground: "#f2e7e1", muted: "#2e1b15" },
    },
    fonts: DESTACK_FONTS,
    text: DESTACK_TEXT,
});

/** Jupiter: warm brown-gray with a bronze accent. */
export const jupiterTheme = defineTheme({
    name: "jupiter",
    base: "#8f8780",
    accent: "bronze",
    ground: {
        light: { background: "#fbf8f3", foreground: "#33261a", muted: "#f3eee6" },
        dark: { background: "#1c1611", foreground: "#efe8de", muted: "#29211a" },
    },
    fonts: DESTACK_FONTS,
    text: DESTACK_TEXT,
});

/** Saturn: pale olive with a gold accent, its warnings in orange to stay apart. */
export const saturnTheme = defineTheme({
    name: "saturn",
    base: "olive",
    accent: "amber",
    status: { warning: "orange" },
    ground: {
        light: { background: "#fbfaf1", foreground: "#2f2c12", muted: "#f3f1e3" },
        dark: { background: "#1a1909", foreground: "#efeddc", muted: "#262512" },
    },
    fonts: DESTACK_FONTS,
    text: DESTACK_TEXT,
});

/** Uranus: cool gray with a cyan accent. */
export const uranusTheme = defineTheme({
    name: "uranus",
    base: "#868e90",
    accent: "cyan",
    ground: {
        light: { background: "#f5fbfb", foreground: "#0f3238", muted: "#e9f3f3" },
        dark: { background: "#0a1e22", foreground: "#e2efef", muted: "#112a2f" },
    },
    fonts: DESTACK_FONTS,
    text: DESTACK_TEXT,
});

/** Neptune: slate with a deep indigo accent, its informational states in cyan to stay apart. */
export const neptuneTheme = defineTheme({
    name: "neptune",
    base: "slate",
    accent: "indigo",
    status: { info: "cyan" },
    ground: {
        light: { background: "#f7f8fd", foreground: "#17204a", muted: "#edeffa" },
        dark: { background: "#0d1226", foreground: "#e6e9f6", muted: "#161c38" },
    },
    fonts: DESTACK_FONTS,
    text: DESTACK_TEXT,
});

/** Destack's themes: the brand, then one per planet from the sun outward. */
export const THEMES = [
    destackTheme,
    mercuryTheme,
    venusTheme,
    earthTheme,
    marsTheme,
    jupiterTheme,
    saturnTheme,
    uranusTheme,
    neptuneTheme,
] as const;
