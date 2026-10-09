import { Icon } from "@destack/icon";
import caretLeft from "@destack/icon/phosphor/caret-left";
import caretRight from "@destack/icon/phosphor/caret-right";
import { t } from "@destack/locale";
import * as style from "@destack/style";
import { media } from "@destack/style/media.stylex";
import { color, motion, space, stroke } from "@destack/theme/tokens.stylex";
import { text } from "@destack/theme/text";
import { type JSX, merge, omit, Show, useLocale } from "@destack/view";
import { type PartAttributes, type Render, rendered, renderPart } from "../part/index.ts";
import { visuallyHiddenStyle } from "../visually-hidden/index.ts";

/** The styles of a breadcrumb and its elements. */
const styles = style.create({
    list: {
        display: "flex",
        flexWrap: "wrap",
        alignItems: "center",
        gap: space[2],
        color: color.mutedForeground,
        wordBreak: "break-word",
    },
    item: {
        display: "inline-flex",
        alignItems: "center",
        gap: space[1],
    },
    link: {
        color: { default: "inherit", ":hover": { default: null, [media.hover]: color.foreground } },
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
});

/** The properties of an element of a breadcrumb, the native element's attributes included. */
export type BreadcrumbElementProperties<Attributes> = Omit<Attributes, "class"> & {
    /** The StyleX styles applied after the element's styles. */
    readonly xstyle?: style.Styles;
};

/** Render the navigation landmark of the trail of pages above the current one. */
export function Breadcrumb(
    properties: BreadcrumbElementProperties<JSX.HTMLAttributes<HTMLElement>>,
): JSX.Element {
    const locale = useLocale();

    return renderPart("nav", "breadcrumb", properties, null, {
        get "aria-label"() {
            return locale.render(t`Breadcrumb`);
        },
    });
}

/** Render the ordered list of the trail. */
export function BreadcrumbList(
    properties: BreadcrumbElementProperties<JSX.OlHTMLAttributes<HTMLOListElement>>,
): JSX.Element {
    return renderPart("ol", "breadcrumb-list", properties, [text.footnote, styles.list]);
}

/** Render one step of the trail. */
export function BreadcrumbItem(
    properties: BreadcrumbElementProperties<JSX.LiHTMLAttributes<HTMLLIElement>>,
): JSX.Element {
    return renderPart("li", "breadcrumb-item", properties, styles.item);
}

/** The properties of the link to a page of a trail, the native anchor's attributes included. */
export type BreadcrumbLinkProperties = BreadcrumbElementProperties<
    JSX.AnchorHTMLAttributes<HTMLAnchorElement>
> & {
    /** Render another element with the link's attributes, the native anchor by default. */
    readonly render?: Render;
};

/** Render the link to a page of the trail. */
export function BreadcrumbLink(properties: BreadcrumbLinkProperties): JSX.Element {
    const rest = omit(properties, "xstyle", "style", "render");
    const part: PartAttributes = merge({ "data-slot": "breadcrumb-link" }, () =>
        style.attributes([styles.link, properties.xstyle], properties.style),
    );

    return rendered(properties.render, part, rest, () => <a {...part} {...rest} />);
}

/** Render the current page at the end of the trail. */
export function BreadcrumbPage(
    properties: BreadcrumbElementProperties<JSX.HTMLAttributes<HTMLSpanElement>>,
): JSX.Element {
    return renderPart("span", "breadcrumb-page", properties, styles.page, {
        "aria-current": "page",
    });
}

/** Render the mark between two steps, a chevron pointing along the text by default. */
export function BreadcrumbSeparator(
    properties: BreadcrumbElementProperties<JSX.LiHTMLAttributes<HTMLLIElement>>,
): JSX.Element {
    const locale = useLocale();
    const rest = omit(properties, "xstyle", "style", "children");

    return (
        <li
            role="presentation"
            aria-hidden="true"
            data-slot="breadcrumb-separator"
            {...rest}
            {...style.attributes([styles.item, properties.xstyle], properties.style)}
        >
            <Show
                when={"children" in properties}
                fallback={<Icon icon={locale.direction === "rtl" ? caretLeft : caretRight} />}
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

    return renderPart("span", "breadcrumb-ellipsis", properties, styles.ellipsis, {
        get children() {
            return (
                <>
                    <Icon name="dots-three" />
                    <span {...style.attrs(visuallyHiddenStyle())}>{locale.render(t`More`)}</span>
                </>
            );
        },
    });
}
