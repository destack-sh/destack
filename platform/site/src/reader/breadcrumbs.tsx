import { A } from "@solidjs/router";
import { For } from "solid-js";
import * as stylex from "@stylexjs/stylex";

import { tokens } from "../style/tokens.stylex";

export type Breadcrumb = {
    /// The breadcrumb destination when it is navigable.
    href?: string;

    /// The visible breadcrumb label.
    label: string;
};

type BreadcrumbsProps = {
    /// The ordered path from root to current page.
    items: readonly Breadcrumb[];
};

/// Render one compact navigable content path.
export function Breadcrumbs(props: BreadcrumbsProps) {
    return (
        <nav aria-label="Breadcrumb" {...stylex.attrs(styles.root)}>
            <For each={props.items}>
                {(item, index) => (
                    <>
                        {index() > 0 && <span>/</span>}
                        {item.href == undefined ? (
                            <span>{item.label}</span>
                        ) : (
                            <A {...stylex.attrs(styles.link)} href={item.href}>{item.label}</A>
                        )}
                    </>
                )}
            </For>
        </nav>
    );
}

const styles = stylex.create({
    link: {
        color: tokens.text,
        fontWeight: 600,
        ":hover": {
            color: tokens.accent,
        },
    },
    root: {
        alignItems: "baseline",
        display: "flex",
        flexWrap: "wrap",
        gap: "0.5rem",
    },
});
