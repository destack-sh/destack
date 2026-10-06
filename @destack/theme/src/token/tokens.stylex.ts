// generate with `bun run generate` from tokens.json

import { defineConsts } from "@destack/style";

/** Semantic color roles and status roles, with the default theme's values. */
export const color = defineConsts({
    /** The page background, the base surface. */
    background: "var(--destack-color-background)",
    /** Body text on the background. */
    foreground: "var(--destack-color-foreground)",
    /** The background of cards, the raised surface. */
    card: "var(--destack-color-card)",
    /** Text on cards. */
    cardForeground: "var(--destack-color-card-foreground)",
    /** The background of popovers and menus, the overlay surface. */
    popover: "var(--destack-color-popover)",
    /** Text in popovers and menus. */
    popoverForeground: "var(--destack-color-popover-foreground)",
    /** The solid background of primary actions. */
    primary: "var(--destack-color-primary)",
    /** Text on primary actions. */
    primaryForeground: "var(--destack-color-primary-foreground)",
    /** The background of secondary actions. */
    secondary: "var(--destack-color-secondary)",
    /** Text on secondary actions. */
    secondaryForeground: "var(--destack-color-secondary-foreground)",
    /** The background of muted areas. */
    muted: "var(--destack-color-muted)",
    /** Secondary text such as descriptions. */
    mutedForeground: "var(--destack-color-muted-foreground)",
    /** The background of hovered and selected items. */
    accent: "var(--destack-color-accent)",
    /** Text on hovered and selected items. */
    accentForeground: "var(--destack-color-accent-foreground)",
    /** The solid background of destructive actions. */
    destructive: "var(--destack-color-destructive)",
    /** Text on destructive actions. */
    destructiveForeground: "var(--destack-color-destructive-foreground)",
    /** The solid background of success states. */
    success: "var(--destack-color-success)",
    /** Text on success states. */
    successForeground: "var(--destack-color-success-foreground)",
    /** The solid background of warning states. */
    warning: "var(--destack-color-warning)",
    /** Text on warning states. */
    warningForeground: "var(--destack-color-warning-foreground)",
    /** The solid background of informational states. */
    info: "var(--destack-color-info)",
    /** Text on informational states. */
    infoForeground: "var(--destack-color-info-foreground)",
    /** Borders and separators. */
    border: "var(--destack-color-border)",
    /** Borders of form controls. */
    input: "var(--destack-color-input)",
    /** Focus rings. */
    ring: "var(--destack-color-ring)",
    /** The veil behind dialogs, sheets and drawers. */
    scrim: "var(--destack-color-scrim)",
});

/** Surface levels from the page up, with the default theme's values. */
export const surface = defineConsts({
    /** The page level. */
    base: "var(--destack-surface-base)",
    /** Cards and panels above the page. */
    raised: "var(--destack-surface-raised)",
    /** Popovers, menus and dialogs above everything else. */
    overlay: "var(--destack-surface-overlay)",
});

