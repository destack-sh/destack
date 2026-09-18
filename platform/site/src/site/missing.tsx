import { color, fontFamily } from "@destack/theme/tokens.stylex";

import { httpStatus } from "@destack/view";
import * as stylex from "@destack/style";

import { tokens } from "../style/tokens.stylex";
import { Seo } from "./seo";

type MissingPageProps = {
    /// The destination offered after the missing route.
    backHref: string;

    /// The visible recovery link.
    backLabel: string;

    /// The metadata description.
    description: string;

    /// The compact missing-resource label.
    label: string;

    /// The visible page title.
    title: string;
};

/// Render a consistent not-found response inside the site shell.
export function MissingPage(props: MissingPageProps) {
    httpStatus(404);

    return (
        <>
            <Seo title="404" description={props.description} />

            <section {...stylex.attrs(styles.page)}>
                <p {...stylex.attrs(styles.label)}>{props.label}</p>
                <h1 {...stylex.attrs(styles.title)}>{props.title}</h1>
                <a {...stylex.attrs(styles.action)} href={props.backHref}>
                    ← {props.backLabel}
                </a>
            </section>
        </>
    );
}

const styles = stylex.create({
    action: {
        alignItems: "center",
        display: "inline-flex",
        fontFamily: fontFamily.default,
        fontWeight: 500,
        fontSize: "var(--size-navigation)",
        color: color.foreground,
        paddingBlock: "0.625rem",
        textTransform: "none",
        width: "max-content",
        ":hover": {
            color: color.primary,
        },
    },
    label: {
        color: color.foreground,
        fontFamily: fontFamily.default,
        fontSize: "var(--size-navigation)",
        margin: 0,
    },
    page: {
        alignContent: "center",
        display: "grid",
        gap: "1.25rem",
        marginInline: "auto",
        maxWidth: tokens.siteWidth,
        padding: `var(--content-section-gap) ${tokens.gutterRight} 3rem ${tokens.gutterLeft}`,
        width: "100%",
    },
    title: {
        fontFamily: fontFamily.default,
        fontSize: "var(--content-title-size)",
        fontWeight: 500,
        letterSpacing: "-0.04em",
        lineHeight: 1.25,
        margin: 0,
    },
});
