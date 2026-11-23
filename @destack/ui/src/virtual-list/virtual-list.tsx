import { nth, present } from "@destack/schema";
import { stroke } from "@destack/theme/tokens.stylex";
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
    Length,
    type ScrollBehavior,
    type VirtualItem,
    Virtualizer,
    type VirtualRect,
} from "../virtualizer/index.ts";

/** The reach of an item's focus ring outside its box: the ring and its offset. */
const REACH = `calc(${stroke.ring} + ${stroke.border})`;

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
    roomAcross: {
        marginInline: REACH,
    },
    roomDown: {
        marginBlock: REACH,
    },
    roomAround: {
        padding: REACH,
    },
    item: {
        position: "absolute",
    },
});

/** The sizes and places of a virtual list's items. */
const places = style.create({
    vertical: (size: number) => ({
        blockSize: `${String(size)}px`,
        inlineSize: `calc(100% - 2 * ${REACH})`,
    }),
    horizontal: (size: number) => ({
        inlineSize: `${String(size)}px`,
        blockSize: `calc(100% - 2 * ${REACH})`,
    }),
    down: (start: number, lane: number, lanes: number) => ({
        insetBlockStart: `${String(start)}px`,
        insetInlineStart: `${String((lane / lanes) * 100)}%`,
        inlineSize: `${String(100 / lanes)}%`,
    }),
    gapDown: (gap: string) => ({ paddingBlockEnd: gap }),
    gapAcross: (gap: string) => ({ paddingInlineEnd: gap }),
    across: (start: number, lane: number, lanes: number) => ({
        insetInlineStart: `${String(start)}px`,
        insetBlockStart: `${String((lane / lanes) * 100)}%`,
        blockSize: `${String(100 / lanes)}%`,
    }),
});

/** The items a virtual list shows, which the list around it reads, such as a sortable container. */
export interface VirtualWindow {
    /** Every item's key in order, those out of view included. */
    readonly keys: Accessor<readonly string[]>;
    /** Each item's place in that order, by its key. */
    readonly positions: Accessor<ReadonlyMap<string, number>>;
    /** Scroll an item into view by its key, which mounts it. */
    readonly reveal: (key: string) => void;
    /** The element that scrolls the items. */
    readonly scroller: Accessor<HTMLElement | undefined>;
}

/** A list around a virtual list, which takes its window and asks for items mounted out of view. */
export interface VirtualHost {
    /** Take the window of the virtual list inside until the virtual list unmounts. */
    readonly add: (window: VirtualWindow) => void;
    /** The keys of items the virtual list mounts out of view, such as a dragged or focused one. */
    readonly persisted: Accessor<readonly string[]>;
}

/** The list around the nearest virtual list, null where none reads its window. */
export const VirtualHostContext = createContext<VirtualHost | null>(null);

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
    /** The room after each item but the last, in pixels or as a CSS length, measured with it. */
    readonly gap?: number | string;
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
    // size the room by every item along the virtualizer's axis, the virtualizer fixed at mount
    const virtualizer = untrack(() => properties.virtualizer);
    const isVertical = virtualizer.orientation === "vertical";
    const rest = omit(
        properties,
        "virtualizer",
        "each",
        "scrollMargin",
        "gap",
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
                    isVertical ? styles.roomAcross : styles.roomDown,
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
                        gap={properties.gap ?? 0}
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
    /** The room after the item unless it is the last. */
    readonly gap: number | string;
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

    // leave the gap after every item but the last, measured with the item
    const gap = (): style.Styles => {
        // leave no gap after the last item
        const length = lengthOf(properties.gap);
        if (index() === properties.each.length - 1) {
            return null;
        }

        return properties.virtualizer.orientation === "vertical"
            ? places.gapDown(length)
            : places.gapAcross(length);
    };

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
                        gap(),
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
    /** The room between items, in pixels or as a CSS length such as a space token. */
    readonly gap?: number | string;
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

    // read the list around, which needs every item's key
    const host = useContext(VirtualHostContext);
    const itemKey = untrack(() => properties.itemKey);
    if (host !== null && itemKey === undefined) {
        throw new TypeError("virtual list inside a list that reads it needs an item key");
    }
    const keys = createMemo(() =>
        host === null || itemKey === undefined
            ? []
            : properties.each.map((item, index) => String(itemKey(item, index))),
    );
    const positions = createMemo(() => new Map(keys().map((key, index) => [key, index] as const)));
    const persisted = createMemo(
        () =>
            (host?.persisted() ?? []).flatMap((key) => {
                const index = positions().get(key);

                return index === undefined ? [] : [index];
            }),
        { equals: (previous, next) => previous.join() === next.join() },
    );

    // say which items are in view
    const fixed = untrack(() => ({
        estimateSize: properties.estimateSize,
        overscan: properties.overscan,
        anchorTo: properties.anchorTo,
        followOnAppend: properties.followOnAppend,
        scrollEndThreshold: properties.scrollEndThreshold,
        initialRect: properties.initialRect,
    }));
    const gapLength = new Length(() => lengthOf(properties.gap ?? 0));
    const gapPixels = () =>
        typeof properties.gap === "string" ? gapLength.pixels() : (properties.gap ?? 0);

    // leave the items room for their focus rings, none until it is measured on mount
    const reachLength = new Length(() => REACH);
    const reach = () => reachLength.pixels() ?? 0;
    const estimate = createMemo(() => {
        // estimate every item but the last with the gap after it, as its measure has it
        const pixels = gapPixels();

        return (index: number) =>
            index < properties.each.length - 1
                ? fixed.estimateSize(index) + present(pixels, "the virtual list's gap")
                : fixed.estimateSize(index);
    });
    const virtualizer = new Virtualizer({
        count: () => properties.each.length,
        estimateSize: estimate,
        paddingStart: reach,
        paddingEnd: reach,
        isEnabled: () => gapPixels() !== undefined,
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
        overscan: fixed.overscan,
        anchorTo: fixed.anchorTo,
        followOnAppend: fixed.followOnAppend,
        scrollEndThreshold: fixed.scrollEndThreshold,
        initialRect: fixed.initialRect,
        persisted,
    });

    // hand the window to the list around
    host?.add({
        keys,
        positions,
        scroller,
        reveal: (key) => {
            const index = positions().get(key);
            if (index === undefined) {
                throw new RangeError(`virtual list reveal of a missing key '${key}'`);
            }
            virtualizer.scrollToIndex(index);
        },
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
                tabindex={host === null ? "0" : undefined}
                {...mergeProperties({ ref: setScroller }, rest)}
                {...style.attributes(
                    [styles.list, styles[orientation], properties.xstyle],
                    properties.style,
                )}
            >
                {/* Gap length */}
                <Show when={typeof properties.gap === "string"}>{gapLength.element}</Show>
                {reachLength.element}

                {/* Before */}
                <Show when={properties.before}>
                    {(before) => (
                        <div
                            data-slot="virtual-list-before"
                            ref={setLead}
                            {...style.attrs(styles.roomAround)}
                        >
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
                    {...(properties.gap === undefined ? {} : { gap: properties.gap })}
                >
                    {properties.children}
                </VirtualItems>

                {/* After */}
                <Show when={properties.after}>
                    {(after) => (
                        <div data-slot="virtual-list-after" {...style.attrs(styles.roomAround)}>
                            {after()}
                        </div>
                    )}
                </Show>
            </div>
        </VirtualizerContext>
    );
}

/** Write a gap as a CSS length, a number in pixels. */
function lengthOf(gap: number | string): string {
    return typeof gap === "number" ? `${String(gap)}px` : gap;
}
