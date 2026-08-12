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
                <A {...stylex.attrs(styles.action)} href={props.backHref}>← {props.backLabel}</A>
            </section>
        </>
    );
}

const styles = stylex.create({
    action: {
        alignItems: "center",
        backgroundColor: tokens.orange,
        borderColor: tokens.ink,
        borderRadius: tokens.panelRadius,
        borderStyle: "solid",
        borderWidth: tokens.stroke,
        display: "inline-flex",
        fontFamily: tokens.monoFont,
        fontWeight: 700,
        padding: "0.45rem 1.15rem",
        width: "max-content",
        ":hover": {
            color: tokens.cream,
        },
    },
    label: {
        color: tokens.soft,
        fontFamily: tokens.monoFont,
        fontSize: "0.78rem",
        margin: 0,
    },
    page: {
        alignContent: "center",
        display: "grid",
        gap: "1.25rem",
        marginInline: "auto",
        maxWidth: "48rem",
        padding: `4rem ${tokens.gutterRight} 5rem ${tokens.gutterLeft}`,
        width: "100%",
    },
    title: {
        fontFamily: tokens.monoFont,
        fontSize: "clamp(2.2rem, 5vw, 3.25rem)",
        fontWeight: 700,
        letterSpacing: "-0.04em",
        lineHeight: 1,
        margin: 0,
    },
});
