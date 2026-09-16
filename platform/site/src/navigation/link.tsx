import { A } from "@solidjs/router";
import type { StyleXStyles } from "@stylexjs/stylex";
import * as stylex from "@stylexjs/stylex";
import type { JSX } from "solid-js";

type SiteLinkProps = {
    /// The visible link content.
    children: JSX.Element;

    /// The link destination.
    href: string;

    /// The accessible name for an icon-only link.
    ariaLabel?: string;

    /// The optional compiled presentation.
    style?: StyleXStyles;

    /// The optional global keyboard shortcut.
    shortcut?: string;

    /// The optional native tooltip.
    title?: string;
};

/// Render internal navigation or a safely isolated external link.
export function SiteLink(props: SiteLinkProps) {
    // evaluate attributes in the rendered spread so route-driven styles stay reactive
    const attributes = () => ({
        "aria-label": props.ariaLabel,
        "data-shortcut": props.shortcut,
        href: props.href,
        title: props.title,
        ...stylex.attrs(props.style),
    });

    if (isExternalLink(props.href)) {
        return (
            <a {...attributes()} rel="external noopener noreferrer" target="_blank">
                {props.children}
            </a>
        );
    }

    return <A {...attributes()}>{props.children}</A>;
}

/// Return whether a destination leaves the current site.
export function isExternalLink(href: string) {
    return /^[a-z][a-z0-9+.-]*:/i.test(href);
}
