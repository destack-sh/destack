import { DESTACK_FONTS, DESTACK_TEXT, destackTheme } from "./destack.ts";
import { groundRoles } from "./ground.ts";
import { defineTheme } from "./theme.ts";

/** Mercury: neutral gray with ink for an accent, the theme with no hue. */
export const mercuryTheme = defineTheme({
    name: "mercury",
    base: "gray",
    accent: "gray",
    roles: {
        ...groundRoles(
            { background: "#fbfbfa", foreground: "#1c1d1e", muted: "#f1f1ef" },
            { background: "#111213", foreground: "#ededec", muted: "#1b1c1e" },
        ),
        primary: { light: "#1f2021", dark: "#eeeeec" },
    },
    fonts: DESTACK_FONTS,
    text: DESTACK_TEXT,
});

/** Venus: warm mauve with a rose accent. */
export const venusTheme = defineTheme({
    name: "venus",
    base: "mauve",
    accent: "crimson",
    roles: groundRoles(
        { background: "#fcf9fa", foreground: "#3a1c2a", muted: "#f5eef1" },
        { background: "#1d1218", foreground: "#f1e6ea", muted: "#2a1a22" },
    ),
    fonts: DESTACK_FONTS,
    text: DESTACK_TEXT,
});

/** Earth: sage with a green accent, its success states in teal to stay apart. */
export const earthTheme = defineTheme({
    name: "earth",
    base: "sage",
    accent: "grass",
    status: { success: "teal" },
    roles: groundRoles(
        { background: "#f8faf6", foreground: "#17301f", muted: "#edf2ea" },
        { background: "#0f1d17", foreground: "#e6eee6", muted: "#172a21" },
    ),
    fonts: DESTACK_FONTS,
    text: DESTACK_TEXT,
});

/** Mars: sand with a rust accent, its destructive actions in crimson to stay apart. */
export const marsTheme = defineTheme({
    name: "mars",
    base: "sand",
    accent: "tomato",
    status: { destructive: "crimson" },
    roles: groundRoles(
        { background: "#fcf8f4", foreground: "#3b1a12", muted: "#f5ede6" },
        { background: "#20120e", foreground: "#f2e7e1", muted: "#2e1b15" },
    ),
    fonts: DESTACK_FONTS,
    text: DESTACK_TEXT,
});

/** Jupiter: warm brown-gray with a bronze accent. */
export const jupiterTheme = defineTheme({
    name: "jupiter",
    base: "#8f8780",
    accent: "bronze",
    roles: groundRoles(
        { background: "#fbf8f3", foreground: "#33261a", muted: "#f3eee6" },
        { background: "#1c1611", foreground: "#efe8de", muted: "#29211a" },
    ),
    fonts: DESTACK_FONTS,
    text: DESTACK_TEXT,
});

/** Saturn: pale olive with a gold accent, its warnings in orange to stay apart. */
export const saturnTheme = defineTheme({
    name: "saturn",
    base: "olive",
    accent: "amber",
    status: { warning: "orange" },
    roles: groundRoles(
        { background: "#fbfaf1", foreground: "#2f2c12", muted: "#f3f1e3" },
        { background: "#1a1909", foreground: "#efeddc", muted: "#262512" },
    ),
    fonts: DESTACK_FONTS,
    text: DESTACK_TEXT,
});

/** Uranus: cool gray with a cyan accent. */
export const uranusTheme = defineTheme({
    name: "uranus",
    base: "#868e90",
    accent: "cyan",
    roles: groundRoles(
        { background: "#f5fbfb", foreground: "#0f3238", muted: "#e9f3f3" },
        { background: "#0a1e22", foreground: "#e2efef", muted: "#112a2f" },
    ),
    fonts: DESTACK_FONTS,
    text: DESTACK_TEXT,
});

/** Neptune: slate with a deep indigo accent, its informational states in cyan to stay apart. */
export const neptuneTheme = defineTheme({
    name: "neptune",
    base: "slate",
    accent: "indigo",
    status: { info: "cyan" },
    roles: groundRoles(
        { background: "#f7f8fd", foreground: "#17204a", muted: "#edeffa" },
        { background: "#0d1226", foreground: "#e6e9f6", muted: "#161c38" },
    ),
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
