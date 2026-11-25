import { t } from "@destack/locale";
import * as style from "@destack/style";
import { media } from "@destack/style/media.stylex";
import { color, motion, radius, space, stroke } from "@destack/theme/tokens.stylex";
import {
    createEffect,
    createMemo,
    For,
    type JSX,
    omit,
    Show,
    untrack,
    useEnvironment,
    useLocale,
} from "@destack/view";
import { CollectionBuilder } from "../collection/index.ts";
import { DragAnnouncer, DragContext, useDragManager } from "../drag/index.ts";
import { tabbablesIn } from "../focus/index.ts";
import { Move } from "../interaction/index.ts";
import {
    contentOr,
    type ElementPartProperties,
    mergeProperties,
    renderPart,
} from "../part/index.ts";
import { Sortable } from "../sortable/index.ts";
import { TabsControl } from "../tabs/index.ts";
import {
    LEAST,
    PaneContext,
    type PaneControl,
    type PaneEntry,
    PaneGroupContext,
    PaneGroupControl,
    type PaneGroupOptions,
    type PaneInset,
    usePane,
    usePaneGroup,
} from "./control.ts";
import {
    type PaneBox,
    type PaneEdge,
    type PaneItem,
    PaneLayout,
    type PaneLeaf,
    type PanePlacement,
} from "./layout.ts";
import { frameOf, LAYERS, PaneFrame } from "./float.tsx";
import { PaneTabs } from "./tabs.tsx";

/** The pixels a key moves a line between panes by. */
const KEY_STEP = 16;

/** How far past its line a handle takes the pointer, more for a finger. */
const REACH = {
    default: `calc(-1 * ${space[1]})`,
    [media.pointerCoarse]: `calc(-1 * ${space[3]})`,
};

/** The share by which sums of shares round. */
const EPSILON = 1e-6;

/** The pixels around a crossing of two lines that take both. */
const CROSSING = 8;

/** The styles of panes. */
const styles = style.create({
    group: {
        position: "relative",
        width: "100%",
        height: "100%",
        minWidth: 0,
        minHeight: 0,
        overflow: "hidden",
        isolation: "isolate",
    },
    sortable: {
        display: "contents",
    },
    pane: {
        position: "absolute",
        display: "flex",
        flexDirection: "column",
        minWidth: 0,
        minHeight: 0,
        overflow: "hidden",
    },
    place: (start: string, top: string, width: string, height: string) => ({
        insetInlineStart: start,
        top,
        width,
        height,
    }),
    layer: (layer: number) => ({ zIndex: layer }),
    round: (start: string, end: string, below: string, beyond: string) => ({
        borderStartStartRadius: start,
        borderStartEndRadius: end,
        borderEndStartRadius: below,
        borderEndEndRadius: beyond,
    }),
    body: {
        flexGrow: 1,
        minHeight: 0,
    },
    content: {
        position: "absolute",
        overflow: "auto",
    },
    shield: {
        position: "absolute",
        inset: 0,
    },
    still: {
        cursor: "default",
    },
    handle: {
        position: "absolute",
        display: "flex",
        touchAction: "none",
        outlineStyle: "none",
        // draw the line in the middle of what takes the pointer
        "::before": {
            content: "''",
            position: "absolute",
            backgroundColor: {
                default: color.border,
                ":is(:focus-visible)": color.ring,
                ":is([data-moving])": color.ring,
                [media.forcedColors]: "CanvasText",
            },
        },
        "::after": { content: "''", position: "absolute" },
    },
    aim: {
        position: "absolute",
        pointerEvents: "none",
        borderRadius: radius[2],
        borderWidth: { default: 0, [media.forcedColors]: stroke.ring },
        borderColor: "Highlight",
        backgroundColor: `color-mix(in oklab, ${color.ring} 20%, transparent)`,
        transitionProperty: "top, inset-inline-start, width, height",
        transitionDuration: motion.durationShort,
        transitionTimingFunction: motion.easingStandard,
    },
});

