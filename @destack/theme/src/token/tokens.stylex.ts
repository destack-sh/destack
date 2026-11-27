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
    /** Sidebars beside the page, a shade off it. */
    sidebar: "var(--destack-color-sidebar)",
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

/** One swatch per hue: the accent, the statuses, the chart series and the hues. */
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
    /** The gray swatch's solid fill, such as a filled button. */
    graySolid: "var(--destack-swatch-gray-solid)",
    /** The label reading on the gray swatch's solid fill. */
    grayLabel: "var(--destack-swatch-gray-label)",
    /** The gray swatch's tint, such as a tag's background. */
    grayTint: "var(--destack-swatch-gray-tint)",
    /** The text reading on the gray swatch's tint. */
    grayTintText: "var(--destack-swatch-gray-tint-text)",
    /** The gray swatch's text on every surface, such as a link. */
    grayText: "var(--destack-swatch-gray-text)",
    /** The gray swatch's graphics on the page, such as a chart's line or a tag's dot. */
    grayMark: "var(--destack-swatch-gray-mark)",
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
});

/** Font stacks, which text styles and code take. */
export const font = defineConsts({
    /** The interface font stack. */
    text: "var(--destack-font-text)",
    /** The code font stack. */
    code: "var(--destack-font-code)",
});

/** Text styles after Tailwind's type scale, scaled by the person's text size. */
export const text = defineConsts({
    /** Extra small text, such as badges, shortcuts and labels. */
    xsFontFamily: "var(--destack-text-xs-font-family)",
    /** Extra small text, such as badges, shortcuts and labels. */
    xsFontSize: "var(--destack-text-xs-font-size)",
    /** Extra small text, such as badges, shortcuts and labels. */
    xsFontWeight: "var(--destack-text-xs-font-weight)",
    /** Extra small text, such as badges, shortcuts and labels. */
    xsLineHeight: "var(--destack-text-xs-line-height)",
    /** Extra small text, such as badges, shortcuts and labels. */
    xsLetterSpacing: "var(--destack-text-xs-letter-spacing)",
    /** Extra small text, such as badges, shortcuts and labels. */
    xsFontSizeAdjust: "var(--destack-text-xs-font-size-adjust)",
    /** Small text, the default in controls and lists. */
    smFontFamily: "var(--destack-text-sm-font-family)",
    /** Small text, the default in controls and lists. */
    smFontSize: "var(--destack-text-sm-font-size)",
    /** Small text, the default in controls and lists. */
    smFontWeight: "var(--destack-text-sm-font-weight)",
    /** Small text, the default in controls and lists. */
    smLineHeight: "var(--destack-text-sm-line-height)",
    /** Small text, the default in controls and lists. */
    smLetterSpacing: "var(--destack-text-sm-letter-spacing)",
    /** Small text, the default in controls and lists. */
    smFontSizeAdjust: "var(--destack-text-sm-font-size-adjust)",
    /** Base text, for reading. */
    baseFontFamily: "var(--destack-text-base-font-family)",
    /** Base text, for reading. */
    baseFontSize: "var(--destack-text-base-font-size)",
    /** Base text, for reading. */
    baseFontWeight: "var(--destack-text-base-font-weight)",
    /** Base text, for reading. */
    baseLineHeight: "var(--destack-text-base-line-height)",
    /** Base text, for reading. */
    baseLetterSpacing: "var(--destack-text-base-letter-spacing)",
    /** Base text, for reading. */
    baseFontSizeAdjust: "var(--destack-text-base-font-size-adjust)",
    /** Large text, such as a dialog's title. */
    lgFontFamily: "var(--destack-text-lg-font-family)",
    /** Large text, such as a dialog's title. */
    lgFontSize: "var(--destack-text-lg-font-size)",
    /** Large text, such as a dialog's title. */
    lgFontWeight: "var(--destack-text-lg-font-weight)",
    /** Large text, such as a dialog's title. */
    lgLineHeight: "var(--destack-text-lg-line-height)",
    /** Large text, such as a dialog's title. */
    lgLetterSpacing: "var(--destack-text-lg-letter-spacing)",
    /** Large text, such as a dialog's title. */
    lgFontSizeAdjust: "var(--destack-text-lg-font-size-adjust)",
    /** Extra large text. */
    xlFontFamily: "var(--destack-text-xl-font-family)",
    /** Extra large text. */
    xlFontSize: "var(--destack-text-xl-font-size)",
    /** Extra large text. */
    xlFontWeight: "var(--destack-text-xl-font-weight)",
    /** Extra large text. */
    xlLineHeight: "var(--destack-text-xl-line-height)",
    /** Extra large text. */
    xlLetterSpacing: "var(--destack-text-xl-letter-spacing)",
    /** Extra large text. */
    xlFontSizeAdjust: "var(--destack-text-xl-font-size-adjust)",
    /** Headings. */
    "2xlFontFamily": "var(--destack-text-2xl-font-family)",
    /** Headings. */
    "2xlFontSize": "var(--destack-text-2xl-font-size)",
    /** Headings. */
    "2xlFontWeight": "var(--destack-text-2xl-font-weight)",
    /** Headings. */
    "2xlLineHeight": "var(--destack-text-2xl-line-height)",
    /** Headings. */
    "2xlLetterSpacing": "var(--destack-text-2xl-letter-spacing)",
    /** Headings. */
    "2xlFontSizeAdjust": "var(--destack-text-2xl-font-size-adjust)",
    /** Headings, such as a view's title. */
    "3xlFontFamily": "var(--destack-text-3xl-font-family)",
    /** Headings, such as a view's title. */
    "3xlFontSize": "var(--destack-text-3xl-font-size)",
    /** Headings, such as a view's title. */
    "3xlFontWeight": "var(--destack-text-3xl-font-weight)",
    /** Headings, such as a view's title. */
    "3xlLineHeight": "var(--destack-text-3xl-line-height)",
    /** Headings, such as a view's title. */
    "3xlLetterSpacing": "var(--destack-text-3xl-letter-spacing)",
    /** Headings, such as a view's title. */
    "3xlFontSizeAdjust": "var(--destack-text-3xl-font-size-adjust)",
    /** Large headings. */
    "4xlFontFamily": "var(--destack-text-4xl-font-family)",
    /** Large headings. */
    "4xlFontSize": "var(--destack-text-4xl-font-size)",
    /** Large headings. */
    "4xlFontWeight": "var(--destack-text-4xl-font-weight)",
    /** Large headings. */
    "4xlLineHeight": "var(--destack-text-4xl-line-height)",
    /** Large headings. */
    "4xlLetterSpacing": "var(--destack-text-4xl-letter-spacing)",
    /** Large headings. */
    "4xlFontSizeAdjust": "var(--destack-text-4xl-font-size-adjust)",
    /** Display text. */
    "5xlFontFamily": "var(--destack-text-5xl-font-family)",
    /** Display text. */
    "5xlFontSize": "var(--destack-text-5xl-font-size)",
    /** Display text. */
    "5xlFontWeight": "var(--destack-text-5xl-font-weight)",
    /** Display text. */
    "5xlLineHeight": "var(--destack-text-5xl-line-height)",
    /** Display text. */
    "5xlLetterSpacing": "var(--destack-text-5xl-letter-spacing)",
    /** Display text. */
    "5xlFontSizeAdjust": "var(--destack-text-5xl-font-size-adjust)",
    /** Display text. */
    "6xlFontFamily": "var(--destack-text-6xl-font-family)",
    /** Display text. */
    "6xlFontSize": "var(--destack-text-6xl-font-size)",
    /** Display text. */
    "6xlFontWeight": "var(--destack-text-6xl-font-weight)",
    /** Display text. */
    "6xlLineHeight": "var(--destack-text-6xl-line-height)",
    /** Display text. */
    "6xlLetterSpacing": "var(--destack-text-6xl-letter-spacing)",
    /** Display text. */
    "6xlFontSizeAdjust": "var(--destack-text-6xl-font-size-adjust)",
    /** Display text. */
    "7xlFontFamily": "var(--destack-text-7xl-font-family)",
    /** Display text. */
    "7xlFontSize": "var(--destack-text-7xl-font-size)",
    /** Display text. */
    "7xlFontWeight": "var(--destack-text-7xl-font-weight)",
    /** Display text. */
    "7xlLineHeight": "var(--destack-text-7xl-line-height)",
    /** Display text. */
    "7xlLetterSpacing": "var(--destack-text-7xl-letter-spacing)",
    /** Display text. */
    "7xlFontSizeAdjust": "var(--destack-text-7xl-font-size-adjust)",
    /** Display text. */
    "8xlFontFamily": "var(--destack-text-8xl-font-family)",
    /** Display text. */
    "8xlFontSize": "var(--destack-text-8xl-font-size)",
    /** Display text. */
    "8xlFontWeight": "var(--destack-text-8xl-font-weight)",
    /** Display text. */
    "8xlLineHeight": "var(--destack-text-8xl-line-height)",
    /** Display text. */
    "8xlLetterSpacing": "var(--destack-text-8xl-letter-spacing)",
    /** Display text. */
    "8xlFontSizeAdjust": "var(--destack-text-8xl-font-size-adjust)",
    /** Display text, the largest. */
    "9xlFontFamily": "var(--destack-text-9xl-font-family)",
    /** Display text, the largest. */
    "9xlFontSize": "var(--destack-text-9xl-font-size)",
    /** Display text, the largest. */
    "9xlFontWeight": "var(--destack-text-9xl-font-weight)",
    /** Display text, the largest. */
    "9xlLineHeight": "var(--destack-text-9xl-line-height)",
    /** Display text, the largest. */
    "9xlLetterSpacing": "var(--destack-text-9xl-letter-spacing)",
    /** Display text, the largest. */
    "9xlFontSizeAdjust": "var(--destack-text-9xl-font-size-adjust)",
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
    /** A board column's width. */
    column: "var(--destack-width-column)",
    /** A dialog's width. */
    dialog: "var(--destack-width-dialog)",
    /** The width of a sheet at the side of the screen. */
    sheet: "var(--destack-width-sheet)",
    /** The widest a tooltip runs. */
    tooltip: "var(--destack-width-tooltip)",
    /** A command palette's width. */
    palette: "var(--destack-width-palette)",
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
