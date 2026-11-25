import * as style from "@destack/style";
import { media } from "@destack/style/media.stylex";
import { state } from "@destack/theme/state";
import { color, radius, space, weight } from "@destack/theme/tokens.stylex";
import { text } from "@destack/theme/text";
import {
    type Accessor,
    createContext,
    createMemo,
    createSignal,
    createUniqueId,
    For,
    handle,
    type JSX,
    omit,
    Show,
    useContext,
    useLocale,
} from "@destack/view";
import { createElementSize } from "@destack/view/primitives/resize-observer";
import { openItem } from "../action/index.ts";
import { ListState } from "../focus/index.ts";
import { type ElementPartProperties, renderPart } from "../part/index.ts";
import { type Box, CollageLayout, type Flow, type Layout } from "./layout.ts";

/** The most of a gallery's width a row is high, in percent, so a phone shows a few beside it. */
const NARROW_SHARE = 25;

/** How far the last row of photos is stretched, beside the filler that takes its room. */
const FILLER = 1_000_000;

/** The styles of a gallery and its items. */
const styles = style.create({
    frame: {
        containerType: "inline-size",
        width: "100%",
    },
    grid: (minimum: string, gap: string) => ({
        display: "grid",
        gridTemplateColumns: `repeat(auto-fill, minmax(min(100%, ${minimum}), 1fr))`,
        gap,
    }),
    rows: (gap: string) => ({
        display: "flex",
        flexWrap: "wrap",
        gap,
    }),
    collage: (rows: string, ratio: number, gap: string) => ({
        display: "grid",
        gridTemplateColumns: "repeat(4, minmax(0, 1fr))",
        gridTemplateRows: rows,
        aspectRatio: ratio,
        gap,
    }),
    placed: (height: string) => ({
        position: "relative",
        height,
    }),
    item: {
        position: "relative",
        minWidth: 0,
        overflow: "hidden",
        borderRadius: radius[3],
        backgroundColor: color.muted,
    },
    ratio: (ratio: number) => ({
        aspectRatio: ratio,
    }),
    row: (ratio: number, height: string) => ({
        flexGrow: ratio,
        flexShrink: 1,
        flexBasis: `calc(${ratio} * min(${height}, ${NARROW_SHARE}cqi))`,
        maxWidth: "100%",
    }),
    area: (area: string) => ({
        gridArea: area,
    }),
    box: (start: string, top: string, width: string, height: string) => ({
        position: "absolute",
        insetInlineStart: start,
        top,
        width,
        height,
    }),
    filler: (gap: string) => ({
        flexGrow: FILLER,
        flexBasis: 0,
        height: 0,
        marginBlockStart: `calc(-1 * ${gap})`,
    }),
    content: {
        display: "block",
        width: "100%",
        height: "100%",
        objectFit: "cover",
    },
    open: {
        position: "absolute",
        zIndex: 1,
        inset: 0,
        borderRadius: "inherit",
        outlineOffset: "-3px",
        cursor: "pointer",
    },
    more: {
        position: "absolute",
        inset: 0,
        display: "grid",
        placeItems: "center",
        backgroundColor: {
            default: `color-mix(in oklab, ${color.foreground} 65%, transparent)`,
            [media.forcedColors]: "Canvas",
        },
        color: { default: color.background, [media.forcedColors]: "CanvasText" },
        fontWeight: weight.medium,
        pointerEvents: "none",
    },
});

/** An item of a gallery as it is laid out. */
interface GalleryItemState {
    /** The styles that place it. */
    readonly place: Accessor<style.Styles>;
    /** How many items have no room after it. */
    readonly more: Accessor<number>;
    /** The gallery's Tab stop. */
    readonly list: ListState;
}

/** The nearest item, null outside one. */
const GalleryItemContext = createContext<GalleryItemState | null>(null);

/** The properties of a gallery, with its element's attributes. */
export interface GalleryProperties<Item>
    extends
        Omit<JSX.HTMLAttributes<HTMLDivElement>, "class" | "style" | "children">,
        ElementPartProperties {
    /** The items. */
    readonly each: readonly Item[];
    /** Read an item's ratio of width to height, square by default. */
    readonly ratio?: (item: Item) => number;
    /** How it lays the items out. */
    readonly layout: Layout;
    /** Render an item, as a `GalleryItem`. */
    readonly children: (item: Accessor<Item>, index: number) => JSX.Element;
}

/** Render items in a layout, laid out by the browser itself where it can. */
export function Gallery<Item>(properties: GalleryProperties<Item>): JSX.Element {
    // show as many items as the layout has room for
    const locale = useLocale();
    const rest = omit(properties, "each", "ratio", "layout", "children");
    const list = new ListState({ orientation: "both", isLooping: false, isTypeahead: false });
    const flow = (): Flow | undefined => properties.layout.flow;
    const shown = createMemo((): readonly Item[] => {
        const current = flow();

        return current?.kind === "collage"
            ? properties.each.slice(0, current.count)
            : properties.each;
    });
    const ratioOf = (item: Item): number => properties.ratio?.(item) ?? 1;

    // place the items of a layout the browser has none for once measured
    const [frame, setFrame] = createSignal<HTMLElement | undefined>(undefined, {
        ownedWrite: true,
    });
    const size = createElementSize(frame);
    const boxes = createMemo((): readonly Box[] | undefined =>
        flow() === undefined && size.width !== null && size.width > 0
            ? properties.layout.place(shown().map(ratioOf), size.width)
            : undefined,
    );
    const gap = (): string => `${String(properties.layout.gap)}px`;

    return renderPart("div", "gallery", rest, styles.frame, {
        ref: (element: HTMLElement) => setFrame(element),
        get children() {
            return (
                <div
                    role="list"
                    data-slot="gallery-items"
                    data-flow={flow()?.kind ?? "placed"}
                    onKeyDown={(event) => {
                        // move between the items that open
                        if (
                            event.target instanceof Element &&
                            event.target.matches("[data-slot=gallery-open]")
                        ) {
                            list.moveAcross(event, locale.direction);
                        }
                    }}
                    onFocusOut={(event) => list.focus.focusOut(event)}
                    {...style.attributes([containerOf(flow(), boxes(), shown().length, gap())])}
                >
                    <For each={shown()} keyed={false}>
                        {(item, index) => (
                            <GalleryItemContext
                                value={{
                                    list,
                                    place: () =>
                                        placeOf(
                                            flow(),
                                            boxes()?.[index],
                                            ratioOf(item()),
                                            index,
                                            shown().length,
                                        ),
                                    more: () =>
                                        index === shown().length - 1
                                            ? properties.each.length - shown().length
                                            : 0,
                                }}
                            >
                                {properties.children(item, index)}
                            </GalleryItemContext>
                        )}
                    </For>
                    <Show when={flow()?.kind === "rows"}>
                        <div aria-hidden="true" {...style.attrs(styles.filler(gap()))} />
                    </Show>
                </div>
            );
        },
    });
}

