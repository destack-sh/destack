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
                            <span {...stylex.attrs(styles.current)}>
                                {item.label}
                            </span>
                        ) : (
                            <A {...stylex.attrs(styles.link)} href={item.href}>
                                {item.label}
                            </A>
                        )}
                    </>
                )}
            </For>
        </nav>
    );
}

const styles = stylex.create({
    current: {
        color: tokens.ink,
        fontWeight: 400,
    },
    link: {
        color: tokens.text,
        fontWeight: 400,
        ":hover": {
            color: tokens.accent,
        },
    },
    root: {
        alignItems: "baseline",
        display: "flex",
        flexWrap: "wrap",
        fontFamily: tokens.textFont,
        fontSize: "var(--size-label)",
        gap: "0.5rem",
        letterSpacing: "0.02em",
    },
});
