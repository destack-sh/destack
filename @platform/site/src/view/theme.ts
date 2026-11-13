import { DEFAULT_DISPLAY } from "@destack/theme";
import { destackTheme } from "@destack/theme/declare";

/** The dark theme root of surfaces that stay night in both appearances, like the sky bands. */
export const night = destackTheme.variables({ ...DEFAULT_DISPLAY, appearance: "dark" });
