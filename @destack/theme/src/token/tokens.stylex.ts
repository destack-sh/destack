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
    /** The first chart series. */
    chart1: "var(--destack-color-chart1)",
    /** The second chart series. */
    chart2: "var(--destack-color-chart2)",
    /** The third chart series. */
    chart3: "var(--destack-color-chart3)",
    /** The fourth chart series. */
    chart4: "var(--destack-color-chart4)",
    /** The fifth chart series. */
    chart5: "var(--destack-color-chart5)",
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

/** One swatch per colorful preset, the colors an object marked with it takes, with the default theme's values. */
export const swatch = defineConsts({
    /** The tomato swatch's solid fill, such as a tag's dot. */
    tomatoSolid: "var(--destack-swatch-tomato-solid)",
    /** The label reading on the tomato swatch's solid fill. */
    tomatoLabel: "var(--destack-swatch-tomato-label)",
    /** The tomato swatch's subtle tint, such as a tag's background. */
    tomatoTint: "var(--destack-swatch-tomato-tint)",
    /** The text reading on the tomato swatch's tint. */
    tomatoText: "var(--destack-swatch-tomato-text)",
    /** The red swatch's solid fill, such as a tag's dot. */
    redSolid: "var(--destack-swatch-red-solid)",
    /** The label reading on the red swatch's solid fill. */
    redLabel: "var(--destack-swatch-red-label)",
    /** The red swatch's subtle tint, such as a tag's background. */
    redTint: "var(--destack-swatch-red-tint)",
    /** The text reading on the red swatch's tint. */
    redText: "var(--destack-swatch-red-text)",
    /** The ruby swatch's solid fill, such as a tag's dot. */
    rubySolid: "var(--destack-swatch-ruby-solid)",
    /** The label reading on the ruby swatch's solid fill. */
    rubyLabel: "var(--destack-swatch-ruby-label)",
    /** The ruby swatch's subtle tint, such as a tag's background. */
    rubyTint: "var(--destack-swatch-ruby-tint)",
    /** The text reading on the ruby swatch's tint. */
    rubyText: "var(--destack-swatch-ruby-text)",
    /** The crimson swatch's solid fill, such as a tag's dot. */
    crimsonSolid: "var(--destack-swatch-crimson-solid)",
    /** The label reading on the crimson swatch's solid fill. */
    crimsonLabel: "var(--destack-swatch-crimson-label)",
    /** The crimson swatch's subtle tint, such as a tag's background. */
    crimsonTint: "var(--destack-swatch-crimson-tint)",
    /** The text reading on the crimson swatch's tint. */
    crimsonText: "var(--destack-swatch-crimson-text)",
    /** The pink swatch's solid fill, such as a tag's dot. */
    pinkSolid: "var(--destack-swatch-pink-solid)",
    /** The label reading on the pink swatch's solid fill. */
    pinkLabel: "var(--destack-swatch-pink-label)",
    /** The pink swatch's subtle tint, such as a tag's background. */
    pinkTint: "var(--destack-swatch-pink-tint)",
    /** The text reading on the pink swatch's tint. */
    pinkText: "var(--destack-swatch-pink-text)",
    /** The plum swatch's solid fill, such as a tag's dot. */
    plumSolid: "var(--destack-swatch-plum-solid)",
    /** The label reading on the plum swatch's solid fill. */
    plumLabel: "var(--destack-swatch-plum-label)",
    /** The plum swatch's subtle tint, such as a tag's background. */
    plumTint: "var(--destack-swatch-plum-tint)",
    /** The text reading on the plum swatch's tint. */
    plumText: "var(--destack-swatch-plum-text)",
    /** The purple swatch's solid fill, such as a tag's dot. */
    purpleSolid: "var(--destack-swatch-purple-solid)",
    /** The label reading on the purple swatch's solid fill. */
    purpleLabel: "var(--destack-swatch-purple-label)",
    /** The purple swatch's subtle tint, such as a tag's background. */
    purpleTint: "var(--destack-swatch-purple-tint)",
    /** The text reading on the purple swatch's tint. */
    purpleText: "var(--destack-swatch-purple-text)",
    /** The violet swatch's solid fill, such as a tag's dot. */
    violetSolid: "var(--destack-swatch-violet-solid)",
    /** The label reading on the violet swatch's solid fill. */
    violetLabel: "var(--destack-swatch-violet-label)",
    /** The violet swatch's subtle tint, such as a tag's background. */
    violetTint: "var(--destack-swatch-violet-tint)",
    /** The text reading on the violet swatch's tint. */
    violetText: "var(--destack-swatch-violet-text)",
    /** The iris swatch's solid fill, such as a tag's dot. */
    irisSolid: "var(--destack-swatch-iris-solid)",
    /** The label reading on the iris swatch's solid fill. */
    irisLabel: "var(--destack-swatch-iris-label)",
    /** The iris swatch's subtle tint, such as a tag's background. */
    irisTint: "var(--destack-swatch-iris-tint)",
    /** The text reading on the iris swatch's tint. */
    irisText: "var(--destack-swatch-iris-text)",
    /** The indigo swatch's solid fill, such as a tag's dot. */
    indigoSolid: "var(--destack-swatch-indigo-solid)",
    /** The label reading on the indigo swatch's solid fill. */
    indigoLabel: "var(--destack-swatch-indigo-label)",
    /** The indigo swatch's subtle tint, such as a tag's background. */
    indigoTint: "var(--destack-swatch-indigo-tint)",
    /** The text reading on the indigo swatch's tint. */
    indigoText: "var(--destack-swatch-indigo-text)",
    /** The blue swatch's solid fill, such as a tag's dot. */
    blueSolid: "var(--destack-swatch-blue-solid)",
    /** The label reading on the blue swatch's solid fill. */
    blueLabel: "var(--destack-swatch-blue-label)",
    /** The blue swatch's subtle tint, such as a tag's background. */
    blueTint: "var(--destack-swatch-blue-tint)",
    /** The text reading on the blue swatch's tint. */
    blueText: "var(--destack-swatch-blue-text)",
    /** The cyan swatch's solid fill, such as a tag's dot. */
    cyanSolid: "var(--destack-swatch-cyan-solid)",
    /** The label reading on the cyan swatch's solid fill. */
    cyanLabel: "var(--destack-swatch-cyan-label)",
    /** The cyan swatch's subtle tint, such as a tag's background. */
    cyanTint: "var(--destack-swatch-cyan-tint)",
    /** The text reading on the cyan swatch's tint. */
    cyanText: "var(--destack-swatch-cyan-text)",
    /** The teal swatch's solid fill, such as a tag's dot. */
    tealSolid: "var(--destack-swatch-teal-solid)",
    /** The label reading on the teal swatch's solid fill. */
    tealLabel: "var(--destack-swatch-teal-label)",
    /** The teal swatch's subtle tint, such as a tag's background. */
    tealTint: "var(--destack-swatch-teal-tint)",
    /** The text reading on the teal swatch's tint. */
    tealText: "var(--destack-swatch-teal-text)",
    /** The jade swatch's solid fill, such as a tag's dot. */
    jadeSolid: "var(--destack-swatch-jade-solid)",
    /** The label reading on the jade swatch's solid fill. */
    jadeLabel: "var(--destack-swatch-jade-label)",
    /** The jade swatch's subtle tint, such as a tag's background. */
    jadeTint: "var(--destack-swatch-jade-tint)",
    /** The text reading on the jade swatch's tint. */
    jadeText: "var(--destack-swatch-jade-text)",
    /** The green swatch's solid fill, such as a tag's dot. */
    greenSolid: "var(--destack-swatch-green-solid)",
    /** The label reading on the green swatch's solid fill. */
    greenLabel: "var(--destack-swatch-green-label)",
    /** The green swatch's subtle tint, such as a tag's background. */
    greenTint: "var(--destack-swatch-green-tint)",
    /** The text reading on the green swatch's tint. */
    greenText: "var(--destack-swatch-green-text)",
    /** The grass swatch's solid fill, such as a tag's dot. */
    grassSolid: "var(--destack-swatch-grass-solid)",
    /** The label reading on the grass swatch's solid fill. */
    grassLabel: "var(--destack-swatch-grass-label)",
    /** The grass swatch's subtle tint, such as a tag's background. */
    grassTint: "var(--destack-swatch-grass-tint)",
    /** The text reading on the grass swatch's tint. */
    grassText: "var(--destack-swatch-grass-text)",
    /** The brown swatch's solid fill, such as a tag's dot. */
    brownSolid: "var(--destack-swatch-brown-solid)",
    /** The label reading on the brown swatch's solid fill. */
    brownLabel: "var(--destack-swatch-brown-label)",
    /** The brown swatch's subtle tint, such as a tag's background. */
    brownTint: "var(--destack-swatch-brown-tint)",
    /** The text reading on the brown swatch's tint. */
    brownText: "var(--destack-swatch-brown-text)",
    /** The bronze swatch's solid fill, such as a tag's dot. */
    bronzeSolid: "var(--destack-swatch-bronze-solid)",
    /** The label reading on the bronze swatch's solid fill. */
    bronzeLabel: "var(--destack-swatch-bronze-label)",
    /** The bronze swatch's subtle tint, such as a tag's background. */
    bronzeTint: "var(--destack-swatch-bronze-tint)",
    /** The text reading on the bronze swatch's tint. */
    bronzeText: "var(--destack-swatch-bronze-text)",
    /** The gold swatch's solid fill, such as a tag's dot. */
    goldSolid: "var(--destack-swatch-gold-solid)",
    /** The label reading on the gold swatch's solid fill. */
    goldLabel: "var(--destack-swatch-gold-label)",
    /** The gold swatch's subtle tint, such as a tag's background. */
    goldTint: "var(--destack-swatch-gold-tint)",
    /** The text reading on the gold swatch's tint. */
    goldText: "var(--destack-swatch-gold-text)",
    /** The sky swatch's solid fill, such as a tag's dot. */
    skySolid: "var(--destack-swatch-sky-solid)",
    /** The label reading on the sky swatch's solid fill. */
    skyLabel: "var(--destack-swatch-sky-label)",
    /** The sky swatch's subtle tint, such as a tag's background. */
    skyTint: "var(--destack-swatch-sky-tint)",
    /** The text reading on the sky swatch's tint. */
    skyText: "var(--destack-swatch-sky-text)",
    /** The mint swatch's solid fill, such as a tag's dot. */
    mintSolid: "var(--destack-swatch-mint-solid)",
    /** The label reading on the mint swatch's solid fill. */
    mintLabel: "var(--destack-swatch-mint-label)",
    /** The mint swatch's subtle tint, such as a tag's background. */
    mintTint: "var(--destack-swatch-mint-tint)",
    /** The text reading on the mint swatch's tint. */
    mintText: "var(--destack-swatch-mint-text)",
    /** The lime swatch's solid fill, such as a tag's dot. */
    limeSolid: "var(--destack-swatch-lime-solid)",
    /** The label reading on the lime swatch's solid fill. */
    limeLabel: "var(--destack-swatch-lime-label)",
    /** The lime swatch's subtle tint, such as a tag's background. */
    limeTint: "var(--destack-swatch-lime-tint)",
    /** The text reading on the lime swatch's tint. */
    limeText: "var(--destack-swatch-lime-text)",
    /** The yellow swatch's solid fill, such as a tag's dot. */
    yellowSolid: "var(--destack-swatch-yellow-solid)",
    /** The label reading on the yellow swatch's solid fill. */
    yellowLabel: "var(--destack-swatch-yellow-label)",
    /** The yellow swatch's subtle tint, such as a tag's background. */
    yellowTint: "var(--destack-swatch-yellow-tint)",
    /** The text reading on the yellow swatch's tint. */
    yellowText: "var(--destack-swatch-yellow-text)",
    /** The amber swatch's solid fill, such as a tag's dot. */
    amberSolid: "var(--destack-swatch-amber-solid)",
    /** The label reading on the amber swatch's solid fill. */
    amberLabel: "var(--destack-swatch-amber-label)",
    /** The amber swatch's subtle tint, such as a tag's background. */
    amberTint: "var(--destack-swatch-amber-tint)",
    /** The text reading on the amber swatch's tint. */
    amberText: "var(--destack-swatch-amber-text)",
    /** The orange swatch's solid fill, such as a tag's dot. */
    orangeSolid: "var(--destack-swatch-orange-solid)",
    /** The label reading on the orange swatch's solid fill. */
    orangeLabel: "var(--destack-swatch-orange-label)",
    /** The orange swatch's subtle tint, such as a tag's background. */
    orangeTint: "var(--destack-swatch-orange-tint)",
    /** The text reading on the orange swatch's tint. */
    orangeText: "var(--destack-swatch-orange-text)",
});

/** Font stacks, which text styles and code take. */
export const font = defineConsts({
    /** The interface font stack. */
    text: "var(--destack-font-text)",
    /** The code font stack. */
    code: "var(--destack-font-code)",
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
