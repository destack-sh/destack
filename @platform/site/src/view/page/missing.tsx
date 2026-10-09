import { frame } from "../layout/frame.stylex";
import { media } from "@destack/style/media.stylex";
import { text } from "@destack/theme/text";
import { color, font, stroke, weight } from "@destack/theme/tokens.stylex";

import { httpStatus } from "@destack/view";
import * as style from "@destack/style";
import { Metadata } from "@destack/view/document";

import { lattice } from "../layout/lattice.stylex";
import { Shallows } from "../effect/shallows";

/** Properties for a missing page notice. */
type MissingPageProperties = {
    /** The destination offered after the missing route. */
    backHref: string;

    /** The visible recovery link. */
    backLabel: string;

    /** The metadata description. */
    description: string;

    /** The compact missing-resource label. */
    label: string;

    /** The visible page title. */
    title: string;
};

/** Render a consistent not-found response inside the site shell. */
export function MissingPage(properties: MissingPageProperties) {
    httpStatus(404);

    return (
        <>
            <Metadata title="404" description={properties.description} robots={{ index: false }} />

            <section {...style.attrs(lattice.frame, lattice.ruleBottom, styles.page)}>
                <p {...style.attrs(lattice.ruleRight, styles.label)}>{properties.label}</p>
                <div {...style.attrs(styles.message)}>
                    <h1 {...style.attrs(styles.title)}>{properties.title}</h1>
                    <a {...style.attrs(text.subheadline, styles.action)} href={properties.backHref}>
                        ← {properties.backLabel}
                    </a>
                </div>
                <Shallows xstyle={styles.shallows} />
            </section>
        </>
    );
}

/** Render the page for a route the site has none for. */
export function Missing() {
    return (
        <MissingPage
            backHref="/"
            backLabel="back to destack"
            description="This page does not exist."
            label="404"
            title="this page does not exist"
        />
    );
}

/** The missing page styles. */
const styles = style.create({
    page: {
        flexGrow: 1,
        gridTemplateRows: "1fr auto",
        minHeight: `calc(${frame.column} * 3)`,
    },
    label: {
        color: color.mutedForeground,
        fontFamily: font.code,
        fontSize: "0.75rem",
        gridColumn: { default: "1 / span 3", [media.maxMd]: "1 / -1" },
        letterSpacing: "0.12em",
        margin: 0,
        padding: frame.inset,
        textTransform: "uppercase",
        borderRightWidth: { default: stroke.border, [media.maxMd]: 0 },
    },
    message: {
        alignContent: "end",
        display: "grid",
        gap: "1.25rem",
        gridColumn: { default: "4 / span 9", [media.maxMd]: "1 / -1" },
        padding: frame.inset,
    },
    shallows: {
        gridColumn: "1 / -1",
    },
    title: {
        fontFamily: font.text,
        fontSize: "clamp(2.25rem, 3.8vw, 3rem)",
        fontWeight: weight.medium,
        letterSpacing: "-0.025em",
        lineHeight: 1.08,
        margin: 0,
    },
    action: {
        color: {
            default: color.foreground,
            ":hover": { default: null, [media.hover]: color.primary },
        },
        fontWeight: weight.semibold,
        width: "max-content",
    },
});
