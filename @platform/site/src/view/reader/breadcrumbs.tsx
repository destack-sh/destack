import {
    Breadcrumb,
    BreadcrumbItem,
    BreadcrumbLink,
    BreadcrumbList,
    BreadcrumbSeparator,
} from "@destack/ui/breadcrumb";
import { text } from "@destack/theme/text";
import { For } from "@destack/view";

/** One step of a content path. */
export type Step = {
    /** The step's destination. */
    href: string;

    /** The visible step label. */
    label: string;
};

/** Properties for a content path. */
type BreadcrumbsProperties = {
    /** The ordered path from the root to the page above the current one. */
    items: readonly Step[];
};

/** Render one compact navigable content path. */
export function Breadcrumbs(properties: BreadcrumbsProperties) {
    return (
        <Breadcrumb>
            <BreadcrumbList xstyle={text.subheadline}>
                <For each={properties.items}>
                    {(item, index) => (
                        <>
                            {/* separate each step from the one before it */}
                            {index() > 0 && <BreadcrumbSeparator>/</BreadcrumbSeparator>}
                            <BreadcrumbItem>
                                <BreadcrumbLink href={item.href}>{item.label}</BreadcrumbLink>
                            </BreadcrumbItem>
                        </>
                    )}
                </For>
            </BreadcrumbList>
        </Breadcrumb>
    );
}
