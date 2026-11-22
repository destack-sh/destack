import { nth } from "@destack/schema";
import * as style from "@destack/style";
import {
    type Accessor,
    createContext,
    createEffect,
    createMemo,
    createSignal,
    For,
    type JSX,
    omit,
    Show,
    useContext,
    untrack,
    useEnvironment,
    useLocale,
} from "@destack/view";
import { createElementSize } from "@destack/view/primitives/resize-observer";
import { mergeProperties } from "../part/index.ts";
import type { Axis } from "../position/index.ts";
import {
    type ScrollBehavior,
    type VirtualItem,
    Virtualizer,
    type VirtualRect,
} from "../virtualizer/index.ts";

/** The styles of a virtual list and its items. */
const styles = style.create({
    list: {
        position: "relative",
        overscrollBehavior: "contain",
    },
    vertical: { overflowX: "hidden", overflowY: "auto" },
    horizontal: { overflowX: "auto", overflowY: "hidden" },
    items: {
        position: "relative",
        flexShrink: 0,
    },
    item: {
        position: "absolute",
    },
});

/** The sizes and places of a virtual list's items. */
const places = style.create({
    vertical: (size: number) => ({ blockSize: `${String(size)}px`, inlineSize: "100%" }),
    horizontal: (size: number) => ({ inlineSize: `${String(size)}px`, blockSize: "100%" }),
    down: (start: number, lane: number, lanes: number) => ({
        insetBlockStart: `${String(start)}px`,
        insetInlineStart: `${String((lane / lanes) * 100)}%`,
        inlineSize: `${String(100 / lanes)}%`,
    }),
    across: (start: number, lane: number, lanes: number) => ({
        insetInlineStart: `${String(start)}px`,
        insetBlockStart: `${String((lane / lanes) * 100)}%`,
        blockSize: `${String(100 / lanes)}%`,
    }),
});

/** The virtualizer of the nearest virtual list, null outside one. */
const VirtualizerContext = createContext<Virtualizer | null>(null);

/** Read the virtualizer of the nearest virtual list, refusing elements outside one. */
export function useVirtualizer(): Virtualizer {
    const virtualizer = useContext(VirtualizerContext);
    if (virtualizer === null) {
        throw new TypeError("virtual list part is invalid: it renders outside a virtual list");
    }

    return virtualizer;
}

/** The properties of the items of a virtualizer, the native element's attributes included. */
export interface VirtualItemsProperties<Item> extends Omit<
    JSX.HTMLAttributes<HTMLDivElement>,
    "class" | "children"
> {
    /** The virtualizer that says which items are in view. */
    readonly virtualizer: Virtualizer;
    /** Every item, in order. */
    readonly each: readonly Item[];
    /** The pixels from the scrolling element's start to the first item, 0 by default. */
    readonly scrollMargin?: number;
    /** Read the attributes of an item's element, such as its role. */
    readonly itemProperties?:
        | ((item: Item, index: number) => JSX.HTMLAttributes<HTMLDivElement>)
        | undefined;
    /** Render an item. */
    readonly children: (item: Item, index: Accessor<number>) => JSX.Element;
    /** The StyleX styles applied after the items' styles. */
    readonly xstyle?: style.Styles;
}

/** Render the items a virtualizer has in view, each at its place in the room all of them take. */
export function VirtualItems<Item>(properties: VirtualItemsProperties<Item>): JSX.Element {
    // size the room by every item along the virtualizer's axis
    const virtualizer = properties.virtualizer;
    const isVertical = virtualizer.orientation === "vertical";
    const rest = omit(
        properties,
        "virtualizer",
        "each",
        "scrollMargin",
        "itemProperties",
        "children",
        "xstyle",
        "style",
    );

    return (
        <div
            data-slot="virtual-items"
            {...rest}
            {...style.attributes(
                [
                    styles.items,
                    isVertical
                        ? places.vertical(virtualizer.totalSize())
                        : places.horizontal(virtualizer.totalSize()),
                    properties.xstyle,
                ],
                properties.style,
            )}
        >
            <For each={virtualizer.items()} keyed={(virtual) => virtual.key}>
                {(virtual) => (
                    <VirtualItemView
                        virtual={virtual}
                        virtualizer={virtualizer}
                        each={properties.each}
                        scrollMargin={properties.scrollMargin ?? 0}
                        itemProperties={properties.itemProperties}
                    >
                        {properties.children}
                    </VirtualItemView>
                )}
            </For>
        </div>
    );
}

