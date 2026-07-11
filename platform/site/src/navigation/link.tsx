import { A } from "@solidjs/router";
import type { JSX } from "solid-js";

type SiteLinkProps = {
    /// The visible link content.
    children: JSX.Element;

    /// The optional CSS class.
    class?: string;

    /// The link destination.
    href: string;

    /// The optional global keyboard shortcut.
    shortcut?: string;

    /// The optional native tooltip.
    title?: string;
};

/// Render internal navigation or a safely isolated external link.
export function SiteLink(props: SiteLinkProps) {
    const attributes = {
        class: props.class,
        "data-shortcut": props.shortcut,
        href: props.href,
        title: props.title,
    };

    if (isExternalLink(props.href)) {
        return (
            <a {...attributes} rel="external noopener noreferrer" target="_blank">
                {props.children}
            </a>
        );
    }

    return <A {...attributes}>{props.children}</A>;
}

/// Return whether a destination leaves the current site.
export function isExternalLink(href: string) {
    return /^[a-z][a-z0-9+.-]*:/i.test(href);
}
