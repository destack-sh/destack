// generate with `bun run generate` from tokens.json

import * as style from "@destack/style";

/** Text styles at the medium text size with 16px body text, scaled by the person's text size. */
export const text = style.create({
    /** Captions and labels. */
    caption: {
        fontFamily: "var(--destack-text-caption-font-family)",
        fontSize: "var(--destack-text-caption-font-size)",
        fontWeight: "var(--destack-text-caption-font-weight)",
        lineHeight: "var(--destack-text-caption-line-height)",
        letterSpacing: "var(--destack-text-caption-letter-spacing)",
    },
    /** Footnotes and secondary labels. */
    footnote: {
        fontFamily: "var(--destack-text-footnote-font-family)",
        fontSize: "var(--destack-text-footnote-font-size)",
        fontWeight: "var(--destack-text-footnote-font-weight)",
        lineHeight: "var(--destack-text-footnote-line-height)",
        letterSpacing: "var(--destack-text-footnote-letter-spacing)",
    },
    /** Body text. */
    body: {
        fontFamily: "var(--destack-text-body-font-family)",
        fontSize: "var(--destack-text-body-font-size)",
        fontWeight: "var(--destack-text-body-font-weight)",
        lineHeight: "var(--destack-text-body-line-height)",
        letterSpacing: "var(--destack-text-body-letter-spacing)",
    },
    /** Callouts beside body text. */
    callout: {
        fontFamily: "var(--destack-text-callout-font-family)",
        fontSize: "var(--destack-text-callout-font-size)",
        fontWeight: "var(--destack-text-callout-font-weight)",
        lineHeight: "var(--destack-text-callout-line-height)",
        letterSpacing: "var(--destack-text-callout-letter-spacing)",
    },
    /** Subheadings and secondary text beside body text. */
    subheadline: {
        fontFamily: "var(--destack-text-subheadline-font-family)",
        fontSize: "var(--destack-text-subheadline-font-size)",
        fontWeight: "var(--destack-text-subheadline-font-weight)",
        lineHeight: "var(--destack-text-subheadline-line-height)",
        letterSpacing: "var(--destack-text-subheadline-letter-spacing)",
    },
    /** Headlines within body text. */
    headline: {
        fontFamily: "var(--destack-text-headline-font-family)",
        fontSize: "var(--destack-text-headline-font-size)",
        fontWeight: "var(--destack-text-headline-font-weight)",
        lineHeight: "var(--destack-text-headline-line-height)",
        letterSpacing: "var(--destack-text-headline-letter-spacing)",
    },
    /** First-level titles. */
    title1: {
        fontFamily: "var(--destack-text-title1-font-family)",
        fontSize: "var(--destack-text-title1-font-size)",
        fontWeight: "var(--destack-text-title1-font-weight)",
        lineHeight: "var(--destack-text-title1-line-height)",
        letterSpacing: "var(--destack-text-title1-letter-spacing)",
    },
    /** Second-level titles. */
    title2: {
        fontFamily: "var(--destack-text-title2-font-family)",
        fontSize: "var(--destack-text-title2-font-size)",
        fontWeight: "var(--destack-text-title2-font-weight)",
        lineHeight: "var(--destack-text-title2-line-height)",
        letterSpacing: "var(--destack-text-title2-letter-spacing)",
    },
    /** Third-level titles. */
    title3: {
        fontFamily: "var(--destack-text-title3-font-family)",
        fontSize: "var(--destack-text-title3-font-size)",
        fontWeight: "var(--destack-text-title3-font-weight)",
        lineHeight: "var(--destack-text-title3-line-height)",
        letterSpacing: "var(--destack-text-title3-letter-spacing)",
    },
    /** Large titles at the top of a view. */
    largeTitle: {
        fontFamily: "var(--destack-text-large-title-font-family)",
        fontSize: "var(--destack-text-large-title-font-size)",
        fontWeight: "var(--destack-text-large-title-font-weight)",
        lineHeight: "var(--destack-text-large-title-line-height)",
        letterSpacing: "var(--destack-text-large-title-letter-spacing)",
    },
});
