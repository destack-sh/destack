import { Icon } from "@destack/icon";
import caretLeft from "@destack/icon/phosphor/caret-left";
import caretRight from "@destack/icon/phosphor/caret-right";
import { t } from "@destack/locale";
import * as style from "@destack/style";
import { space, stroke } from "@destack/theme/tokens.stylex";
import { useLocale } from "@destack/locale/solid";
import type { JSX } from "@solidjs/web";
import { omit } from "solid-js";
import { buttonStyle, type ButtonSize } from "../button/index.ts";

/** The styles of a pagination and its elements. */
const styles = style.create({
    pagination: {
        display: "flex",
        justifyContent: "center",
        width: "100%",
    },
    content: {
        display: "flex",
        alignItems: "center",
        gap: space[1],
        margin: 0,
        padding: 0,
        listStyle: "none",
    },
    step: {
        paddingInline: space[3],
    },
    ellipsis: {
        display: "inline-flex",
        alignItems: "center",
        justifyContent: "center",
        width: space[6],
        height: space[6],
    },
    hidden: {
        position: "absolute",
        width: stroke.border,
        height: stroke.border,
        overflow: "hidden",
        clipPath: "inset(50%)",
        whiteSpace: "nowrap",
    },
});

/** The properties of an element of a pagination, the native element's attributes included. */
export type PaginationElementProperties<Attributes> = Omit<Attributes, "class" | "style"> & {
    /** The StyleX styles applied after the element's styles. */
    readonly style?: style.Styles;
};

/** The properties of a link to a page. */
export type PaginationLinkProperties = PaginationElementProperties<
    JSX.AnchorHTMLAttributes<HTMLAnchorElement>
> & {
    /** Whether the link points at the current page. */
    readonly isActive?: boolean;
    /** The height and padding of the link, a square icon size by default. */
    readonly size?: ButtonSize;
};

/** Render the navigation landmark of a list of pages. */
export function Pagination(
    properties: PaginationElementProperties<JSX.HTMLAttributes<HTMLElement>>,
): JSX.Element {
    const locale = useLocale();
    const rest = omit(properties, "style");

    return (
        <nav
            aria-label={locale.render(t`Pagination`)}
            data-slot="pagination"
            {...rest}
            {...style.attrs(styles.pagination, properties.style)}
        />
    );
}

/** Render the list of a pagination's links. */
export function PaginationContent(
    properties: PaginationElementProperties<JSX.HTMLAttributes<HTMLUListElement>>,
): JSX.Element {
    const rest = omit(properties, "style");

    return (
        <ul
            data-slot="pagination-content"
            {...rest}
            {...style.attrs(styles.content, properties.style)}
        />
    );
}

/** Render one entry of a pagination's list. */
export function PaginationItem(
    properties: PaginationElementProperties<JSX.LiHTMLAttributes<HTMLLIElement>>,
): JSX.Element {
    const rest = omit(properties, "style");

    return <li data-slot="pagination-item" {...rest} {...style.attrs(properties.style)} />;
}

/** Render a link to a page, marked as the current page when active. */
export function PaginationLink(properties: PaginationLinkProperties): JSX.Element {
    const rest = omit(properties, "isActive", "size", "style");

    return (
        <a
            aria-current={properties.isActive === true ? "page" : undefined}
            data-slot="pagination-link"
            data-active={properties.isActive === true ? "true" : undefined}
            {...rest}
            {...style.attrs(
                buttonStyle({
                    variant: properties.isActive === true ? "outline" : "ghost",
                    size: properties.size ?? "icon",
                }),
                properties.style,
            )}
        />
    );
}

/** Render the link to the previous page, its arrow pointing against the text. */
export function PaginationPrevious(
    properties: Omit<PaginationLinkProperties, "children">,
): JSX.Element {
    const locale = useLocale();
    const rest = omit(properties, "style");

    return (
        <PaginationLink
            aria-label={locale.render(t`Go to the previous page`)}
            data-slot="pagination-previous"
            size="default"
            {...rest}
            style={[styles.step, properties.style]}
        >
            <Icon icon={locale.direction === "rtl" ? caretRight : caretLeft} />
            <span>{locale.render(t`Previous`)}</span>
        </PaginationLink>
    );
}

/** Render the link to the next page, its arrow pointing along the text. */
export function PaginationNext(
    properties: Omit<PaginationLinkProperties, "children">,
): JSX.Element {
    const locale = useLocale();
    const rest = omit(properties, "style");

    return (
        <PaginationLink
            aria-label={locale.render(t`Go to the next page`)}
            data-slot="pagination-next"
            size="default"
            {...rest}
            style={[styles.step, properties.style]}
        >
            <span>{locale.render(t`Next`)}</span>
            <Icon icon={locale.direction === "rtl" ? caretLeft : caretRight} />
        </PaginationLink>
    );
}

/** Render the mark for pages left out of a long list. */
export function PaginationEllipsis(
    properties: PaginationElementProperties<JSX.HTMLAttributes<HTMLSpanElement>>,
): JSX.Element {
    const locale = useLocale();
    const rest = omit(properties, "style");

    return (
        <span
            data-slot="pagination-ellipsis"
            {...rest}
            {...style.attrs(styles.ellipsis, properties.style)}
        >
            <Icon name="dots-three" />
            <span {...style.attrs(styles.hidden)}>{locale.render(t`More pages`)}</span>
        </span>
    );
}