/** Text styles at the medium text size with 16px body text, scaled by the person's text size. */
export const text = defineConsts({
    /** The interface font stack. */
    family: "var(--destack-text-family)",
    /** The code font stack. */
    codeFamily: "var(--destack-text-code-family)",
    /** Captions and labels. */
    captionFontFamily: "var(--destack-text-caption-font-family)",
    /** Captions and labels. */
    captionFontSize: "var(--destack-text-caption-font-size)",
    /** Captions and labels. */
    captionFontWeight: "var(--destack-text-caption-font-weight)",
    /** Captions and labels. */
    captionLineHeight: "var(--destack-text-caption-line-height)",
    /** Captions and labels. */
    captionLetterSpacing: "var(--destack-text-caption-letter-spacing)",
    /** Footnotes and secondary labels. */
    footnoteFontFamily: "var(--destack-text-footnote-font-family)",
    /** Footnotes and secondary labels. */
    footnoteFontSize: "var(--destack-text-footnote-font-size)",
    /** Footnotes and secondary labels. */
    footnoteFontWeight: "var(--destack-text-footnote-font-weight)",
    /** Footnotes and secondary labels. */
    footnoteLineHeight: "var(--destack-text-footnote-line-height)",
    /** Footnotes and secondary labels. */
    footnoteLetterSpacing: "var(--destack-text-footnote-letter-spacing)",
    /** Body text. */
    bodyFontFamily: "var(--destack-text-body-font-family)",
    /** Body text. */
    bodyFontSize: "var(--destack-text-body-font-size)",
    /** Body text. */
    bodyFontWeight: "var(--destack-text-body-font-weight)",
    /** Body text. */
    bodyLineHeight: "var(--destack-text-body-line-height)",
    /** Body text. */
    bodyLetterSpacing: "var(--destack-text-body-letter-spacing)",
    /** Callouts beside body text. */
    calloutFontFamily: "var(--destack-text-callout-font-family)",
    /** Callouts beside body text. */
    calloutFontSize: "var(--destack-text-callout-font-size)",
    /** Callouts beside body text. */
    calloutFontWeight: "var(--destack-text-callout-font-weight)",
    /** Callouts beside body text. */
    calloutLineHeight: "var(--destack-text-callout-line-height)",
    /** Callouts beside body text. */
    calloutLetterSpacing: "var(--destack-text-callout-letter-spacing)",
    /** Headlines within body text. */
    headlineFontFamily: "var(--destack-text-headline-font-family)",
    /** Headlines within body text. */
    headlineFontSize: "var(--destack-text-headline-font-size)",
    /** Headlines within body text. */
    headlineFontWeight: "var(--destack-text-headline-font-weight)",
    /** Headlines within body text. */
    headlineLineHeight: "var(--destack-text-headline-line-height)",
    /** Headlines within body text. */
    headlineLetterSpacing: "var(--destack-text-headline-letter-spacing)",
    /** First-level titles. */
    title1FontFamily: "var(--destack-text-title1-font-family)",
    /** First-level titles. */
    title1FontSize: "var(--destack-text-title1-font-size)",
    /** First-level titles. */
    title1FontWeight: "var(--destack-text-title1-font-weight)",
    /** First-level titles. */
    title1LineHeight: "var(--destack-text-title1-line-height)",
    /** First-level titles. */
    title1LetterSpacing: "var(--destack-text-title1-letter-spacing)",
    /** Second-level titles. */
    title2FontFamily: "var(--destack-text-title2-font-family)",
    /** Second-level titles. */
    title2FontSize: "var(--destack-text-title2-font-size)",
    /** Second-level titles. */
    title2FontWeight: "var(--destack-text-title2-font-weight)",
    /** Second-level titles. */
    title2LineHeight: "var(--destack-text-title2-line-height)",
    /** Second-level titles. */
    title2LetterSpacing: "var(--destack-text-title2-letter-spacing)",
    /** Third-level titles. */
    title3FontFamily: "var(--destack-text-title3-font-family)",
    /** Third-level titles. */
    title3FontSize: "var(--destack-text-title3-font-size)",
    /** Third-level titles. */
    title3FontWeight: "var(--destack-text-title3-font-weight)",
    /** Third-level titles. */
    title3LineHeight: "var(--destack-text-title3-line-height)",
    /** Third-level titles. */
    title3LetterSpacing: "var(--destack-text-title3-letter-spacing)",
    /** Large titles at the top of a view. */
    largeTitleFontFamily: "var(--destack-text-large-title-font-family)",
    /** Large titles at the top of a view. */
    largeTitleFontSize: "var(--destack-text-large-title-font-size)",
    /** Large titles at the top of a view. */
    largeTitleFontWeight: "var(--destack-text-large-title-font-weight)",
    /** Large titles at the top of a view. */
    largeTitleLineHeight: "var(--destack-text-large-title-line-height)",
    /** Large titles at the top of a view. */
    largeTitleLetterSpacing: "var(--destack-text-large-title-letter-spacing)",
});

/** Font weights after the CSS font-weight keywords and their common names. */
export const weight = defineConsts({
    /** Regular text, CSS normal. */
    regular: "var(--destack-weight-regular)",
    /** Controls and emphasised labels. */
    medium: "var(--destack-weight-medium)",
    /** Headings and strong emphasis. */
    semibold: "var(--destack-weight-semibold)",
    /** Bold text, CSS bold. */
    bold: "var(--destack-weight-bold)",
});

