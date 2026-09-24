import type { Styles } from "@destack/style";
import * as stylex from "@destack/style";
import type { JSX } from "@destack/view";

/** Properties for a site link. */
type SiteLinkProperties = {
    /** The visible link content. */
    children: JSX.Element;

    /** The link destination. */
    href: string;

    /** The accessible name for an icon-only link. */
    ariaLabel?: string;

    /** The optional compiled presentation. */
    style?: Styles;

    /** The optional global keyboard shortcut. */
    shortcut?: string;

    /** The optional native tooltip. */
    title?: string;
};

/** Render internal navigation or a safely isolated external link. */
export function SiteLink(properties: SiteLinkProperties) {
    // evaluate attributes in the rendered spread so route-driven styles stay reactive
    const attributes = () => ({
        "aria-label": properties.ariaLabel,
        "data-shortcut": properties.shortcut,
        href: properties.href,
        title: properties.title,
        ...stylex.attrs(properties.style),
    });

    if (isExternalLink(properties.href)) {
        return (
            <a {...attributes()} rel="external noopener noreferrer" target="_blank">
                {properties.children}
            </a>
        );
    }

    return <a {...attributes()}>{properties.children}</a>;
}

/** Return whether a destination leaves the current site. */
export function isExternalLink(href: string) {
    return /^[a-z][a-z0-9+.-]*:/i.test(href);
}