/** Render one item in view at its place, measured as it renders. */
function VirtualItemView<Item>(properties: {
    /** The item's place in view. */
    readonly virtual: Accessor<VirtualItem>;
    /** The virtualizer that measures it. */
    readonly virtualizer: Virtualizer;
    /** Every item of the list. */
    readonly each: readonly Item[];
    /** The pixels before the list in its scroller. */
    readonly scrollMargin: number;
    /** The attributes of the item's element, such as its role. */
    readonly itemProperties: VirtualItemsProperties<Item>["itemProperties"];
    /** Render the item's content. */
    readonly children: (item: Item, index: Accessor<number>) => JSX.Element;
}): JSX.Element {
    // render the item's content once per item, however often its place changes
    const index = () => properties.virtual().index;
    const item = createMemo(() => properties.each[index()]);
    const start = () => properties.virtual().start - properties.scrollMargin;
    const lane = () => properties.virtual().lane;
    const lanes = () => properties.virtualizer.lanes();

    return (
        <Show when={item()} keyed>
            {(shown) => (
                <div
                    data-slot="virtual-item"
                    {...properties.itemProperties?.(shown, index())}
                    {...properties.virtualizer.itemAttributes(properties.virtual())}
                    {...style.attrs(
                        styles.item,
                        properties.virtualizer.orientation === "vertical"
                            ? places.down(start(), lane(), lanes())
                            : places.across(start(), lane(), lanes()),
                    )}
                >
                    {properties.children(shown, index)}
                </div>
            )}
        </Show>
    );
}

/** The properties of a virtual list, the native element's attributes included. */
export interface VirtualListProperties<Item> extends Omit<
    JSX.HTMLAttributes<HTMLDivElement>,
    "class" | "children"
> {
    /** Every item, in order. */
    readonly each: readonly Item[];
    /** Estimate the size of an item along the scroll before it is measured, in pixels. */
    readonly estimateSize: (index: number) => number;
    /** Read the key of an item, its index by default. */
    readonly itemKey?: (item: Item, index: number) => string | number;
    /** The axis the items run along, vertical by default. */
    readonly orientation?: Axis;
    /** The number of lanes across the scroll, such as the columns of a grid, one by default. */
    readonly lanes?: number;
    /** The room between items, in pixels. */
    readonly gap?: number;
    /** The items rendered beyond the visible ones on either side, eight by default. */
    readonly overscan?: number;
    /** The end the view stays at as items come and go, the start by default. */
    readonly anchorTo?: "start" | "end";
    /** Whether the view follows items added at the end while it is at the end. */
    readonly followOnAppend?: boolean | ScrollBehavior;
    /** The distance from the end that still counts as at the end, in pixels. */
    readonly scrollEndThreshold?: number;
    /** The size the list is taken to have before it is measured. */
    readonly initialRect?: VirtualRect;
    /** What scrolls before the items, such as a heading. */
    readonly before?: JSX.Element;
    /** What scrolls after the items, such as a loading indicator. */
    readonly after?: JSX.Element;
    /** Read the attributes of an item's element, such as its role. */
    readonly itemProperties?:
        | ((item: Item, index: number) => JSX.HTMLAttributes<HTMLDivElement>)
        | undefined;
    /** Render an item. */
    readonly children: (item: Item, index: Accessor<number>) => JSX.Element;
    /** The StyleX styles applied after the list's styles, usually its size. */
    readonly xstyle?: style.Styles;
}

