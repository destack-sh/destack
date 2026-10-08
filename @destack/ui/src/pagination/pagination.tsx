import { Icon } from "@destack/icon";
import caretLeft from "@destack/icon/phosphor/caret-left";
import caretRight from "@destack/icon/phosphor/caret-right";
import { t } from "@destack/locale";
import * as style from "@destack/style";
import { space } from "@destack/theme/tokens.stylex";
import { createControllableSignal, For, type JSX, merge, omit, useLocale } from "@destack/view";
import { type PartAttributes, type Render, rendered } from "../part/index.ts";
import { buttonStyle, type ButtonSize } from "../button/index.ts";
import { visuallyHiddenStyle } from "../visually-hidden/index.ts";

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
});

/** The properties of an element of a pagination, the native element's attributes included. */
export type PaginationElementProperties<Attributes> = Omit<Attributes, "class"> & {
    /** The StyleX styles applied after the element's styles. */
    readonly xstyle?: style.Styles;
};

/** The properties of a link to a page. */
export type PaginationLinkProperties = PaginationElementProperties<
    JSX.AnchorHTMLAttributes<HTMLAnchorElement>
> & {
    /** Whether the link points at the current page. */
    readonly active?: boolean;
    /** The height and padding of the link, a square icon size by default. */
    readonly size?: ButtonSize;
    /** Render another element with the link's attributes, the native anchor by default. */
    readonly render?: Render;
};

/** Render the navigation landmark of a list of pages. */
export function Pagination(
    properties: PaginationElementProperties<JSX.HTMLAttributes<HTMLElement>>,
): JSX.Element {
    const locale = useLocale();
    const rest = omit(properties, "xstyle", "style");

    return (
        <nav
            aria-label={locale.render(t`Pagination`)}
            data-slot="pagination"
            {...rest}
            {...style.attributes([styles.pagination, properties.xstyle], properties.style)}
        />
    );
}

/** Render the list of a pagination's links. */
export function PaginationContent(
    properties: PaginationElementProperties<JSX.HTMLAttributes<HTMLUListElement>>,
): JSX.Element {
    const rest = omit(properties, "xstyle", "style");

    return (
        <ul
            data-slot="pagination-content"
            {...rest}
            {...style.attributes([styles.content, properties.xstyle], properties.style)}
        />
    );
}

/** Render one entry of a pagination's list. */
export function PaginationItem(
    properties: PaginationElementProperties<JSX.LiHTMLAttributes<HTMLLIElement>>,
): JSX.Element {
    const rest = omit(properties, "xstyle", "style");

    return (
        <li
            data-slot="pagination-item"
            {...rest}
            {...style.attributes([properties.xstyle], properties.style)}
        />
    );
}

/** Render a link to a page, marked as the current page when active. */
export function PaginationLink(properties: PaginationLinkProperties): JSX.Element {
    const rest = omit(properties, "active", "size", "xstyle", "style", "render");
    const part: PartAttributes = merge(
        {
            get "aria-current"() {
                return properties.active === true ? "page" : undefined;
            },
            "data-slot": "pagination-link",
            get "data-active"() {
                return properties.active === true ? "true" : undefined;
            },
        },
        () =>
            style.attributes(
                [
                    buttonStyle({
                        variant: properties.active === true ? "outline" : "ghost",
                        size: properties.size ?? "icon",
                    }),
                    properties.xstyle,
                ],
                properties.style,
            ),
    );

    return rendered(properties.render, part, rest, () => <a {...part} {...rest} />);
}