/** The line of a handle in each direction. */
const handles = style.create({
    horizontal: {
        width: 0,
        cursor: "col-resize",
        "::before": { insetBlock: 0, insetInlineStart: 0, width: stroke.border },
        "::after": { insetBlock: 0, insetInline: REACH },
    },
    vertical: {
        height: 0,
        cursor: "row-resize",
        "::before": { insetInline: 0, insetBlockStart: 0, height: stroke.border },
        "::after": { insetInline: 0, insetBlock: REACH },
    },
});

/** The properties of a group of panes, the native element's attributes included. */
export type PaneGroupProperties = PaneGroupOptions &
    Omit<JSX.HTMLAttributes<HTMLDivElement>, "class" | "style" | "children"> &
    ElementPartProperties & {
        /** Render what an item shows, which stays in place as the item moves. */
        readonly content?: (item: () => PaneItem) => JSX.Element;
        /** Render a pane, the standard one by default. */
        readonly children?: (pane: () => PaneLeaf) => JSX.Element;
        /** What shows without any pane. */
        readonly fallback?: JSX.Element;
    };

/** Render panes side by side, which a person resizes, splits, merges and moves. */
export function PaneGroup(properties: PaneGroupProperties): JSX.Element {
    // share one set of drags with its tabs and panes
    const drags = useDragManager();
    const control = new PaneGroupControl(properties, drags.manager);
    const locale = useLocale();
    const rest = omit(
        properties,
        "layout",
        "onLayoutChange",
        "onMove",
        "reference",
        "content",
        "label",
        "href",
        "children",
        "fallback",
    );
    const panes = createMemo(() => PaneLayout.panes(properties.layout));

    return (
        <PaneGroupContext value={control}>
            <DragContext value={drags.manager}>
                {renderPart("div", "pane-group", rest, styles.group, {
                    role: "group",
                    ref: [(element) => control.attach(element), ...control.handlers(locale)],
                    onKeyDown: (event) => moveFocus(event, control),
                    get children() {
                        return (
                            <Sortable
                                indicator="gap"
                                xstyle={styles.sortable}
                                onMove={(move) => control.moveTab(move)}
                            >
                                <For each={panes()} keyed={(pane) => pane.id}>
                                    {(pane) =>
                                        properties.children === undefined ? (
                                            <PaneProvider pane={pane}>
                                                <Pane />
                                            </PaneProvider>
                                        ) : (
                                            <PaneProvider pane={pane}>
                                                {properties.children(pane)}
                                            </PaneProvider>
                                        )
                                    }
                                </For>
                                <For
                                    each={
                                        properties.layout.maximized === undefined &&
                                        !control.isSingle()
                                            ? properties.layout.floats
                                            : []
                                    }
                                    keyed={(float) => float.id}
                                >
                                    {(float) => (
                                        <PaneFrame
                                            float={float()}
                                            height={heightOf(properties.layout, float().id)}
                                        />
                                    )}
                                </For>
                                <For each={control.drawn()}>
                                    {(item) => (
                                        <PaneHost item={item}>
                                            {(each) => properties.content?.(each)}
                                        </PaneHost>
                                    )}
                                </For>
                                <For
                                    each={control.edges()}
                                    keyed={(edge) =>
                                        [edge.float ?? "", ...edge.path, edge.index].join(".")
                                    }
                                >
                                    {(edge) => <PaneHandle edge={edge()} />}
                                </For>
                                <PaneAim />
                                <Show when={panes().length === 0}>{properties.fallback}</Show>
                            </Sortable>
                        );
                    },
                    get "aria-label"() {
                        return locale.render(t`Panes`);
                    },
                })}
                <Show when={!drags.isShared}>
                    <DragAnnouncer manager={drags.manager} />
                </Show>
            </DragContext>
        </PaneGroupContext>
    );
}

