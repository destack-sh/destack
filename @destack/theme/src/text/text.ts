// generate with `bun run generate` from tokens.json

import * as style from "@destack/style";

/** Text styles after Tailwind's type scale, scaled by the person's text size. */
export const text = style.create({
    /** Extra small text, such as badges, shortcuts and labels. */
    xs: {
        fontFamily: "var(--destack-text-xs-font-family)",
        fontSize: "var(--destack-text-xs-font-size)",
        fontWeight: "var(--destack-text-xs-font-weight)",
        lineHeight: "var(--destack-text-xs-line-height)",
        letterSpacing: "var(--destack-text-xs-letter-spacing)",
    },
    /** Small text, the default in controls and lists. */
    sm: {
        fontFamily: "var(--destack-text-sm-font-family)",
        fontSize: "var(--destack-text-sm-font-size)",
        fontWeight: "var(--destack-text-sm-font-weight)",
        lineHeight: "var(--destack-text-sm-line-height)",
        letterSpacing: "var(--destack-text-sm-letter-spacing)",
    },
    /** Base text, for reading. */
    base: {
        fontFamily: "var(--destack-text-base-font-family)",
        fontSize: "var(--destack-text-base-font-size)",
        fontWeight: "var(--destack-text-base-font-weight)",
        lineHeight: "var(--destack-text-base-line-height)",
        letterSpacing: "var(--destack-text-base-letter-spacing)",
    },
    /** Large text, such as a dialog's title. */
    lg: {
        fontFamily: "var(--destack-text-lg-font-family)",
        fontSize: "var(--destack-text-lg-font-size)",
        fontWeight: "var(--destack-text-lg-font-weight)",
        lineHeight: "var(--destack-text-lg-line-height)",
        letterSpacing: "var(--destack-text-lg-letter-spacing)",
    },
    /** Extra large text. */
    xl: {
        fontFamily: "var(--destack-text-xl-font-family)",
        fontSize: "var(--destack-text-xl-font-size)",
        fontWeight: "var(--destack-text-xl-font-weight)",
        lineHeight: "var(--destack-text-xl-line-height)",
        letterSpacing: "var(--destack-text-xl-letter-spacing)",
    },
    /** Headings. */
    "2xl": {
        fontFamily: "var(--destack-text-2xl-font-family)",
        fontSize: "var(--destack-text-2xl-font-size)",
        fontWeight: "var(--destack-text-2xl-font-weight)",
        lineHeight: "var(--destack-text-2xl-line-height)",
        letterSpacing: "var(--destack-text-2xl-letter-spacing)",
    },
    /** Headings, such as a view's title. */
    "3xl": {
        fontFamily: "var(--destack-text-3xl-font-family)",
        fontSize: "var(--destack-text-3xl-font-size)",
        fontWeight: "var(--destack-text-3xl-font-weight)",
        lineHeight: "var(--destack-text-3xl-line-height)",
        letterSpacing: "var(--destack-text-3xl-letter-spacing)",
    },
    /** Large headings. */
    "4xl": {
        fontFamily: "var(--destack-text-4xl-font-family)",
        fontSize: "var(--destack-text-4xl-font-size)",
        fontWeight: "var(--destack-text-4xl-font-weight)",
        lineHeight: "var(--destack-text-4xl-line-height)",
        letterSpacing: "var(--destack-text-4xl-letter-spacing)",
    },
    /** Display text. */
    "5xl": {
        fontFamily: "var(--destack-text-5xl-font-family)",
        fontSize: "var(--destack-text-5xl-font-size)",
        fontWeight: "var(--destack-text-5xl-font-weight)",
        lineHeight: "var(--destack-text-5xl-line-height)",
        letterSpacing: "var(--destack-text-5xl-letter-spacing)",
    },
    /** Display text. */
    "6xl": {
        fontFamily: "var(--destack-text-6xl-font-family)",
        fontSize: "var(--destack-text-6xl-font-size)",
        fontWeight: "var(--destack-text-6xl-font-weight)",
        lineHeight: "var(--destack-text-6xl-line-height)",
        letterSpacing: "var(--destack-text-6xl-letter-spacing)",
    },
    /** Display text. */
    "7xl": {
        fontFamily: "var(--destack-text-7xl-font-family)",
        fontSize: "var(--destack-text-7xl-font-size)",
        fontWeight: "var(--destack-text-7xl-font-weight)",
        lineHeight: "var(--destack-text-7xl-line-height)",
        letterSpacing: "var(--destack-text-7xl-letter-spacing)",
    },
    /** Display text. */
    "8xl": {
        fontFamily: "var(--destack-text-8xl-font-family)",
        fontSize: "var(--destack-text-8xl-font-size)",
        fontWeight: "var(--destack-text-8xl-font-weight)",
        lineHeight: "var(--destack-text-8xl-line-height)",
        letterSpacing: "var(--destack-text-8xl-letter-spacing)",
    },
    /** Display text, the largest. */
    "9xl": {
        fontFamily: "var(--destack-text-9xl-font-family)",
        fontSize: "var(--destack-text-9xl-font-size)",
        fontWeight: "var(--destack-text-9xl-font-weight)",
        lineHeight: "var(--destack-text-9xl-line-height)",
        letterSpacing: "var(--destack-text-9xl-letter-spacing)",
    },
});
