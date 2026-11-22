import { type Accessor, type JSX, omit, untrack } from "@destack/view";
import { tabbablesIn } from "../focus/index.ts";
import { LoadMore } from "../load-more/index.ts";
import { mergeProperties } from "../part/index.ts";
import { VirtualList, type VirtualListProperties } from "../virtual-list/index.ts";

/** The properties of a feed, the native element's attributes included. */
export interface FeedProperties<Item> extends Omit<
    VirtualListProperties<Item>,
    "orientation" | "lanes" | "before" | "after" | "itemProperties"
> {
    /** Read the label of an item, which labels its article. */
    readonly label: (item: Item) => string;
    /** Load more items: older ones in a feed that stays at its end, later ones otherwise. */
    readonly onLoadMore?: () => void;
    /** Whether items are loading. */
    readonly isLoading?: boolean;
    /** The number of items there are in all, where known. */
    readonly total?: number;
    /** What shows while loading, such as a spinner. */
    readonly loading?: JSX.Element;
    /** Render an item. */
    readonly children: (item: Item, index: Accessor<number>) => JSX.Element;
}

/** Render an endless list of articles that loads more as it scrolls, as the ARIA feed pattern. */
export function Feed<Item>(properties: FeedProperties<Item>): JSX.Element {
    // load at the start of a feed that stays at its end, such as a conversation
    const rest = omit(
        properties,
        "label",
        "onLoadMore",
        "isLoading",
        "total",
        "loading",
        "children",
    );
    const isLoading = () => properties.isLoading === true;
    const more =
        untrack(() => properties.onLoadMore) === undefined ? undefined : (
            <LoadMore onLoadMore={() => properties.onLoadMore?.()} isLoading={isLoading()}>
                {properties.loading}
            </LoadMore>
        );
    const isReversed = untrack(() => properties.anchorTo === "end");

    // move between articles with the page keys, and past the feed with control and Home or End
    const keys = {
        onKeyDown: (event: KeyboardEvent & { readonly currentTarget: HTMLElement }) => {
            // leave the feed for the element before or after it
            const feed = event.currentTarget;
            if (event.ctrlKey && (event.key === "Home" || event.key === "End")) {
                const outside = tabbablesIn(feed.ownerDocument).filter(
                    (each) => !feed.contains(each),
                );
                const isAfter = (each: HTMLElement) =>
                    (feed.compareDocumentPosition(each) & Node.DOCUMENT_POSITION_FOLLOWING) !== 0;
                const target =
                    event.key === "End"
                        ? outside.find(isAfter)
                        : outside.findLast((each) => !isAfter(each));
                event.preventDefault();
                target?.focus();

                return;
            }

            // find the article the focus is in
            const step = event.key === "PageDown" ? 1 : event.key === "PageUp" ? -1 : 0;
            const article =
                event.target instanceof Element ? event.target.closest("[role=article]") : null;
            if (article === null) {
                return;
            }
            const index = Number(article.getAttribute("data-index"));
            const next = feed.querySelector(`[role=article][data-index="${String(index + step)}"]`);

            // focus the next or the previous one
            if (step !== 0 && next instanceof HTMLElement) {
                event.preventDefault();
                next.focus();
            }
        },
    };

    return (
        <VirtualList
            role="feed"
            tabindex="-1"
            aria-busy={isLoading() ? "true" : "false"}
            {...mergeProperties(keys, rest)}
            data-slot="feed"
            before={isReversed ? more : undefined}
            after={isReversed ? undefined : more}
            itemProperties={(item, index) => ({
                role: "article",
                tabindex: 0,
                "aria-label": properties.label(item),
                "aria-posinset": index + 1,
                // tell assistive technology the total is unknown with -1, as ARIA defines
                "aria-setsize": properties.total ?? -1,
            })}
        >
            {properties.children}
        </VirtualList>
    );
}
