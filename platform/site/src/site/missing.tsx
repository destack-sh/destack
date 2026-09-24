import { color, fontFamily } from "@destack/theme/tokens.stylex";

import { httpStatus } from "@destack/view";
import * as stylex from "@destack/style";

import { lattice } from "../style/lattice.stylex";
import { tokens } from "../style/tokens.stylex";
import { Seo } from "./seo";
import { Shallows } from "./shallows";

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
            <Seo title="404" description={properties.description} />

            <section {...stylex.attrs(lattice.frame, lattice.ruleBottom, styles.page)}>
                <p {...stylex.attrs(lattice.ruleRight, styles.label)}>{properties.label}</p>
                <div {...stylex.attrs(styles.message)}>
                    <h1 {...stylex.attrs(styles.title)}>{properties.title}</h1>
                    <a {...stylex.attrs(styles.action)} href={properties.backHref}>
                        ← {properties.backLabel}
                    </a>
                </div>
                <Shallows style={styles.shallows} />
            </section>
        </>
    );
}

/** The media query for phone-width screens. */
const mobile = "@media (max-width: 767px)";

/** The missing page styles. */
const styles = stylex.create({
    page: {
        flexGrow: 1,
        gridTemplateRows: "1fr auto",
        minHeight: `calc(${tokens.column} * 3)`,
    },
    label: {
        color: color.mutedForeground,
        fontFamily: tokens.monoFont,
        fontSize: "0.75rem",
        gridColumn: "1 / span 3",
        letterSpacing: "0.12em",
        margin: 0,
        padding: tokens.inset,
        textTransform: "uppercase",
        [mobile]: { borderRightWidth: 0, gridColumn: "1 / -1" },
    },
    message: {
        alignContent: "end",
        display: "grid",
        gap: "1.25rem",
        gridColumn: "4 / span 9",
        padding: tokens.inset,
        [mobile]: { gridColumn: "1 / -1" },
    },
    shallows: {
        gridColumn: "1 / -1",
    },
    title: {
        fontFamily: fontFamily.default,
        fontSize: "clamp(2.25rem, 3.8vw, 3rem)",
        fontWeight: 500,
        letterSpacing: "-0.025em",
        lineHeight: 1.08,
        margin: 0,
    },
    action: {
        color: color.foreground,
        fontSize: "var(--size-navigation)",
        fontWeight: 600,
        width: "max-content",
        ":hover": { color: color.primary },
    },
});
