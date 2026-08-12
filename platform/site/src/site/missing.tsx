import { A } from "@solidjs/router";
import { HttpStatusCode } from "@solidjs/start";
import * as stylex from "@stylexjs/stylex";

import { Seo } from "./seo";
import { styles } from "./missing.stylex";

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
