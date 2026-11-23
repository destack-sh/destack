// generate with `bun run generate` from tokens.json

import { defineConsts } from "@destack/style";

/** Neutral color roles, with the default theme's values. */
export const color = defineConsts({
    /** The page background. */
    background: "var(--destack-color-background)",
    /** Cards and panels raised above the page. */
    card: "var(--destack-color-card)",
    /** Popovers, menus and dialogs above everything else. */
    popover: "var(--destack-color-popover)",
    /** Body text on every surface. */
    foreground: "var(--destack-color-foreground)",
    /** Secondary text on every surface, such as descriptions. */
    mutedForeground: "var(--destack-color-muted-foreground)",
    /** Quiet fills, such as a code block's background. */
    muted: "var(--destack-color-muted)",
    /** The layer of the text color over a hovered item. */
    hover: "var(--destack-color-hover)",
    /** The layer of the text color over a selected or highlighted item. */
    selected: "var(--destack-color-selected)",
    /** Borders and separators. */
    border: "var(--destack-color-border)",
    /** Borders of form controls. */
    input: "var(--destack-color-input)",
    /** Focus rings. */
    ring: "var(--destack-color-ring)",
    /** The veil behind dialogs, sheets and drawers. */
    scrim: "var(--destack-color-scrim)",
});