/** Share a pane with its parts. */
function PaneProvider(properties: {
    /** The pane. */
    readonly pane: () => PaneLeaf;
    /** The parts. */
    readonly children: JSX.Element;
}): JSX.Element {
    // share its body and its tabs
    const group = usePaneGroup();
    const pane = untrack(() => properties.pane);
    let body: HTMLElement | undefined;
    const tabs = new CollectionBuilder<TabsControl>();
    const control: PaneControl = {
        pane,
        group,
        setBody: (element) => {
            body = element;
        },
        body: () => body,
        setTabs: (each) => void tabs.add(each),
        tabs: () => tabs.items()[0],
    };

    return <PaneContext value={control}>{properties.children}</PaneContext>;
}

/** The properties of a pane, the native element's attributes included. */
export type PaneProperties = Omit<JSX.HTMLAttributes<HTMLDivElement>, "class" | "style"> &
    ElementPartProperties & {
        /** The types of what it takes, tabs and panes by default. */
        readonly accepts?: readonly string[];
        /** Its least length along both axes in pixels, enough for a tab by default. */
        readonly minSize?: number;
    };

/** Render a pane where the layout has it, its tabs over its content by default. */
export function Pane(properties: PaneProperties): JSX.Element {
    // join the group, taking drops
    const control = usePane();
    const { group } = control;
    const locale = useLocale();
    const id = untrack(() => control.pane().id);
    let element: HTMLElement | undefined;
    const entry: PaneEntry = {
        id,
        element: () => element,
        body: control.body,
        least: (axis) => properties.minSize ?? LEAST[axis],
        accepts: () => properties.accepts,
        tabs: control.tabs,
    };
    group.add(entry);
    const rest = omit(properties, "accepts", "minSize");
    const placement = createMemo(() => group.placement(id));
    const isActive = (): boolean => group.layout().active === id;

    return renderPart(
        "div",
        "pane",
        rest,
        () => {
            const at = placement();

            return [
                styles.pane,
                at !== undefined && [
                    placeOf(at.box),
                    styles.layer(at.height * LAYERS + 1),
                    cornersOf(group.layout(), at),
                ],
            ];
        },
        mergeProperties(
            {
                role: "group" as const,
                "data-pane": id,
                ref: (created: HTMLElement) => {
                    element = created;
                },
                get hidden() {
                    return placement() === undefined;
                },
                get "data-active"() {
                    return isActive() ? "" : undefined;
                },
                get "aria-label"() {
                    const place = PaneLayout.panes(group.layout()).findIndex(
                        (pane) => pane.id === id,
                    );

                    return locale.render(t`Pane ${place + 1}`);
                },
                get "aria-owns"() {
                    // have its contents read with it
                    const selected = PaneLayout.selected(control.pane());

                    return selected !== undefined && group.drawn().includes(selected.id)
                        ? group.contentId(selected.id)
                        : undefined;
                },
                onFocusIn: () => {
                    if (!untrack(isActive)) {
                        group.change((layout) => PaneLayout.activate(layout, id));
                    }
                },
                onPointerDown: () => {
                    if (!untrack(isActive)) {
                        group.change((layout) => PaneLayout.activate(layout, id));
                    }
                },
            },
            {
                get children() {
                    return contentOr(properties, () => (
                        <>
                            <PaneTabs />
                            <PaneContent />
                        </>
                    ));
                },
            },
        ),
    );
}

/** Render the place of a pane's contents. */
export function PaneContent(
    properties: Omit<JSX.HTMLAttributes<HTMLDivElement>, "class" | "style"> & ElementPartProperties,
): JSX.Element {
    // measure the room around it in its pane
    const { pane, group, setBody } = usePane();
    let element: HTMLElement | undefined;
    createEffect(
        () => pane().id,
        (id) => {
            // follow its size and the pane's
            const body = element;
            const around = group.entry(id)?.element();
            if (body === undefined || around === undefined) {
                return undefined;
            }
            const measure = (): void => group.inset(id, insetOf(body, around));
            measure();
            const resizes = new ResizeObserver(measure);
            [body, around].forEach((each) => resizes.observe(each));

            return () => {
                resizes.disconnect();
                group.inset(id, undefined);
            };
        },
    );

    return renderPart("div", "pane-content", properties, styles.body, {
        ref: (created) => {
            element = created;
            setBody(created);
        },
    });
}

