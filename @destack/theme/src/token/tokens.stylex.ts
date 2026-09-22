import { defineConsts } from "@destack/style";

/** Semantic color roles. */
export const color = defineConsts({
    background: "var(--destack-color-background)",
    foreground: "var(--destack-color-foreground)",
    card: "var(--destack-color-card)",
    cardForeground: "var(--destack-color-cardForeground)",
    popover: "var(--destack-color-popover)",
    popoverForeground: "var(--destack-color-popoverForeground)",
    primary: "var(--destack-color-primary)",
    primaryForeground: "var(--destack-color-primaryForeground)",
    secondary: "var(--destack-color-secondary)",
    secondaryForeground: "var(--destack-color-secondaryForeground)",
    muted: "var(--destack-color-muted)",
    mutedForeground: "var(--destack-color-mutedForeground)",
    accent: "var(--destack-color-accent)",
    accentForeground: "var(--destack-color-accentForeground)",
    destructive: "var(--destack-color-destructive)",
    border: "var(--destack-color-border)",
    input: "var(--destack-color-input)",
    ring: "var(--destack-color-ring)",
    sidebar: "var(--destack-color-sidebar)",
    sidebarForeground: "var(--destack-color-sidebarForeground)",
    sidebarPrimary: "var(--destack-color-sidebarPrimary)",
    sidebarPrimaryForeground: "var(--destack-color-sidebarPrimaryForeground)",
    sidebarAccent: "var(--destack-color-sidebarAccent)",
    sidebarAccentForeground: "var(--destack-color-sidebarAccentForeground)",
    sidebarBorder: "var(--destack-color-sidebarBorder)",
    sidebarRing: "var(--destack-color-sidebarRing)",
});

/** Spacing steps 1 through 9. */
export const space = defineConsts({
    "1": "var(--destack-space-1)",
    "2": "var(--destack-space-2)",
    "3": "var(--destack-space-3)",
    "4": "var(--destack-space-4)",
    "5": "var(--destack-space-5)",
    "6": "var(--destack-space-6)",
    "7": "var(--destack-space-7)",
    "8": "var(--destack-space-8)",
    "9": "var(--destack-space-9)",
});

/** Font sizes 1 through 9. */
export const fontSize = defineConsts({
    "1": "var(--destack-font-size-1)",
    "2": "var(--destack-font-size-2)",
    "3": "var(--destack-font-size-3)",
    "4": "var(--destack-font-size-4)",
    "5": "var(--destack-font-size-5)",
    "6": "var(--destack-font-size-6)",
    "7": "var(--destack-font-size-7)",
    "8": "var(--destack-font-size-8)",
    "9": "var(--destack-font-size-9)",
});

/** Line heights 1 through 9. */
export const lineHeight = defineConsts({
    "1": "var(--destack-line-height-1)",
    "2": "var(--destack-line-height-2)",
    "3": "var(--destack-line-height-3)",
    "4": "var(--destack-line-height-4)",
    "5": "var(--destack-line-height-5)",
    "6": "var(--destack-line-height-6)",
    "7": "var(--destack-line-height-7)",
    "8": "var(--destack-line-height-8)",
    "9": "var(--destack-line-height-9)",
});

/** Letter spacing 1 through 9. */
export const letterSpacing = defineConsts({
    "1": "var(--destack-letter-spacing-1)",
    "2": "var(--destack-letter-spacing-2)",
    "3": "var(--destack-letter-spacing-3)",
    "4": "var(--destack-letter-spacing-4)",
    "5": "var(--destack-letter-spacing-5)",
    "6": "var(--destack-letter-spacing-6)",
    "7": "var(--destack-letter-spacing-7)",
    "8": "var(--destack-letter-spacing-8)",
    "9": "var(--destack-letter-spacing-9)",
});

/** Heading line heights 1 through 9. */
export const headingLineHeight = defineConsts({
    "1": "var(--destack-heading-line-height-1)",
    "2": "var(--destack-heading-line-height-2)",
    "3": "var(--destack-heading-line-height-3)",
    "4": "var(--destack-heading-line-height-4)",
    "5": "var(--destack-heading-line-height-5)",
    "6": "var(--destack-heading-line-height-6)",
    "7": "var(--destack-heading-line-height-7)",
    "8": "var(--destack-heading-line-height-8)",
    "9": "var(--destack-heading-line-height-9)",
});

/** Text weights. */
export const fontWeight = defineConsts({
    light: "var(--destack-font-weight-light)",
    regular: "var(--destack-font-weight-regular)",
    medium: "var(--destack-font-weight-medium)",
    bold: "var(--destack-font-weight-bold)",
});

/** Default and code font stacks. */
export const fontFamily = defineConsts({
    default: "var(--destack-default-font-family)",
    code: "var(--destack-code-font-family)",
});

/** Corner radii and control treatments. */
export const radius = defineConsts({
    "1": "var(--destack-radius-1)",
    "2": "var(--destack-radius-2)",
    "3": "var(--destack-radius-3)",
    "4": "var(--destack-radius-4)",
    "5": "var(--destack-radius-5)",
    "6": "var(--destack-radius-6)",
    full: "var(--destack-radius-full)",
    thumb: "var(--destack-radius-thumb)",
});

/** Elevation shadows 1 through 6. */
export const shadow = defineConsts({
    "1": "var(--destack-shadow-1)",
    "2": "var(--destack-shadow-2)",
    "3": "var(--destack-shadow-3)",
    "4": "var(--destack-shadow-4)",
    "5": "var(--destack-shadow-5)",
    "6": "var(--destack-shadow-6)",
});

/** Chart series colors. */
export const chart = defineConsts({
    "1": "light-dark(oklch(0.646 0.222 41.116), oklch(0.488 0.243 264.376))",
    "2": "light-dark(oklch(0.6 0.118 184.704), oklch(0.696 0.17 162.48))",
    "3": "light-dark(oklch(0.398 0.07 227.392), oklch(0.769 0.188 70.08))",
    "4": "light-dark(oklch(0.828 0.189 84.429), oklch(0.627 0.265 303.9))",
    "5": "light-dark(oklch(0.769 0.188 70.08), oklch(0.645 0.246 16.439))",
});
