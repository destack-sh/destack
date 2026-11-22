import * as style from "@destack/style";
import { stroke } from "@destack/theme/tokens.stylex";
import { createEffect, createSignal, type JSX, omit, Show, untrack } from "@destack/view";
import { Scroll } from "../position/index.ts";

/** The styles of a load-more element. */
const styles = style.create({
    more: {
        display: "flex",
        justifyContent: "center",
        minBlockSize: stroke.border,
        minInlineSize: stroke.border,
    },
});

/** The properties of a load-more element, the native element's attributes included. */
export interface LoadMoreProperties extends Omit<JSX.HTMLAttributes<HTMLDivElement>, "class"> {
    /** Load the next items, called as the element comes near the view. */
    readonly onLoadMore: () => void;
    /** Whether items are loading, which shows the children and stops further loads. */
    readonly isLoading?: boolean;
    /** The distance from the view at which loading starts, in views, one by default. */
    readonly scrollOffset?: number;
    /** What shows while loading, such as a spinner. */
    readonly children?: JSX.Element;
    /** The StyleX styles applied after the element's styles. */
    readonly xstyle?: style.Styles;
}

/** Render the end of a list that loads more as it scrolls near the view, after React Aria. */
export function LoadMore(properties: LoadMoreProperties): JSX.Element {
    // watch the element against the view, widened by the offset on every side
    const [element, setElement] = createSignal<HTMLElement>();
    const rest = omit(
        properties,
        "onLoadMore",
        "isLoading",
        "scrollOffset",
        "children",
        "xstyle",
        "style",
    );
    createEffect(
        () => ({
            watched: element(),
            isLoading: properties.isLoading === true,
            scrollOffset: properties.scrollOffset ?? 1,
        }),
        ({ watched, isLoading, scrollOffset }) => {
            // wait for the element, and for a load under way to finish
            if (watched === undefined || isLoading) {
                return undefined;
            }

            // load as the element comes within the offset of its scrolling ancestor
            const margin = `${String(100 * scrollOffset)}%`;
            const observer = new IntersectionObserver(
                (entries) => {
                    if (entries.some((entry) => entry.isIntersecting)) {
                        untrack(() => properties.onLoadMore)();
                    }
                },
                { root: Scroll.parentOf(watched) ?? null, rootMargin: margin },
            );
            observer.observe(watched);

            return () => observer.disconnect();
        },
    );

    return (
        <div
            data-slot="load-more"
            data-loading={properties.isLoading === true ? "" : undefined}
            {...rest}
            ref={setElement}
            {...style.attributes([styles.more, properties.xstyle], properties.style)}
        >
            <Show when={properties.isLoading === true}>{properties.children}</Show>
        </div>
    );
}