/** Render the link to the previous page, its arrow pointing against the text. */
export function PaginationPrevious(
    properties: Omit<PaginationLinkProperties, "children">,
): JSX.Element {
    const locale = useLocale();
    const rest = omit(properties, "xstyle", "style");

    return (
        <PaginationLink
            aria-label={locale.render(t`Go to the previous page`)}
            data-slot="pagination-previous"
            size="default"
            {...rest}
            xstyle={[styles.step, properties.xstyle]}
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
    const rest = omit(properties, "xstyle", "style");

    return (
        <PaginationLink
            aria-label={locale.render(t`Go to the next page`)}
            data-slot="pagination-next"
            size="default"
            {...rest}
            xstyle={[styles.step, properties.xstyle]}
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
    const rest = omit(properties, "xstyle", "style");

    return (
        <span
            data-slot="pagination-ellipsis"
            {...rest}
            {...style.attributes([styles.ellipsis, properties.xstyle], properties.style)}
        >
            <Icon name="dots-three" />
            <span {...style.attrs(visuallyHiddenStyle())}>{locale.render(t`More pages`)}</span>
        </span>
    );
}

/** The page list's entries: a page number, or a gap of left-out pages. */
export type PaginationEntry = number | "ellipsis";

/** The properties of a page list: the pages, the current one and the links to each. */
export interface PaginationPagesProperties {
    /** The number of pages. */
    readonly count: number;
    /** The current page, from one, which makes the state controlled. */
    readonly page?: number;
    /** The page current at first while uncontrolled, one by default. */
    readonly defaultPage?: number;
    /** Handle the person choosing a page, which keeps the links from navigating. */
    readonly onPageChange?: (page: number) => void;
    /** The address of a page. */
    readonly href: (page: number) => string;
    /** The pages shown on either side of the current one, one by default. */
    readonly siblingCount?: number;
    /** The pages shown at either end, one by default. */
    readonly boundaryCount?: number;
}

/** List the pages from one page to another, none when the range is empty. */
function pageRange(from: number, to: number): number[] {
    return Array.from({ length: Math.max(to - from + 1, 0) }, (_, index) => from + index);
}

/** List the entries of a page list: the pages at either end, the current page with its siblings, and gaps between. */
export function paginationEntries(
    page: number,
    count: number,
    siblings = 1,
    boundaries = 1,
): readonly PaginationEntry[] {
    // the pages at either end
    const start = pageRange(1, Math.min(boundaries, count));
    const end = pageRange(Math.max(count - boundaries + 1, boundaries + 1), count);

    // keep the current page and its siblings clear of the ends
    const first = Math.max(
        Math.min(page - siblings, count - boundaries - siblings * 2 - 1),
        boundaries + 2,
    );
    const last = Math.min(
        Math.max(page + siblings, boundaries + siblings * 2 + 2),
        (end[0] ?? count + 1) - 2,
    );

    // mark left-out pages with a gap or show the one page it would hide
    const before: PaginationEntry[] =
        first > boundaries + 2
            ? ["ellipsis"]
            : boundaries + 1 < count - boundaries
              ? [boundaries + 1]
              : [];
    const after: PaginationEntry[] =
        last < count - boundaries - 1
            ? ["ellipsis"]
            : count - boundaries > boundaries
              ? [count - boundaries]
              : [];

    return [...start, ...before, ...pageRange(first, last), ...after, ...end];
}

/** Render the steps and the links to each page around the current one, telling the change handler of the page chosen. */
export function PaginationPages(properties: PaginationPagesProperties): JSX.Element {
    // follow the controlled page or the list's own
    const [page, setPage] = createControllableSignal({
        isControlled: () => properties.page !== undefined,
        value: () => properties.page ?? 1,
        defaultValue: properties.defaultPage ?? 1,
        onChange: (next) => properties.onPageChange?.(next),
    });
    const go = (target: number) => (event: MouseEvent) => {
        // page in place when the owner handles the change
        if (properties.onPageChange !== undefined) {
            event.preventDefault();
        }
        setPage(target);
    };

    return (
        <PaginationContent>
            <PaginationItem>
                <PaginationPrevious
                    href={properties.href(Math.max(page() - 1, 1))}
                    aria-disabled={page() <= 1 ? "true" : undefined}
                    onClick={go(Math.max(page() - 1, 1))}
                />
            </PaginationItem>
            <For
                each={paginationEntries(
                    page(),
                    properties.count,
                    properties.siblingCount,
                    properties.boundaryCount,
                )}
            >
                {(entry) => (
                    <PaginationItem>
                        {entry === "ellipsis" ? (
                            <PaginationEllipsis />
                        ) : (
                            <PaginationLink
                                href={properties.href(entry)}
                                active={entry === page()}
                                onClick={go(entry)}
                            >
                                {entry}
                            </PaginationLink>
                        )}
                    </PaginationItem>
                )}
            </For>
            <PaginationItem>
                <PaginationNext
                    href={properties.href(Math.min(page() + 1, properties.count))}
                    aria-disabled={page() >= properties.count ? "true" : undefined}
                    onClick={go(Math.min(page() + 1, properties.count))}
                />
            </PaginationItem>
        </PaginationContent>
    );
}
