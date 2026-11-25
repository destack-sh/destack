import { state } from "@destack/theme/state";
import { Icon } from "@destack/ui/icon";
import caretLeft from "@destack/icon/phosphor/caret-left";
import caretRight from "@destack/icon/phosphor/caret-right";
import { t } from "@destack/locale";
import * as style from "@destack/style";
import { media } from "@destack/style/media.stylex";
import { color, motion, size, space } from "@destack/theme/tokens.stylex";
import { text } from "@destack/theme/text";
import { type JSX, omit, useContext, useLocale } from "@destack/view";
import { OverflowContext, OverflowEntryContext, useOverflowEntry } from "../overflow/index.ts";
import { contentOr, type Render, renderPart } from "../part/index.ts";
import { visuallyHiddenStyle } from "../visually-hidden/index.ts";

/** The styles of a breadcrumb and its elements. */
const styles = style.create({
    list: {
        display: "flex",
        flexWrap: "wrap",
        alignItems: "center",
        gap: space[2],
        color: color.mutedForeground,
        overflowWrap: "break-word",
    },
    // stand on one line where what does not fit hides
    line: {
        flexWrap: "nowrap",
        flexShrink: 0,
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
        transitionTimingFunction: motion.easingStandard,
    },
    page: {
        color: color.foreground,
    },
    ellipsis: {
        display: "inline-flex",
        alignItems: "center",
        justifyContent: "center",
        width: size[1],
        height: size[1],
    },
});

/** The properties of an element of a breadcrumb, the native element's attributes included. */
export type BreadcrumbElementProperties<Attributes> = Omit<Attributes, "class"> & {
    /** The StyleX styles applied after the element's styles. */
    readonly xstyle?: style.Styles;
    /** Render another element with its attributes, its own by default. */
    readonly render?: Render;
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
    // stand on one line in an overflow
    const isLine = useContext(OverflowContext) !== null;

    return renderPart("ol", "breadcrumb-list", properties, [
        text.sm,
        styles.list,
        isLine && styles.line,
    ]);
}

/** Render one step of the trail. */
export function BreadcrumbItem(
    properties: BreadcrumbElementProperties<JSX.LiHTMLAttributes<HTMLLIElement>>,
): JSX.Element {
    // hide in an overflow where it does not fit
    let element: HTMLElement | undefined;
    const entry = useOverflowEntry({ kind: "item", element: () => element });
    const around = useContext(OverflowEntryContext);

    return (
        <OverflowEntryContext value={entry ?? around}>
            {renderPart("li", "breadcrumb-item", properties, styles.item, {
                ref: (created) => {
                    element = created;
                },
            })}
        </OverflowEntryContext>
    );
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
    return renderPart("a", "breadcrumb-link", properties, [state.focus, styles.link]);
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
    // point along the text
    const locale = useLocale();
    const rest = omit(properties, "children");

    // show only between two that show
    let element: HTMLElement | undefined;
    useOverflowEntry({ kind: "separator", element: () => element });

    return renderPart("li", "breadcrumb-separator", rest, styles.item, {
        role: "presentation" as const,
        "aria-hidden": "true" as const,
        ref: [rest.ref, (created: HTMLElement) => (element = created)],
        get children() {
            return contentOr(properties, () => (
                <Icon icon={locale.direction === "rtl" ? caretLeft : caretRight} />
            ));
        },
    });
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
