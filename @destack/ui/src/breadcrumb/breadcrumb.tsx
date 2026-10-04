import { Icon } from "@destack/icon";
import caretLeft from "@destack/icon/phosphor/caret-left";
import caretRight from "@destack/icon/phosphor/caret-right";
import { t } from "@destack/locale";
import * as style from "@destack/style";
import { color, motion, space, stroke } from "@destack/theme/tokens.stylex";
import { useLocale } from "@destack/locale/solid";
import { text } from "@destack/theme/text";
import type { JSX } from "@solidjs/web";
import { omit, Show } from "solid-js";
import { directionOf } from "../focus/index.ts";

/** The styles of a breadcrumb and its elements. */
const styles = style.create({
    list: {
        display: "flex",
        flexWrap: "wrap",
        alignItems: "center",
        gap: space[2],
        margin: 0,
        padding: 0,
        listStyle: "none",
        color: color.mutedForeground,
        wordBreak: "break-word",
    },
    item: {
        display: "inline-flex",
        alignItems: "center",
        gap: space[1],
    },
    link: {
        color: { default: "inherit", ":hover": color.foreground },
        textDecoration: "none",
        transitionProperty: "color",
        transitionDuration: motion.durationShort,
        outlineStyle: { default: "none", ":focus-visible": "solid" },
        outlineWidth: stroke.ring,
        outlineColor: `color-mix(in oklab, ${color.ring} 50%, transparent)`,
    },
    page: {
        color: color.foreground,
    },
    ellipsis: {
        display: "inline-flex",
        alignItems: "center",
        justifyContent: "center",
        width: space[5],
        height: space[5],
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

/** The properties of an element of a breadcrumb, the native element's attributes included. */
export type BreadcrumbElementProperties<Attributes> = Omit<Attributes, "class" | "style"> & {
    /** The StyleX styles applied after the element's styles. */
    readonly style?: style.Styles;
};

/** Render the navigation landmark of the trail of pages above the current one. */
export function Breadcrumb(
    properties: BreadcrumbElementProperties<JSX.HTMLAttributes<HTMLElement>>,
): JSX.Element {
    const locale = useLocale();
    const rest = omit(properties, "style");

    return (
        <nav
            aria-label={locale.render(t`Breadcrumb`)}
            data-slot="breadcrumb"
            {...rest}
            {...style.attrs(properties.style)}
        />
    );
}

/** Render the ordered list of the trail. */
export function BreadcrumbList(
    properties: BreadcrumbElementProperties<JSX.OlHTMLAttributes<HTMLOListElement>>,
): JSX.Element {
    const rest = omit(properties, "style");

    return (
        <ol
            data-slot="breadcrumb-list"
            {...rest}
            {...style.attrs(text.footnote, styles.list, properties.style)}
        />
    );
}

/** Render one step of the trail. */
export function BreadcrumbItem(
    properties: BreadcrumbElementProperties<JSX.LiHTMLAttributes<HTMLLIElement>>,
): JSX.Element {
    const rest = omit(properties, "style");

    return (
        <li data-slot="breadcrumb-item" {...rest} {...style.attrs(styles.item, properties.style)} />
    );
}

/** Render the link to a page of the trail. */
export function BreadcrumbLink(
    properties: BreadcrumbElementProperties<JSX.AnchorHTMLAttributes<HTMLAnchorElement>>,
): JSX.Element {
    const rest = omit(properties, "style");

    return (
        <a data-slot="breadcrumb-link" {...rest} {...style.attrs(styles.link, properties.style)} />
    );
}

/** Render the current page at the end of the trail. */
export function BreadcrumbPage(
    properties: BreadcrumbElementProperties<JSX.HTMLAttributes<HTMLSpanElement>>,
): JSX.Element {
    const rest = omit(properties, "style");

    return (
        <span
            aria-current="page"
            data-slot="breadcrumb-page"
            {...rest}
            {...style.attrs(styles.page, properties.style)}
        />
    );
}

/** Render the mark between two steps, a chevron pointing along the text by default. */
export function BreadcrumbSeparator(
    properties: BreadcrumbElementProperties<JSX.LiHTMLAttributes<HTMLLIElement>>,
): JSX.Element {
    const locale = useLocale();
    const rest = omit(properties, "style", "children");

    return (
        <li
            role="presentation"
            aria-hidden="true"
            data-slot="breadcrumb-separator"
            {...rest}
            {...style.attrs(styles.item, properties.style)}
        >
            <Show
                when={"children" in properties}
                fallback={
                    <Icon icon={directionOf(locale.tag) === "rtl" ? caretLeft : caretRight} />
                }
            >
                {properties.children}
            </Show>
        </li>
    );
}

/** Render the mark for steps left out of a long trail. */
export function BreadcrumbEllipsis(
    properties: BreadcrumbElementProperties<JSX.HTMLAttributes<HTMLSpanElement>>,
): JSX.Element {
    const locale = useLocale();
    const rest = omit(properties, "style");

    return (
        <span
            data-slot="breadcrumb-ellipsis"
            {...rest}
            {...style.attrs(styles.ellipsis, properties.style)}
        >
            <Icon name="dots-three" />
            <span {...style.attrs(styles.hidden)}>{locale.render(t`More`)}</span>
        </span>
    );
}