/** What opens an item: what it does, and its label, such as the photo's description. */
type GalleryItemOpen =
    | {
          readonly onOpen: (event: MouseEvent & { readonly currentTarget: HTMLElement }) => void;
          readonly label: string;
      }
    | { readonly onOpen?: undefined; readonly label?: undefined };

/** The properties of a gallery's item, with its element's attributes. */
export type GalleryItemProperties = Omit<JSX.HTMLAttributes<HTMLDivElement>, "class" | "style"> &
    ElementPartProperties &
    GalleryItemOpen;

/** Render an item where its gallery places it, such as a photo. */
export function GalleryItem(properties: GalleryItemProperties): JSX.Element {
    // join the gallery's Tab stop where it opens
    const item = useContext(GalleryItemContext);
    if (item === null) {
        throw new TypeError("a gallery item needs a gallery around it");
    }
    const locale = useLocale();
    const rest = omit(properties, "onOpen", "label", "children");
    const more = (): string =>
        item.more() > 0 ? locale.number(item.more(), { signDisplay: "always" }) : "";
    const key = createUniqueId();
    let button: HTMLElement | undefined;
    item.list.add({
        key,
        text: () => properties.label ?? "",
        isDisabled: () => properties.onOpen === undefined,
        element: () => button,
    });

    return renderPart("div", "gallery-item", rest, () => [styles.item, item.place()], {
        role: "listitem",
        get children() {
            return (
                <>
                    {properties.children}
                    <Show when={item.more() > 0}>
                        <span
                            aria-hidden="true"
                            data-slot="gallery-more"
                            {...style.attrs(text["2xl"], styles.more)}
                        >
                            {more()}
                        </span>
                    </Show>
                    <Show when={properties.onOpen}>
                        {(open) => (
                            <button
                                type="button"
                                data-slot="gallery-open"
                                aria-label={[properties.label, more()].join(" ").trim()}
                                tabindex={item.list.focus.isActive(key) ? 0 : -1}
                                ref={[
                                    (element: HTMLElement) => (button = element),
                                    handle(openItem, { run: () => button?.click() }),
                                ]}
                                onFocus={() => item.list.focus.focusIn(key)}
                                onClick={(event) => open()(event)}
                                {...style.attrs(state.focus, styles.open)}
                            />
                        )}
                    </Show>
                </>
            );
        },
    });
}

/** Return the StyleX styles of content that fills its item, such as a photo, cut to its shape. */
export function galleryContentStyle(): style.Styles {
    return styles.content;
}

/** Return the styles of the element the items are in. */
function containerOf(
    flow: Flow | undefined,
    boxes: readonly Box[] | undefined,
    count: number,
    gap: string,
): style.Styles {
    // let the browser lay them out
    if (flow?.kind === "grid") {
        return styles.grid(`${String(flow.minimum)}px`, gap);
    } else if (flow?.kind === "rows") {
        return styles.rows(gap);
    } else if (flow?.kind === "collage") {
        const rows = `repeat(${CollageLayout.rows(count)}, minmax(0, 1fr))`;

        return styles.collage(rows, flow.ratio, gap);
    }

    // stand as tall as the lowest box, in a plain grid until measured
    const height = (boxes ?? []).reduce((low, box) => Math.max(low, box.y + box.height), 0);

    return boxes === undefined
        ? styles.grid(`calc(40 * ${space[1]})`, gap)
        : styles.placed(`${String(height)}px`);
}

/** Return the styles that place an item. */
function placeOf(
    flow: Flow | undefined,
    box: Box | undefined,
    ratio: number,
    index: number,
    count: number,
): style.Styles {
    // place it where the layout says
    if (box !== undefined) {
        const [left, top, width, height] = [box.x, box.y, box.width, box.height].map(
            (each) => `${String(each)}px`,
        );

        return styles.box(left ?? "", top ?? "", width ?? "", height ?? "");
    }
    // else let the browser place it
    else if (flow?.kind === "rows") {
        return [styles.ratio(ratio), styles.row(ratio, `${String(flow.height)}px`)];
    } else if (flow?.kind === "collage") {
        return styles.area(areaOf(index, count));
    }

    return styles.ratio(flow?.kind === "grid" ? flow.ratio : ratio);
}

/** Return the grid area of a collage's item: its rows and columns. */
function areaOf(index: number, count: number): string {
    const { x, y, width, height } = CollageLayout.area(index, count);

    return `${y + 1} / ${x + 1} / span ${height} / span ${width}`;
}
