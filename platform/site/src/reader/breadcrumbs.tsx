import { A } from "@solidjs/router";
import { For } from "solid-js";

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
        <nav aria-label="Breadcrumb" class="breadcrumbs">
            <For each={props.items}>
                {(item, index) => (
                    <>
                        {index() > 0 && <span>/</span>}
                        {item.href == undefined ? (
                            <span>{item.label}</span>
                        ) : (
                            <A href={item.href}>[{item.label}]</A>
                        )}
                    </>
                )}
            </For>
        </nav>
    );
}