/** Spacing steps, scaled by the scaling and density. */
export const space = defineConsts({
    /** Spacing step 1. */
    "1": "var(--destack-space-1)",
    /** Spacing step 2. */
    "2": "var(--destack-space-2)",
    /** Spacing step 3. */
    "3": "var(--destack-space-3)",
    /** Spacing step 4. */
    "4": "var(--destack-space-4)",
    /** Spacing step 5. */
    "5": "var(--destack-space-5)",
    /** Spacing step 6. */
    "6": "var(--destack-space-6)",
    /** Spacing step 7. */
    "7": "var(--destack-space-7)",
    /** Spacing step 8. */
    "8": "var(--destack-space-8)",
    /** Spacing step 9. */
    "9": "var(--destack-space-9)",
});

/** Control heights, scaled by the scaling and density. */
export const size = defineConsts({
    /** Control height 1. */
    "1": "var(--destack-size-1)",
    /** Control height 2. */
    "2": "var(--destack-size-2)",
    /** Control height 3. */
    "3": "var(--destack-size-3)",
    /** Control height 4. */
    "4": "var(--destack-size-4)",
});

/** Widths of panels and overlays, scaled by the scaling and density. */
export const width = defineConsts({
    /** A popover's width. */
    popover: "var(--destack-width-popover)",
    /** A hover card's width. */
    hoverCard: "var(--destack-width-hover-card)",
    /** An expanded sidebar's width. */
    sidebar: "var(--destack-width-sidebar)",
    /** A sidebar's width while it shows its icons. */
    sidebarIcon: "var(--destack-width-sidebar-icon)",
    /** A toast's width. */
    toast: "var(--destack-width-toast)",
    /** The widest a block of centred text runs, such as an empty state's. */
    prose: "var(--destack-width-prose)",
    /** A vertical attachment tile's width. */
    tile: "var(--destack-width-tile)",
    /** The narrowest a horizontal attachment row runs. */
    row: "var(--destack-width-row)",
});

/** Corner radii, scaled by the scaling and the theme's radius. */
export const radius = defineConsts({
    /** Corner radius 1. */
    "1": "var(--destack-radius-1)",
    /** Corner radius 2. */
    "2": "var(--destack-radius-2)",
    /** Corner radius 3. */
    "3": "var(--destack-radius-3)",
    /** Corner radius 4. */
    "4": "var(--destack-radius-4)",
    /** Corner radius 5. */
    "5": "var(--destack-radius-5)",
    /** Corner radius 6. */
    "6": "var(--destack-radius-6)",
    /** Pill corners under the full radius, square corners otherwise. */
    full: "var(--destack-radius-full)",
});

/** Line widths of borders and focus rings, unscaled so hairlines stay sharp. */
export const stroke = defineConsts({
    /** Borders of controls, cards and separators. */
    border: "var(--destack-stroke-border)",
    /** Focus rings. */
    ring: "var(--destack-stroke-ring)",
});

/** Elevation shadows of the surface levels. */
export const shadow = defineConsts({
    /** The hairline around raised and overlaid surfaces, the gray's alpha steps 3 and 6. */
    edge: "var(--destack-shadow-edge)",
    /** The cast shadow color, black's alpha steps 2 and 6. */
    cast: "var(--destack-shadow-cast)",
    /** Sunken fields such as text inputs. */
    inset: "var(--destack-shadow-inset)",
    /** Raised surfaces such as cards and buttons. */
    raised: "var(--destack-shadow-raised)",
    /** Overlaid surfaces such as popovers and dialogs. */
    overlay: "var(--destack-shadow-overlay)",
});

/** Durations and easings, collapsed to zero by the person's motion setting. */
export const motion = defineConsts({
    /** Small changes such as hover and press. */
    durationShort: "var(--destack-motion-duration-short)",
    /** Elements entering and leaving. */
    durationMedium: "var(--destack-motion-duration-medium)",
    /** Large surfaces and full-screen changes. */
    durationLong: "var(--destack-motion-duration-long)",
    /** Movement within the screen. */
    easingStandard: "var(--destack-motion-easing-standard)",
    /** Elements entering the screen. */
    easingEmphasised: "var(--destack-motion-easing-emphasised)",
    /** Playful movement with a slight overshoot, sampled from a spring as linear(), with an overshooting cubic Bézier as its portable form. */
    easingSpring: "var(--destack-motion-easing-spring)",
});
