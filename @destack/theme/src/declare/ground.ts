import type { ThemeDefinition } from "../theme/index.ts";

/** The colors a theme takes at one time of day, as six-digit sRGB hex colors. */
export interface Ground {
    /** The page. */
    readonly background: string;
    /** The text on it. */
    readonly foreground: string;
    /** The quiet fills on it: muted and secondary areas. */
    readonly muted: string;
}

/** The alpha of a line drawn in the text color, a hairline on any ground. */
const LINE_ALPHA = 0.18;

/** How far the text on a raised surface lightens at night, which the lighter surface needs. */
const NIGHT_LIFT = 0.02;

/** Declare the roles of a theme that sets its ground by day and by night. */
export function groundRoles(day: Ground, night: Ground): NonNullable<ThemeDefinition["roles"]> {
    const text = { from: "foreground", lightness: { light: 0, dark: NIGHT_LIFT } } as const;

    return {
        background: { light: day.background, dark: night.background },
        foreground: { light: day.foreground, dark: night.foreground },
        cardForeground: text,
        popoverForeground: text,
        secondary: { light: day.muted, dark: night.muted },
        secondaryForeground: text,
        muted: { light: day.muted, dark: night.muted },
        border: { from: "foreground", lightness: { light: 0, dark: 0 }, alpha: LINE_ALPHA },
    };
}