/** One swatch per hue: the accent, the statuses, the chart series and every colorful preset. */
export const swatch = defineConsts({
    /** The accent swatch's solid fill, such as a filled button. */
    accentSolid: "var(--destack-swatch-accent-solid)",
    /** The label reading on the accent swatch's solid fill. */
    accentLabel: "var(--destack-swatch-accent-label)",
    /** The accent swatch's tint, such as a tag's background. */
    accentTint: "var(--destack-swatch-accent-tint)",
    /** The text reading on the accent swatch's tint. */
    accentTintText: "var(--destack-swatch-accent-tint-text)",
    /** The accent swatch's text on every surface, such as a link. */
    accentText: "var(--destack-swatch-accent-text)",
    /** The accent swatch's graphics on the page, such as a chart's line or a tag's dot. */
    accentMark: "var(--destack-swatch-accent-mark)",
    /** The destructive swatch's solid fill, such as a filled button. */
    destructiveSolid: "var(--destack-swatch-destructive-solid)",
    /** The label reading on the destructive swatch's solid fill. */
    destructiveLabel: "var(--destack-swatch-destructive-label)",
    /** The destructive swatch's tint, such as a tag's background. */
    destructiveTint: "var(--destack-swatch-destructive-tint)",
    /** The text reading on the destructive swatch's tint. */
    destructiveTintText: "var(--destack-swatch-destructive-tint-text)",
    /** The destructive swatch's text on every surface, such as a link. */
    destructiveText: "var(--destack-swatch-destructive-text)",
    /** The destructive swatch's graphics on the page, such as a chart's line or a tag's dot. */
    destructiveMark: "var(--destack-swatch-destructive-mark)",
    /** The success swatch's solid fill, such as a filled button. */
    successSolid: "var(--destack-swatch-success-solid)",
    /** The label reading on the success swatch's solid fill. */
    successLabel: "var(--destack-swatch-success-label)",
    /** The success swatch's tint, such as a tag's background. */
    successTint: "var(--destack-swatch-success-tint)",
    /** The text reading on the success swatch's tint. */
    successTintText: "var(--destack-swatch-success-tint-text)",
    /** The success swatch's text on every surface, such as a link. */
    successText: "var(--destack-swatch-success-text)",
    /** The success swatch's graphics on the page, such as a chart's line or a tag's dot. */
    successMark: "var(--destack-swatch-success-mark)",
    /** The warning swatch's solid fill, such as a filled button. */
    warningSolid: "var(--destack-swatch-warning-solid)",
    /** The label reading on the warning swatch's solid fill. */
    warningLabel: "var(--destack-swatch-warning-label)",
    /** The warning swatch's tint, such as a tag's background. */
    warningTint: "var(--destack-swatch-warning-tint)",
    /** The text reading on the warning swatch's tint. */
    warningTintText: "var(--destack-swatch-warning-tint-text)",
    /** The warning swatch's text on every surface, such as a link. */
    warningText: "var(--destack-swatch-warning-text)",
    /** The warning swatch's graphics on the page, such as a chart's line or a tag's dot. */
    warningMark: "var(--destack-swatch-warning-mark)",
    /** The info swatch's solid fill, such as a filled button. */
    infoSolid: "var(--destack-swatch-info-solid)",
    /** The label reading on the info swatch's solid fill. */
    infoLabel: "var(--destack-swatch-info-label)",
    /** The info swatch's tint, such as a tag's background. */
    infoTint: "var(--destack-swatch-info-tint)",
    /** The text reading on the info swatch's tint. */
    infoTintText: "var(--destack-swatch-info-tint-text)",
    /** The info swatch's text on every surface, such as a link. */
    infoText: "var(--destack-swatch-info-text)",
    /** The info swatch's graphics on the page, such as a chart's line or a tag's dot. */
    infoMark: "var(--destack-swatch-info-mark)",
    /** The series 1 swatch's solid fill, such as a filled button. */
    series1Solid: "var(--destack-swatch-series1-solid)",
    /** The label reading on the series 1 swatch's solid fill. */
    series1Label: "var(--destack-swatch-series1-label)",
    /** The series 1 swatch's tint, such as a tag's background. */
    series1Tint: "var(--destack-swatch-series1-tint)",
    /** The text reading on the series 1 swatch's tint. */
    series1TintText: "var(--destack-swatch-series1-tint-text)",
    /** The series 1 swatch's text on every surface, such as a link. */
    series1Text: "var(--destack-swatch-series1-text)",
    /** The series 1 swatch's graphics on the page, such as a chart's line or a tag's dot. */
    series1Mark: "var(--destack-swatch-series1-mark)",
    /** The series 2 swatch's solid fill, such as a filled button. */
    series2Solid: "var(--destack-swatch-series2-solid)",
    /** The label reading on the series 2 swatch's solid fill. */
    series2Label: "var(--destack-swatch-series2-label)",
    /** The series 2 swatch's tint, such as a tag's background. */
    series2Tint: "var(--destack-swatch-series2-tint)",
    /** The text reading on the series 2 swatch's tint. */
    series2TintText: "var(--destack-swatch-series2-tint-text)",
    /** The series 2 swatch's text on every surface, such as a link. */
    series2Text: "var(--destack-swatch-series2-text)",
    /** The series 2 swatch's graphics on the page, such as a chart's line or a tag's dot. */
    series2Mark: "var(--destack-swatch-series2-mark)",
    /** The series 3 swatch's solid fill, such as a filled button. */
    series3Solid: "var(--destack-swatch-series3-solid)",
    /** The label reading on the series 3 swatch's solid fill. */
    series3Label: "var(--destack-swatch-series3-label)",
    /** The series 3 swatch's tint, such as a tag's background. */
    series3Tint: "var(--destack-swatch-series3-tint)",
    /** The text reading on the series 3 swatch's tint. */
    series3TintText: "var(--destack-swatch-series3-tint-text)",
    /** The series 3 swatch's text on every surface, such as a link. */
    series3Text: "var(--destack-swatch-series3-text)",
    /** The series 3 swatch's graphics on the page, such as a chart's line or a tag's dot. */
    series3Mark: "var(--destack-swatch-series3-mark)",
    /** The series 4 swatch's solid fill, such as a filled button. */
    series4Solid: "var(--destack-swatch-series4-solid)",
    /** The label reading on the series 4 swatch's solid fill. */
    series4Label: "var(--destack-swatch-series4-label)",
    /** The series 4 swatch's tint, such as a tag's background. */
    series4Tint: "var(--destack-swatch-series4-tint)",
    /** The text reading on the series 4 swatch's tint. */
    series4TintText: "var(--destack-swatch-series4-tint-text)",
    /** The series 4 swatch's text on every surface, such as a link. */
    series4Text: "var(--destack-swatch-series4-text)",
    /** The series 4 swatch's graphics on the page, such as a chart's line or a tag's dot. */
    series4Mark: "var(--destack-swatch-series4-mark)",
    /** The series 5 swatch's solid fill, such as a filled button. */
    series5Solid: "var(--destack-swatch-series5-solid)",
    /** The label reading on the series 5 swatch's solid fill. */
    series5Label: "var(--destack-swatch-series5-label)",
    /** The series 5 swatch's tint, such as a tag's background. */
    series5Tint: "var(--destack-swatch-series5-tint)",
    /** The text reading on the series 5 swatch's tint. */
    series5TintText: "var(--destack-swatch-series5-tint-text)",
    /** The series 5 swatch's text on every surface, such as a link. */
    series5Text: "var(--destack-swatch-series5-text)",
    /** The series 5 swatch's graphics on the page, such as a chart's line or a tag's dot. */
    series5Mark: "var(--destack-swatch-series5-mark)",
    /** The tomato swatch's solid fill, such as a filled button. */
    tomatoSolid: "var(--destack-swatch-tomato-solid)",
    /** The label reading on the tomato swatch's solid fill. */
    tomatoLabel: "var(--destack-swatch-tomato-label)",
    /** The tomato swatch's tint, such as a tag's background. */
    tomatoTint: "var(--destack-swatch-tomato-tint)",
    /** The text reading on the tomato swatch's tint. */
    tomatoTintText: "var(--destack-swatch-tomato-tint-text)",
    /** The tomato swatch's text on every surface, such as a link. */
    tomatoText: "var(--destack-swatch-tomato-text)",
    /** The tomato swatch's graphics on the page, such as a chart's line or a tag's dot. */
    tomatoMark: "var(--destack-swatch-tomato-mark)",
    /** The red swatch's solid fill, such as a filled button. */
    redSolid: "var(--destack-swatch-red-solid)",
    /** The label reading on the red swatch's solid fill. */
    redLabel: "var(--destack-swatch-red-label)",
    /** The red swatch's tint, such as a tag's background. */
    redTint: "var(--destack-swatch-red-tint)",
    /** The text reading on the red swatch's tint. */
    redTintText: "var(--destack-swatch-red-tint-text)",
    /** The red swatch's text on every surface, such as a link. */
    redText: "var(--destack-swatch-red-text)",
    /** The red swatch's graphics on the page, such as a chart's line or a tag's dot. */
    redMark: "var(--destack-swatch-red-mark)",
    /** The ruby swatch's solid fill, such as a filled button. */
    rubySolid: "var(--destack-swatch-ruby-solid)",
    /** The label reading on the ruby swatch's solid fill. */
    rubyLabel: "var(--destack-swatch-ruby-label)",
    /** The ruby swatch's tint, such as a tag's background. */
    rubyTint: "var(--destack-swatch-ruby-tint)",
    /** The text reading on the ruby swatch's tint. */
    rubyTintText: "var(--destack-swatch-ruby-tint-text)",
    /** The ruby swatch's text on every surface, such as a link. */
    rubyText: "var(--destack-swatch-ruby-text)",
    /** The ruby swatch's graphics on the page, such as a chart's line or a tag's dot. */
    rubyMark: "var(--destack-swatch-ruby-mark)",
    /** The crimson swatch's solid fill, such as a filled button. */
    crimsonSolid: "var(--destack-swatch-crimson-solid)",
    /** The label reading on the crimson swatch's solid fill. */
    crimsonLabel: "var(--destack-swatch-crimson-label)",
    /** The crimson swatch's tint, such as a tag's background. */
    crimsonTint: "var(--destack-swatch-crimson-tint)",
    /** The text reading on the crimson swatch's tint. */
    crimsonTintText: "var(--destack-swatch-crimson-tint-text)",
    /** The crimson swatch's text on every surface, such as a link. */
    crimsonText: "var(--destack-swatch-crimson-text)",
    /** The crimson swatch's graphics on the page, such as a chart's line or a tag's dot. */
    crimsonMark: "var(--destack-swatch-crimson-mark)",
    /** The pink swatch's solid fill, such as a filled button. */
    pinkSolid: "var(--destack-swatch-pink-solid)",
    /** The label reading on the pink swatch's solid fill. */
    pinkLabel: "var(--destack-swatch-pink-label)",
    /** The pink swatch's tint, such as a tag's background. */
    pinkTint: "var(--destack-swatch-pink-tint)",
    /** The text reading on the pink swatch's tint. */
    pinkTintText: "var(--destack-swatch-pink-tint-text)",
    /** The pink swatch's text on every surface, such as a link. */
    pinkText: "var(--destack-swatch-pink-text)",
    /** The pink swatch's graphics on the page, such as a chart's line or a tag's dot. */
    pinkMark: "var(--destack-swatch-pink-mark)",
    /** The plum swatch's solid fill, such as a filled button. */
    plumSolid: "var(--destack-swatch-plum-solid)",
    /** The label reading on the plum swatch's solid fill. */
    plumLabel: "var(--destack-swatch-plum-label)",
    /** The plum swatch's tint, such as a tag's background. */
    plumTint: "var(--destack-swatch-plum-tint)",
    /** The text reading on the plum swatch's tint. */
    plumTintText: "var(--destack-swatch-plum-tint-text)",
    /** The plum swatch's text on every surface, such as a link. */
    plumText: "var(--destack-swatch-plum-text)",
    /** The plum swatch's graphics on the page, such as a chart's line or a tag's dot. */
    plumMark: "var(--destack-swatch-plum-mark)",
    /** The purple swatch's solid fill, such as a filled button. */
    purpleSolid: "var(--destack-swatch-purple-solid)",
    /** The label reading on the purple swatch's solid fill. */
    purpleLabel: "var(--destack-swatch-purple-label)",
    /** The purple swatch's tint, such as a tag's background. */
    purpleTint: "var(--destack-swatch-purple-tint)",
    /** The text reading on the purple swatch's tint. */
    purpleTintText: "var(--destack-swatch-purple-tint-text)",
    /** The purple swatch's text on every surface, such as a link. */
    purpleText: "var(--destack-swatch-purple-text)",
    /** The purple swatch's graphics on the page, such as a chart's line or a tag's dot. */
    purpleMark: "var(--destack-swatch-purple-mark)",
    /** The violet swatch's solid fill, such as a filled button. */
    violetSolid: "var(--destack-swatch-violet-solid)",
    /** The label reading on the violet swatch's solid fill. */
    violetLabel: "var(--destack-swatch-violet-label)",
    /** The violet swatch's tint, such as a tag's background. */
    violetTint: "var(--destack-swatch-violet-tint)",
    /** The text reading on the violet swatch's tint. */
    violetTintText: "var(--destack-swatch-violet-tint-text)",
    /** The violet swatch's text on every surface, such as a link. */
    violetText: "var(--destack-swatch-violet-text)",
    /** The violet swatch's graphics on the page, such as a chart's line or a tag's dot. */
    violetMark: "var(--destack-swatch-violet-mark)",
    /** The iris swatch's solid fill, such as a filled button. */
    irisSolid: "var(--destack-swatch-iris-solid)",
    /** The label reading on the iris swatch's solid fill. */
    irisLabel: "var(--destack-swatch-iris-label)",
    /** The iris swatch's tint, such as a tag's background. */
    irisTint: "var(--destack-swatch-iris-tint)",
    /** The text reading on the iris swatch's tint. */
    irisTintText: "var(--destack-swatch-iris-tint-text)",
    /** The iris swatch's text on every surface, such as a link. */
    irisText: "var(--destack-swatch-iris-text)",
    /** The iris swatch's graphics on the page, such as a chart's line or a tag's dot. */
    irisMark: "var(--destack-swatch-iris-mark)",
    /** The indigo swatch's solid fill, such as a filled button. */
    indigoSolid: "var(--destack-swatch-indigo-solid)",
    /** The label reading on the indigo swatch's solid fill. */
    indigoLabel: "var(--destack-swatch-indigo-label)",
    /** The indigo swatch's tint, such as a tag's background. */
    indigoTint: "var(--destack-swatch-indigo-tint)",
    /** The text reading on the indigo swatch's tint. */
    indigoTintText: "var(--destack-swatch-indigo-tint-text)",
    /** The indigo swatch's text on every surface, such as a link. */
    indigoText: "var(--destack-swatch-indigo-text)",
    /** The indigo swatch's graphics on the page, such as a chart's line or a tag's dot. */
    indigoMark: "var(--destack-swatch-indigo-mark)",
    /** The blue swatch's solid fill, such as a filled button. */
    blueSolid: "var(--destack-swatch-blue-solid)",
    /** The label reading on the blue swatch's solid fill. */
    blueLabel: "var(--destack-swatch-blue-label)",
    /** The blue swatch's tint, such as a tag's background. */
    blueTint: "var(--destack-swatch-blue-tint)",
    /** The text reading on the blue swatch's tint. */
    blueTintText: "var(--destack-swatch-blue-tint-text)",
    /** The blue swatch's text on every surface, such as a link. */
    blueText: "var(--destack-swatch-blue-text)",
    /** The blue swatch's graphics on the page, such as a chart's line or a tag's dot. */
    blueMark: "var(--destack-swatch-blue-mark)",
    /** The cyan swatch's solid fill, such as a filled button. */
    cyanSolid: "var(--destack-swatch-cyan-solid)",
    /** The label reading on the cyan swatch's solid fill. */
    cyanLabel: "var(--destack-swatch-cyan-label)",
    /** The cyan swatch's tint, such as a tag's background. */
    cyanTint: "var(--destack-swatch-cyan-tint)",
    /** The text reading on the cyan swatch's tint. */
    cyanTintText: "var(--destack-swatch-cyan-tint-text)",
    /** The cyan swatch's text on every surface, such as a link. */
    cyanText: "var(--destack-swatch-cyan-text)",
    /** The cyan swatch's graphics on the page, such as a chart's line or a tag's dot. */
    cyanMark: "var(--destack-swatch-cyan-mark)",
    /** The teal swatch's solid fill, such as a filled button. */
    tealSolid: "var(--destack-swatch-teal-solid)",
    /** The label reading on the teal swatch's solid fill. */
    tealLabel: "var(--destack-swatch-teal-label)",
    /** The teal swatch's tint, such as a tag's background. */
    tealTint: "var(--destack-swatch-teal-tint)",
    /** The text reading on the teal swatch's tint. */
    tealTintText: "var(--destack-swatch-teal-tint-text)",
    /** The teal swatch's text on every surface, such as a link. */
    tealText: "var(--destack-swatch-teal-text)",
    /** The teal swatch's graphics on the page, such as a chart's line or a tag's dot. */
    tealMark: "var(--destack-swatch-teal-mark)",
    /** The jade swatch's solid fill, such as a filled button. */
    jadeSolid: "var(--destack-swatch-jade-solid)",
    /** The label reading on the jade swatch's solid fill. */
    jadeLabel: "var(--destack-swatch-jade-label)",
    /** The jade swatch's tint, such as a tag's background. */
    jadeTint: "var(--destack-swatch-jade-tint)",
    /** The text reading on the jade swatch's tint. */
    jadeTintText: "var(--destack-swatch-jade-tint-text)",
    /** The jade swatch's text on every surface, such as a link. */
    jadeText: "var(--destack-swatch-jade-text)",
    /** The jade swatch's graphics on the page, such as a chart's line or a tag's dot. */
    jadeMark: "var(--destack-swatch-jade-mark)",
    /** The green swatch's solid fill, such as a filled button. */
    greenSolid: "var(--destack-swatch-green-solid)",
    /** The label reading on the green swatch's solid fill. */
    greenLabel: "var(--destack-swatch-green-label)",
    /** The green swatch's tint, such as a tag's background. */
    greenTint: "var(--destack-swatch-green-tint)",
    /** The text reading on the green swatch's tint. */
    greenTintText: "var(--destack-swatch-green-tint-text)",
    /** The green swatch's text on every surface, such as a link. */
    greenText: "var(--destack-swatch-green-text)",
    /** The green swatch's graphics on the page, such as a chart's line or a tag's dot. */
    greenMark: "var(--destack-swatch-green-mark)",
    /** The grass swatch's solid fill, such as a filled button. */
    grassSolid: "var(--destack-swatch-grass-solid)",
    /** The label reading on the grass swatch's solid fill. */
    grassLabel: "var(--destack-swatch-grass-label)",
    /** The grass swatch's tint, such as a tag's background. */
    grassTint: "var(--destack-swatch-grass-tint)",
    /** The text reading on the grass swatch's tint. */
    grassTintText: "var(--destack-swatch-grass-tint-text)",
    /** The grass swatch's text on every surface, such as a link. */
    grassText: "var(--destack-swatch-grass-text)",
    /** The grass swatch's graphics on the page, such as a chart's line or a tag's dot. */
    grassMark: "var(--destack-swatch-grass-mark)",
    /** The brown swatch's solid fill, such as a filled button. */
    brownSolid: "var(--destack-swatch-brown-solid)",
    /** The label reading on the brown swatch's solid fill. */
    brownLabel: "var(--destack-swatch-brown-label)",
    /** The brown swatch's tint, such as a tag's background. */
    brownTint: "var(--destack-swatch-brown-tint)",
    /** The text reading on the brown swatch's tint. */
    brownTintText: "var(--destack-swatch-brown-tint-text)",
    /** The brown swatch's text on every surface, such as a link. */
    brownText: "var(--destack-swatch-brown-text)",
    /** The brown swatch's graphics on the page, such as a chart's line or a tag's dot. */
    brownMark: "var(--destack-swatch-brown-mark)",
    /** The bronze swatch's solid fill, such as a filled button. */
    bronzeSolid: "var(--destack-swatch-bronze-solid)",
    /** The label reading on the bronze swatch's solid fill. */
    bronzeLabel: "var(--destack-swatch-bronze-label)",
    /** The bronze swatch's tint, such as a tag's background. */
    bronzeTint: "var(--destack-swatch-bronze-tint)",
    /** The text reading on the bronze swatch's tint. */
    bronzeTintText: "var(--destack-swatch-bronze-tint-text)",
    /** The bronze swatch's text on every surface, such as a link. */
    bronzeText: "var(--destack-swatch-bronze-text)",
    /** The bronze swatch's graphics on the page, such as a chart's line or a tag's dot. */
    bronzeMark: "var(--destack-swatch-bronze-mark)",
    /** The gold swatch's solid fill, such as a filled button. */
    goldSolid: "var(--destack-swatch-gold-solid)",
    /** The label reading on the gold swatch's solid fill. */
    goldLabel: "var(--destack-swatch-gold-label)",
    /** The gold swatch's tint, such as a tag's background. */
    goldTint: "var(--destack-swatch-gold-tint)",
    /** The text reading on the gold swatch's tint. */
    goldTintText: "var(--destack-swatch-gold-tint-text)",
    /** The gold swatch's text on every surface, such as a link. */
    goldText: "var(--destack-swatch-gold-text)",
    /** The gold swatch's graphics on the page, such as a chart's line or a tag's dot. */
    goldMark: "var(--destack-swatch-gold-mark)",
    /** The sky swatch's solid fill, such as a filled button. */
    skySolid: "var(--destack-swatch-sky-solid)",
    /** The label reading on the sky swatch's solid fill. */
    skyLabel: "var(--destack-swatch-sky-label)",
    /** The sky swatch's tint, such as a tag's background. */
    skyTint: "var(--destack-swatch-sky-tint)",
    /** The text reading on the sky swatch's tint. */
    skyTintText: "var(--destack-swatch-sky-tint-text)",
    /** The sky swatch's text on every surface, such as a link. */
    skyText: "var(--destack-swatch-sky-text)",
    /** The sky swatch's graphics on the page, such as a chart's line or a tag's dot. */
    skyMark: "var(--destack-swatch-sky-mark)",
    /** The mint swatch's solid fill, such as a filled button. */
    mintSolid: "var(--destack-swatch-mint-solid)",
    /** The label reading on the mint swatch's solid fill. */
    mintLabel: "var(--destack-swatch-mint-label)",
    /** The mint swatch's tint, such as a tag's background. */
    mintTint: "var(--destack-swatch-mint-tint)",
    /** The text reading on the mint swatch's tint. */
    mintTintText: "var(--destack-swatch-mint-tint-text)",
    /** The mint swatch's text on every surface, such as a link. */
    mintText: "var(--destack-swatch-mint-text)",
    /** The mint swatch's graphics on the page, such as a chart's line or a tag's dot. */
    mintMark: "var(--destack-swatch-mint-mark)",
    /** The lime swatch's solid fill, such as a filled button. */
    limeSolid: "var(--destack-swatch-lime-solid)",
    /** The label reading on the lime swatch's solid fill. */
    limeLabel: "var(--destack-swatch-lime-label)",
    /** The lime swatch's tint, such as a tag's background. */
    limeTint: "var(--destack-swatch-lime-tint)",
    /** The text reading on the lime swatch's tint. */
    limeTintText: "var(--destack-swatch-lime-tint-text)",
    /** The lime swatch's text on every surface, such as a link. */
    limeText: "var(--destack-swatch-lime-text)",
    /** The lime swatch's graphics on the page, such as a chart's line or a tag's dot. */
    limeMark: "var(--destack-swatch-lime-mark)",
    /** The yellow swatch's solid fill, such as a filled button. */
    yellowSolid: "var(--destack-swatch-yellow-solid)",
    /** The label reading on the yellow swatch's solid fill. */
    yellowLabel: "var(--destack-swatch-yellow-label)",
    /** The yellow swatch's tint, such as a tag's background. */
    yellowTint: "var(--destack-swatch-yellow-tint)",
    /** The text reading on the yellow swatch's tint. */
    yellowTintText: "var(--destack-swatch-yellow-tint-text)",
    /** The yellow swatch's text on every surface, such as a link. */
    yellowText: "var(--destack-swatch-yellow-text)",
    /** The yellow swatch's graphics on the page, such as a chart's line or a tag's dot. */
    yellowMark: "var(--destack-swatch-yellow-mark)",
    /** The amber swatch's solid fill, such as a filled button. */
    amberSolid: "var(--destack-swatch-amber-solid)",
    /** The label reading on the amber swatch's solid fill. */
    amberLabel: "var(--destack-swatch-amber-label)",
    /** The amber swatch's tint, such as a tag's background. */
    amberTint: "var(--destack-swatch-amber-tint)",
    /** The text reading on the amber swatch's tint. */
    amberTintText: "var(--destack-swatch-amber-tint-text)",
    /** The amber swatch's text on every surface, such as a link. */
    amberText: "var(--destack-swatch-amber-text)",
    /** The amber swatch's graphics on the page, such as a chart's line or a tag's dot. */
    amberMark: "var(--destack-swatch-amber-mark)",
    /** The orange swatch's solid fill, such as a filled button. */
    orangeSolid: "var(--destack-swatch-orange-solid)",
    /** The label reading on the orange swatch's solid fill. */
    orangeLabel: "var(--destack-swatch-orange-label)",
    /** The orange swatch's tint, such as a tag's background. */
    orangeTint: "var(--destack-swatch-orange-tint)",
    /** The text reading on the orange swatch's tint. */
    orangeTintText: "var(--destack-swatch-orange-tint-text)",
    /** The orange swatch's text on every surface, such as a link. */
    orangeText: "var(--destack-swatch-orange-text)",
    /** The orange swatch's graphics on the page, such as a chart's line or a tag's dot. */
    orangeMark: "var(--destack-swatch-orange-mark)",
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
    /** A board column's width, as Trello's lists run. */
    column: "var(--destack-width-column)",
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
    /** The hairline around raised and overlaid surfaces, the gray's alpha step 3. */
    edge: "var(--destack-shadow-edge)",
    /** The cast shadow color, black's alpha steps 2 and 6. */
    cast: "var(--destack-shadow-cast)",
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
    /** Playful movement with a slight overshoot, sampled from a spring as linear(). */
    easingSpring: "var(--destack-motion-easing-spring)",
});