/** Render what shows in a pane without items. */
export function PaneEmpty(properties: {
    /** What shows. */
    readonly children?: JSX.Element;
}): JSX.Element {
    const { pane } = usePane();

    return <Show when={pane().items.length === 0}>{properties.children}</Show>;
}

/** The properties of a pane's bar, the native element's attributes included. */
export type PaneBarProperties = Omit<JSX.HTMLAttributes<HTMLDivElement>, "class" | "style"> &
    ElementPartProperties;

/** Render what a person drags a pane by, and presses twice to cover the others with it. */
export function PaneBar(properties: PaneBarProperties): JSX.Element {
    // drag the pane, leaving presses on its tabs to them
    const { pane, group } = usePane();
    const id = untrack(() => pane().id);
    let element: HTMLElement | undefined;
    const draggable = group.drag.draggable(id, () => element);

    // move its float by it, and by the arrow keys while it has the focus
    const locale = useLocale();
    const float = () => PaneLayout.floatOf(group.layout(), id);
    const move = frameOf(group, float).move();

    return renderPart(
        "div",
        "pane-bar",
        properties,
        null,
        mergeProperties(mergeProperties(draggable.pointerAttributes(), move.attributes()), {
            get tabindex() {
                return float() === undefined || group.isLocked() ? undefined : 0;
            },
            get role() {
                return float() === undefined ? undefined : ("group" as const);
            },
            get "aria-label"() {
                return float() === undefined ? undefined : locale.render(t`Move`);
            },
            get "data-floating"() {
                return float() === undefined ? undefined : "";
            },
            ref: (created: HTMLElement) => {
                element = created;
            },
            onDblClick: (event: MouseEvent) => {
                if (!isOnControl(event.target)) {
                    group.toggleMaximized(id);
                }
            },
        }),
    );
}

/** Render an item's content over the body of its pane, which stays in place as the item moves. */
function PaneHost(properties: {
    /** The id of the item. */
    readonly item: string;
    /** Render what it shows. */
    readonly children: (item: () => PaneItem) => JSX.Element;
}): JSX.Element {
    // follow the pane the item is in
    const group = usePaneGroup();
    const locale = useLocale();
    const id = untrack(() => properties.item);
    const last = createMemo<PaneItem>((before) => group.item(id) ?? before ?? { id, value: id });
    const pane = createMemo(() => PaneLayout.paneOf(group.layout(), id));
    const placement = createMemo(() => {
        const current = pane();

        return current === undefined || PaneLayout.selected(current)?.id !== id
            ? undefined
            : group.placement(current.id);
    });
    const place = (): style.Styles => {
        const at = placement();
        const inset = at === undefined ? undefined : group.insets().get(pane()?.id ?? "");

        return at === undefined || inset === undefined
            ? null
            : [
                  placeOf(at.box, inset),
                  styles.layer(at.height * LAYERS + 2),
                  cornersOf(group.layout(), at, inset),
              ];
    };
    const isShown = (): boolean =>
        placement() !== undefined && group.insets().has(pane()?.id ?? "");
    const tabs = (): TabsControl | undefined => group.entry(pane()?.id ?? "")?.tabs();
    const hasTabs = (): boolean => tabs() !== undefined;
    let element: HTMLElement | undefined;
    const isEmbedding = (): boolean => element?.querySelector("iframe") != null;
    const activate = (): void => {
        const current = untrack(pane);
        if (current !== undefined && untrack(group.layout.bind(group)).active !== current.id) {
            group.change((layout) => PaneLayout.activate(layout, current.id));
        }
    };

    return renderPart("div", "pane-host", {}, () => [styles.content, place()], {
        id: group.contentId(id),
        ref: [
            (created: HTMLElement) => (element = created),
            ...TabsControl.handlers(
                tabs,
                () => locale.direction,
                (item) => group.options.reference?.(item),
            ),
        ],
        get "data-pane"() {
            return pane()?.id;
        },
        get role() {
            return hasTabs() ? ("tabpanel" as const) : undefined;
        },
        get "aria-labelledby"() {
            return hasTabs() ? group.tabId(id) : undefined;
        },
        get hidden() {
            return !isShown();
        },
        onFocusIn: activate,
        onPointerDown: activate,
        get children() {
            return (
                <>
                    {properties.children(last)}
                    {/* Cover an embedded page, which hears a drag in place of this one */}
                    <Show when={group.manager.item() !== undefined && isEmbedding()}>
                        <div data-slot="pane-shield" {...style.attrs(styles.shield)} />
                    </Show>
                </>
            );
        },
    });
}

