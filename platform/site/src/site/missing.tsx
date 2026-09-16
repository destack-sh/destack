import { A } from "@solidjs/router";
import { HttpStatusCode } from "@solidjs/start";
import * as stylex from "@stylexjs/stylex";

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
    return (
        <>
            <HttpStatusCode code={404} />
            <Seo title="404" description={props.description} />

            <section {...stylex.attrs(styles.page)}>
                <p {...stylex.attrs(styles.label)}>{props.label}</p>
                <h1 {...stylex.attrs(styles.title)}>{props.title}</h1>
                <A {...stylex.attrs(styles.action)} href={props.backHref}>
                    ← {props.backLabel}
                </A>
            </section>
        </>
    );
}

const styles = stylex.create({
    action: {
        alignItems: "center",
        display: "inline-flex",
        fontFamily: tokens.textFont,
        fontWeight: 500,
        fontSize: "var(--size-navigation)",
        color: tokens.ink,
        paddingBlock: "0.625rem",
        textTransform: "none",
        width: "max-content",
        ":hover": {
            color: tokens.accent,
        },
    },
    label: {
        color: tokens.ink,
        fontFamily: tokens.textFont,
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
        fontFamily: tokens.textFont,
        fontSize: "var(--content-title-size)",
        fontWeight: 500,
        letterSpacing: "-0.04em",
        lineHeight: 1.25,
        margin: 0,
    },
});
