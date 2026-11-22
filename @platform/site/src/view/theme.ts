import { DEFAULT_DISPLAY } from "@destack/theme";
import { defineTheme, destackTheme } from "@destack/theme/declare";

/** The site's theme: Destack's, on the stronger sand paper the marketing pages print on. */
export const paper = defineTheme({
    ...destackTheme.definition,
    name: "paper",
    roles: {
        ...destackTheme.definition.roles,
        background: { light: "#f7f4ef", dark: "#0b2029" },
        muted: { light: "#ede9e2", dark: "#122a33" },
    },
});

/** The dark theme root of surfaces that stay night in both appearances, like the sky bands. */
export const night = paper.variables({ ...DEFAULT_DISPLAY, appearance: "dark" });

/** The app theme of figure windows, compact and in small text as a window drawn smaller. */
export const app = Object.fromEntries(
    Object.entries(
        destackTheme.variables({ ...DEFAULT_DISPLAY, density: "compact", textSize: "small" }),
    ).filter(([name]) => name !== "color-scheme"),
);