/** Render the line between two parts, which moves along their axis, or both where two cross. */
function PaneHandle(properties: {
    /** The line. */
    readonly edge: PaneEdge;
}): JSX.Element {
    // move by the share of both parts' room since the press
    const group = usePaneGroup();
    const locale = useLocale();
    let element: HTMLElement | undefined;
    let start: readonly PaneEdge[] = [];
    const mirror = (): number => (locale.direction === "rtl" ? -1 : 1);
    const shift = (edge: PaneEdge, offsetX: number, offsetY: number): void => {
        // read the room of both parts
        const bounds = group.element()?.getBoundingClientRect();
        const isInline = edge.direction === "horizontal";
        const room = isInline
            ? (bounds?.width ?? 0) * edge.box.width
            : (bounds?.height ?? 0) * edge.box.height;
        const offset = isInline ? offsetX * mirror() : offsetY;
        if (room > 0) {
            group.resize(edge, edge.share + offset / room);
        }
    };
    const move = new Move(
        {
            distance: 0,
            isDisabled: () => group.isLocked(),
            onMoveStart: (event) => {
                // take the lines that cross it there too
                const edge = untrack(() => properties.edge);
                start = [
                    edge,
                    ...(event.pointerType === "keyboard"
                        ? []
                        : crossing(group, edge, event.clientX, event.clientY)),
                ];
            },
            onMove: (event) => {
                // step by the keys, or follow the pointer
                if (event.pointerType === "keyboard") {
                    const edge = untrack(() => properties.edge);
                    shift(edge, event.deltaX * KEY_STEP, event.deltaY * KEY_STEP);
                } else {
                    for (const edge of start) {
                        shift(edge, event.offsetX, event.offsetY);
                    }
                }
            },
        },
        useEnvironment().clock,
    );

    return renderPart(
        "div",
        "pane-handle",
        {},
        () => {
            const { box, share, direction } = properties.edge;
            const isInline = direction === "horizontal";

            return [
                handles[direction],
                styles.handle,
                styles.layer(heightOf(group.layout(), properties.edge.float) * LAYERS + 3),
                group.isLocked() && styles.still,
                isInline
                    ? styles.place(
                          percent(box.start + box.width * share),
                          percent(box.top),
                          "0px",
                          percent(box.height),
                      )
                    : styles.place(
                          percent(box.start),
                          percent(box.top + box.height * share),
                          percent(box.width),
                          "0px",
                      ),
            ];
        },
        mergeProperties(move.attributes(), {
            role: "separator" as const,
            get tabindex() {
                return group.isLocked() ? -1 : 0;
            },
            ref: (created: HTMLElement) => {
                element = created;
            },
            get "aria-orientation"() {
                return properties.edge.direction === "horizontal"
                    ? ("vertical" as const)
                    : ("horizontal" as const);
            },
            get "aria-label"() {
                return locale.render(t`Resize`);
            },
            "aria-valuemin": 0,
            "aria-valuemax": 100,
            get "aria-valuenow"() {
                return Math.round(properties.edge.share * 100);
            },
            get "data-moving"() {
                return move.isMoving() ? "" : undefined;
            },
            onDblClick: () => {
                // even out the parts of the split
                const edge = untrack(() => properties.edge);
                group.change((layout) => PaneLayout.distribute(layout, edge));
                element?.focus();
            },
        }),
    );
}