/** Render a long list that scrolls, with only the items in view in the document. */
export function VirtualList<Item>(properties: VirtualListProperties<Item>): JSX.Element {
    // follow the scrolling element and the room taken before the items
    const locale = useLocale();
    const environment = useEnvironment();
    const [scroller, setScroller] = createSignal<HTMLElement>();
    const [lead, setLead] = createSignal<HTMLElement>();
    const leadSize = createElementSize(lead);
    const orientation = untrack(() => properties.orientation ?? "vertical");
    const scrollMargin = () => {
        // take no room before a lead is given, and none until it is measured
        const size = orientation === "vertical" ? leadSize.height : leadSize.width;

        return properties.before === undefined || size === null ? 0 : size;
    };

    // say which items are in view
    const itemKey = untrack(() => properties.itemKey);
    const fixed = untrack(() => ({
        estimateSize: properties.estimateSize,
        gap: properties.gap,
        overscan: properties.overscan,
        anchorTo: properties.anchorTo,
        followOnAppend: properties.followOnAppend,
        scrollEndThreshold: properties.scrollEndThreshold,
        initialRect: properties.initialRect,
    }));
    const virtualizer = new Virtualizer({
        count: () => properties.each.length,
        estimateSize: fixed.estimateSize,
        scrollElement: scroller,
        orientation,
        lanes: () => properties.lanes ?? 1,
        scrollMargin,
        direction: () => locale.direction,
        itemKey:
            itemKey === undefined
                ? undefined
                : createMemo(() => {
                      // key the items as they stand now, which the count reads from too
                      const each = properties.each;

                      return (index: number) => itemKey(nth(each, index), index);
                  }),
        gap: fixed.gap,
        overscan: fixed.overscan,
        anchorTo: fixed.anchorTo,
        followOnAppend: fixed.followOnAppend,
        scrollEndThreshold: fixed.scrollEndThreshold,
        initialRect: fixed.initialRect,
    });

    // start a list that stays at its end at its end
    const isAnchoredToEnd = untrack(() => properties.anchorTo === "end");
    createEffect(scroller, (element) => {
        // leave a list that starts at its start alone
        if (element === undefined || !isAnchoredToEnd) {
            return undefined;
        }

        // put the view at the end at once, before anything watches where it is
        if (orientation === "vertical") {
            element.scrollTop = element.scrollHeight;
        } else {
            element.scrollLeft = element.scrollWidth * (locale.direction === "rtl" ? -1 : 1);
        }

        // then again once the items are measured
        const frame = new AbortController();
        void environment.display.frame(frame.signal).then(
            () => virtualizer.scrollToEnd(),
            (error: unknown) => {
                if (!frame.signal.aborted) {
                    throw error;
                }
            },
        );

        return () => frame.abort();
    });
    const rest = omit(
        properties,
        "each",
        "estimateSize",
        "itemKey",
        "orientation",
        "lanes",
        "gap",
        "overscan",
        "anchorTo",
        "followOnAppend",
        "scrollEndThreshold",
        "initialRect",
        "before",
        "after",
        "itemProperties",
        "children",
        "xstyle",
        "style",
    );

    return (
        <VirtualizerContext value={virtualizer}>
            <div
                data-slot="virtual-list"
                data-orientation={orientation}
                tabindex="0"
                {...mergeProperties({ ref: setScroller }, rest)}
                {...style.attributes(
                    [styles.list, styles[orientation], properties.xstyle],
                    properties.style,
                )}
            >
                {/* Before */}
                <Show when={properties.before}>
                    {(before) => (
                        <div data-slot="virtual-list-before" ref={setLead}>
                            {before()}
                        </div>
                    )}
                </Show>

                {/* Items */}
                <VirtualItems
                    virtualizer={virtualizer}
                    each={properties.each}
                    scrollMargin={scrollMargin()}
                    itemProperties={properties.itemProperties}
                >
                    {properties.children}
                </VirtualItems>

                {/* After */}
                {properties.after}
            </div>
        </VirtualizerContext>
    );
}
