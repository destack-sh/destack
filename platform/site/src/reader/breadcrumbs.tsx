import { color, fontFamily } from "@destack/theme/tokens.stylex";

import { For } from "@destack/view";
import * as stylex from "@destack/style";

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
                        {item.href == undefined
                            ? (
                                <span {...stylex.attrs(styles.current)}>
                                    {item.label}
                                </span>
                            )
                            : (
                                <a {...stylex.attrs(styles.link)} href={item.href}>
                                    {item.label}
                                </a>
                            )}
                    </>
                )}
            </For>
        </nav>
    );
}

const styles = stylex.create({
    current: {
        color: color.foreground,
        fontWeight: 400,
    },
    link: {
        color: color.foreground,
        fontWeight: 400,
        ":hover": {
            color: color.primary,
        },
    },
    root: {
        alignItems: "baseline",
        display: "flex",
        flexWrap: "wrap",
        fontFamily: fontFamily.default,
        fontSize: "var(--size-label)",
        gap: "0.5rem",
        letterSpacing: "0.02em",
    },
});