/** Render where a dragged tab or pane lands. */
function PaneAim(): JSX.Element {
    // cover the half it takes, or the whole pane
    const group = usePaneGroup();
    const box = createMemo(() => {
        // read where it aims
        const aim = group.drag.aim();
        const placement = aim?.pane === undefined ? undefined : group.placement(aim.pane);
        if (aim === undefined) {
            return undefined;
        } else if (placement === undefined) {
            const share =
                aim.side === "middle" ? 1 : PaneLayout.shareBeside(group.layout(), aim.side);

            return partOf({ start: 0, top: 0, width: 1, height: 1 }, aim.side, share);
        }

        return partOf(placement.box, aim.side, 1 / 2);
    });

    return (
        <Show when={box()}>
            {(shown) => (
                <div
                    data-slot="pane-aim"
                    aria-hidden="true"
                    {...style.attributes([
                        styles.aim,
                        styles.layer((group.layout().floats.length + 1) * LAYERS),
                        placeOf(shown()),
                    ])}
                />
            )}
        </Show>
    );
}

/** Read how high a float is, zero side by side. */
function heightOf(layout: PaneLayout, float: string | undefined): number {
    return float === undefined ? 0 : layout.floats.findIndex(({ id }) => id === float) + 1;
}

/** Report whether two shares are the same but for rounding. */
function isSame(one: number, other: number): boolean {
    return Math.abs(one - other) < EPSILON;
}

/** Write the radius of a corner. */
function round(isRound: boolean): string {
    return isRound ? radius[3] : "0px";
}

/** Round the corners a pane, or what is within an inset of it, has in common with its float. */
function cornersOf(
    layout: PaneLayout,
    { pane, box }: PanePlacement,
    inset: PaneInset = { start: 0, end: 0, top: 0, bottom: 0 },
): style.Styles {
    // leave those side by side square
    const bounds = PaneLayout.floatOf(layout, pane.id)?.bounds;
    if (bounds === undefined || box.width === 1) {
        return null;
    }
    const isStart = isSame(box.start, bounds.start) && inset.start === 0;
    const isTop = isSame(box.top, bounds.top) && inset.top === 0;
    const isEnd = isSame(box.start + box.width, bounds.start + bounds.width) && inset.end === 0;
    const isBottom = isSame(box.top + box.height, bounds.top + bounds.height) && inset.bottom === 0;

    return styles.round(
        round(isTop && isStart),
        round(isTop && isEnd),
        round(isBottom && isStart),
        round(isBottom && isEnd),
    );
}

/** Write a share as a percentage. */
function percent(share: number): string {
    return `${String(share * 100)}%`;
}

/** Write a length in pixels. */
function pixels(length: number): string {
    return `${String(length)}px`;
}

/** Style the place of a box, within an inset. */
function placeOf(
    box: PaneBox,
    inset: PaneInset = { start: 0, end: 0, top: 0, bottom: 0 },
): style.Styles {
    return styles.place(
        `calc(${percent(box.start)} + ${pixels(inset.start)})`,
        `calc(${percent(box.top)} + ${pixels(inset.top)})`,
        `calc(${percent(box.width)} - ${pixels(inset.start + inset.end)})`,
        `calc(${percent(box.height)} - ${pixels(inset.top + inset.bottom)})`,
    );
}

/** Read the room around an element in another. */
function insetOf(element: HTMLElement, around: HTMLElement): PaneInset {
    // read the sides along the text
    const inner = element.getBoundingClientRect();
    const outer = around.getBoundingClientRect();
    const left = inner.left - outer.left;
    const right = outer.right - inner.right;
    const isBackward = getComputedStyle(around).direction === "rtl";

    return {
        start: isBackward ? right : left,
        end: isBackward ? left : right,
        top: inner.top - outer.top,
        bottom: outer.bottom - inner.bottom,
    };
}

/** Read the part of a box at a side of it, all of it for its middle. */
function partOf(
    box: PaneBox,
    side: "start" | "end" | "top" | "bottom" | "middle",
    share: number,
): PaneBox {
    // take the share at its side
    if (side === "start") {
        return { ...box, width: box.width * share };
    } else if (side === "end") {
        return {
            ...box,
            start: box.start + box.width * (1 - share),
            width: box.width * share,
        };
    } else if (side === "top") {
        return { ...box, height: box.height * share };
    } else if (side === "bottom") {
        return {
            ...box,
            top: box.top + box.height * (1 - share),
            height: box.height * share,
        };
    }

    return box;
}

/** List the lines across a line that meet it at a point. */
function crossing(
    group: PaneGroupControl,
    edge: PaneEdge,
    x: number,
    y: number,
): readonly PaneEdge[] {
    // compare in pixels from the group's corner
    const bounds = group.element()?.getBoundingClientRect();
    if (bounds === undefined) {
        return [];
    }
    const isBackward = group.manager.locale.direction === "rtl";
    const along = isBackward ? bounds.right - x : x - bounds.left;
    const down = y - bounds.top;

    return untrack(group.edges).filter((each) => {
        if (each.direction === edge.direction) {
            return false;
        }
        // measure the other line
        const { box, share } = each;
        const isInline = each.direction === "horizontal";
        const line = isInline
            ? (box.start + box.width * share) * bounds.width
            : (box.top + box.height * share) * bounds.height;
        const from = isInline ? box.top * bounds.height : box.start * bounds.width;
        const length = isInline ? box.height * bounds.height : box.width * bounds.width;
        const across = isInline ? along : down;
        const within = isInline ? down : along;

        return (
            Math.abs(across - line) <= CROSSING &&
            within >= from - CROSSING &&
            within <= from + length + CROSSING
        );
    });
}

/** Report whether a press is on a tab or a control. */
function isOnControl(target: EventTarget | null): boolean {
    return (
        target instanceof Element && target.closest("[role=tab], button, a[href], input") !== null
    );
}

/** Move the focus on Tab in the order the panes are read, each with its contents. */
function moveFocus(
    event: KeyboardEvent & { readonly currentTarget: HTMLElement },
    group: PaneGroupControl,
): void {
    // leave other keys, and Tab where something took it
    if (event.key !== "Tab" || event.defaultPrevented) {
        return;
    }
    const root = event.currentTarget;
    const order = untrack(() => group.placements()).flatMap(({ pane }) => {
        // list what takes the focus in the pane and in its content
        const selected = PaneLayout.selected(pane);
        const content =
            selected === undefined
                ? null
                : root.ownerDocument.getElementById(group.contentId(selected.id));
        const element = group.entry(pane.id)?.element();

        return [
            ...(element === undefined ? [] : tabbablesIn(element)),
            ...(content === null ? [] : tabbablesIn(content)),
        ];
    });
    const lines = [
        ...root.querySelectorAll<HTMLElement>(
            "[data-slot=pane-handle][tabindex='0'], [data-slot=frame-side][tabindex='0']",
        ),
    ];
    const all = [...order, ...lines];
    const at = all.findIndex((element) => element === root.ownerDocument.activeElement);
    const next = at === -1 ? undefined : all[at + (event.shiftKey ? -1 : 1)];

    // go to the next there is, else leave the group from its first or its last
    if (next !== undefined) {
        event.preventDefault();
        next.focus();
    } else if (at !== -1) {
        const inside = tabbablesIn(root);
        (event.shiftKey ? inside[0] : inside.at(-1))?.focus({ preventScroll: true });
    }
}
