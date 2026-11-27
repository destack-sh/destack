import arrowLeft from "@destack/icon/phosphor/caret-left";
import arrowRight from "@destack/icon/phosphor/caret-right";
import downloadIcon from "@destack/icon/phosphor/download-simple";
import zoomInIcon from "@destack/icon/phosphor/magnifying-glass-plus";
import zoomOutIcon from "@destack/icon/phosphor/magnifying-glass-minus";
import xIcon from "@destack/icon/phosphor/x";
import { t } from "@destack/locale";
import * as style from "@destack/style";
import { media } from "@destack/style/media.stylex";
import { state } from "@destack/theme/state";
import { text } from "@destack/theme/text";
import { color, motion, radius, size, space, weight } from "@destack/theme/tokens.stylex";
import {
    type Accessor,
    createContext,
    createEffect,
    createSignal,
    For,
    handle,
    handleOnPage,
    type JSX,
    omit,
    onCleanup,
    Show,
    untrack,
    useContext,
    useEnvironment,
    useLocale,
} from "@destack/view";
import { isMotionReduced } from "@destack/view/motion";
import { createIdleTimer } from "@destack/view/primitives/idle";
import {
    ActionButton,
    type ActionButtonProperties,
    downloadItem,
    nextItem,
    previousItem,
    toggleZoom,
    zoomIn,
    zoomOut,
    zoomToFit,
} from "../action/index.ts";
import { Button, type ButtonProperties } from "../button/index.ts";
import {
    Dialog,
    DialogContent,
    type DialogElementProperties,
    DialogTitle,
} from "../dialog/index.ts";
import { scrim } from "../dialog/scrim.stylex.ts";
import { Icon } from "../icon/index.ts";
import { Move, Zoom } from "../interaction/index.ts";
import { OpenChange } from "../layer/index.ts";
import {
    contentOr,
    type ElementPartProperties,
    mergeProperties,
    renderPart,
} from "../part/index.ts";
import { LayoutAnimation } from "../motion/layout.ts";
import { useViewer, ViewerContext, type ViewerControl, type ViewerState } from "./control.ts";
import { FIT, type Frame, type Place, type Point } from "./scale.ts";

/** The share of the stage a finger moves the item past to leave it. */
const LEAVE_SHARE = 0.2;

/** The speed past which a finger leaves the item, in pixels per millisecond: a flick. */
const LEAVE_SPEED = 0.5;

/** The time between two taps of a double tap, in milliseconds, as platforms wait. */
const DOUBLE_TAP = 300;

/** The distance between two taps of a double tap, in pixels. */
const TAP_DISTANCE = 24;

/** The share of its way an item gives a finger past the last. */
const RESISTANCE = 0.3;

/** The distance over which the room fades as a finger takes the item down, in pixels. */
const FADE_DISTANCE = 600;

/** The share of its size an item loses as a finger takes it down. */
const PULL_SHRINK = 0.35;

/** How far a key scales the picture. */
const KEY_SCALE = 1.5;

/** How long the buttons stay once the pointer stands still, in milliseconds, as PhotoSwipe's. */
const IDLE = 4000;

/** The room around an item: black with a little of the theme. */
const ROOM = `color-mix(in oklab, ${color.background} 22%, black)`;

/** The shade behind what floats over an item. */
const SHADE = "color-mix(in oklab, black 50%, transparent)";

/** The shade behind a button under the pointer. */
const SHADE_HOVER = "color-mix(in oklab, black 70%, transparent)";

/** The height of a thumbnail in a strip. */
const THUMBNAIL_HEIGHT = `calc(12 * ${space[1]})`;

/** The width of a thumbnail in a strip: narrow, so many are there. */
const THUMBNAIL_WIDTH = size[2];

/** The width of the thumbnail of the item shown. */
const CURRENT_WIDTH = `calc(18 * ${space[1]})`;

/** What takes the pointer and the keys itself, such as a video's controls. */
const OWN =
    "a, button, input, select, textarea, video, audio, iframe, [contenteditable], [data-slot=video-player], [data-slot=audio-player]";

/** What takes the keys itself. */
const OWN_KEYS =
    "input, select, textarea, video, audio, [contenteditable], [data-slot=video-player], [data-slot=video-player] *, [data-slot=audio-player], [data-slot=audio-player] *";

/** The styles of a viewer and its parts. */
const styles = style.create({
    dialog: {
        display: { default: "none", ":modal": "flex" },
        flexDirection: "column",
        gap: 0,
        width: "100vw",
        height: "100dvh",
        maxWidth: "none",
        maxHeight: "none",
        margin: 0,
        padding: 0,
        borderWidth: 0,
        borderRadius: 0,
        backgroundColor: "transparent",
        boxShadow: "none",
        overflow: "hidden",
        color: color.foreground,
        colorScheme: "dark",
        "::backdrop": {
            backgroundColor: ROOM,
            colorScheme: "dark",
        },
    },
    pulled: (share: string) => ({
        [scrim.share]: share,
        "::backdrop": { transitionDuration: "0s" },
    }),
    items: {
        display: "contents",
    },
    header: {
        position: "absolute",
        zIndex: 2,
        insetBlockStart: 0,
        insetInline: 0,
        display: "flex",
        alignItems: "center",
        gap: space[3],
        paddingBlockStart: space[2],
        paddingBlockEnd: space[7],
        paddingInlineStart: space[4],
        paddingInlineEnd: space[2],
        backgroundImage: `linear-gradient(${SHADE}, transparent)`,
        pointerEvents: "none",
    },
    title: {
        minWidth: 0,
        overflow: "hidden",
        whiteSpace: "nowrap",
        textOverflow: "ellipsis",
        unicodeBidi: "plaintext",
        fontWeight: weight.medium,
    },
    count: {
        flexShrink: 0,
        color: color.mutedForeground,
        fontVariantNumeric: "tabular-nums",
    },
    actions: {
        display: "flex",
        flexShrink: 0,
        gap: space[1],
        marginInlineStart: "auto",
        pointerEvents: "auto",
    },
    action: {
        borderRadius: radius.full,
        fontSize: `calc(5 * ${space[1]})`,
    },
    chrome: {
        transitionProperty: "opacity",
        transitionDuration: motion.durationMedium,
        transitionTimingFunction: motion.easingStandard,
    },
    alone: {
        opacity: { default: 0, ":is(:focus-visible, :has(:focus-visible))": 1 },
        pointerEvents: {
            default: "none",
            ":is(:focus-visible, :has(:focus-visible))": "auto",
        },
    },
    stage: {
        position: "relative",
        isolation: "isolate",
        flex: 1,
        minHeight: 0,
        containerType: "size",
        touchAction: "none",
        userSelect: "none",
        outlineStyle: "none",
    },
    pane: {
        position: "absolute",
        inset: 0,
        display: "grid",
        placeItems: "center",
    },
    still: {
        cursor: "none",
    },
    shown: {
        zIndex: 1,
    },
    sliding: {
        transitionProperty: "transform",
        transitionDuration: motion.durationMedium,
        transitionTimingFunction: motion.easingStandard,
    },
    moved: (x: string, y: string, scale: number) => ({
        transform: `translate(${x}, ${y}) scale(${scale})`,
    }),
    picture: {
        cursor: { default: "zoom-in", ":is([data-closer])": "grab" },
        transformOrigin: "center",
        willChange: "transform",
    },
    fit: (ratio: number) => ({
        aspectRatio: ratio,
        width: `min(100cqw, 100cqh * ${ratio})`,
    }),
    placed: (x: string, y: string, scale: number) => ({
        transform: `translate(${x}, ${y}) scale(${scale})`,
    }),
    step: {
        position: "absolute",
        zIndex: 2,
        top: `calc(50% - ${size[4]} / 2)`,
        width: size[4],
        height: size[4],
        borderRadius: radius.full,
        borderColor: `color-mix(in oklab, ${color.foreground} 16%, transparent)`,
        backgroundColor: {
            default: SHADE,
            ":hover": { default: null, [media.hover]: SHADE_HOVER },
        },
        backdropFilter: "blur(16px)",
        color: color.foreground,
        fontSize: `calc(5.5 * ${space[1]})`,
        // reach past its circle, as the edge is where a person aims
        "::before": {
            content: '""',
            position: "absolute",
            insetBlock: `calc(-1 * ${space[7]})`,
            insetInline: `calc(-1 * ${space[4]})`,
        },
    },
    swiped: {
        opacity: { default: null, [media.pointerCoarse]: 0 },
        pointerEvents: { default: null, [media.pointerCoarse]: "none" },
    },
    previous: {
        insetInlineStart: space[4],
    },
    next: {
        insetInlineEnd: space[4],
    },
    strip: {
        zIndex: 2,
        display: "flex",
        flexShrink: 0,
        gap: space[1],
        paddingBlock: space[3],
        paddingInline: "50%",
        overflowX: "auto",
        overscrollBehaviorX: "contain",
        scrollbarWidth: "none",
        touchAction: "pan-x",
    },
    thumbnail: {
        display: "grid",
        gridTemplate: "100% / 100%",
        placeItems: "center",
        flexShrink: 0,
        width: THUMBNAIL_WIDTH,
        height: THUMBNAIL_HEIGHT,
        padding: 0,
        borderWidth: 0,
        borderRadius: radius[2],
        overflow: "hidden",
        backgroundColor: color.muted,
        color: color.foreground,
        opacity: { default: 0.6, ":hover": { default: null, [media.hover]: 0.9 } },
        cursor: "pointer",
        transitionProperty: "opacity",
        transitionDuration: motion.durationMedium,
        transitionTimingFunction: motion.easingStandard,
    },
    current: {
        width: CURRENT_WIDTH,
        marginInline: space[1],
        opacity: 1,
    },
});

/** The properties of a viewer. */
export interface ViewerProperties<Item> {
    /** The items and the one it shows. */
    readonly control: ViewerControl<Item>;
    /** Render the item it shows. */
    readonly children: (item: Accessor<Item>) => JSX.Element;
}

/** Render a viewer over the page, showing one item at a time. */
export function Viewer<Item>(properties: ViewerProperties<Item>): JSX.Element {
    // read the control once
    const control = untrack(() => properties.control);
    const locale = useLocale();
    let element: HTMLElement | undefined;

    // leave the item alone once the pointer stands still
    const idle = createIdleTimer({
        events: [],
        idleTimeout: IDLE,
        startManually: true,
        onIdle: () => control.setAlone(true),
    });
    const wake = (): void => {
        control.setAlone(false);
        idle.start();
    };

    // move, scale and close by keys
    const press = (event: KeyboardEvent): void => {
        // bring the buttons back for the focus to move to
        if (event.key === "Tab") {
            wake();
        }

        // leave keys something inside takes itself
        const isOwn = event.target instanceof Element && event.target.matches(OWN_KEYS);
        const isPlain = !event.altKey && !event.ctrlKey && !event.metaKey;
        if (!event.defaultPrevented && !isOwn && isPlain) {
            steer(event, control, locale.direction);
        }
    };

    // offer what a person does as actions
    const { scale } = control;
    const isCloser = () => scale.isScalable() && scale.isCloser();
    const handlers = [
        handle(nextItem, { isEnabled: () => control.has(1), run: () => control.step(1) }),
        handle(previousItem, { isEnabled: () => control.has(-1), run: () => control.step(-1) }),
        handle(zoomIn, { isEnabled: scale.isScalable, run: () => scale.zoom(KEY_SCALE) }),
        handle(zoomOut, { isEnabled: isCloser, run: () => scale.zoom(1 / KEY_SCALE) }),
        handle(toggleZoom, {
            isEnabled: scale.isScalable,
            isOn: () => scale.isCloser(),
            run: () => scale.toggle(),
        }),
        handle(zoomToFit, { isEnabled: isCloser, run: () => scale.reset() }),
    ];

    // bring the rest back as the pointer moves
    const move = (event: PointerEvent): void => {
        if (event.pointerType !== "touch") {
            wake();
        }
    };
    createEffect(control.dialog.isOpen, (isOpen) => {
        if (!isOpen) {
            return undefined;
        }
        control.setAlone(false);

        return () => idle.stop();
    });

    // grow from what stands for the item on the page, and shrink back to it
    createEffect(control.dialog.isOpen, (isOpen, wasOpen) => {
        if (isOpen && element !== undefined) {
            void grow(element, control.shown());
        } else if (!isOpen && wasOpen === true) {
            shrink(element, control.shown());
            queueMicrotask(() => control.restore());
        }
    });

    // close once the item is gone
    createEffect(
        () => control.dialog.isOpen() && control.index() === -1,
        (isGone) => (isGone ? control.dialog.close() : undefined),
    );

    return (
        <ViewerContext value={control}>
            <Dialog control={control.dialog}>
                <DialogContent
                    data-slot="viewer"
                    showCloseButton={false}
                    xstyle={[
                        styles.dialog,
                        control.pull() > 0 && styles.pulled(String(1 - control.pull())),
                    ]}
                    ref={handlers}
                    onKeyDown={press}
                    onPointerMove={move}
                >
                    <div ref={(created) => (element = created)} {...style.attrs(styles.items)}>
                        <Show when={control.item()}>{(item) => properties.children(item)}</Show>
                    </div>
                </DialogContent>
            </Dialog>
        </ViewerContext>
    );
}

/** Render the row over a viewer's item, such as its title and buttons. */
export function ViewerHeader(properties: DialogElementProperties<HTMLDivElement>): JSX.Element {
    const viewer = useViewer();

    return renderPart("div", "viewer-header", properties, () => [
        text.sm,
        styles.header,
        styles.chrome,
        isBare(viewer) && styles.alone,
    ]);
}

/** Render the buttons of a viewer's header, at its end. */
export function ViewerActions(properties: DialogElementProperties<HTMLDivElement>): JSX.Element {
    return renderPart("div", "viewer-actions", properties, styles.actions);
}

/** Render the title of a viewer's item, which labels it. */
export function ViewerTitle(properties: DialogElementProperties<HTMLHeadingElement>): JSX.Element {
    return (
        <DialogTitle
            data-slot="viewer-title"
            {...properties}
            xstyle={[text.sm, styles.title, properties.xstyle]}
        />
    );
}

/** Render which item of how many a viewer shows, told as it changes. */
export function ViewerCount(properties: DialogElementProperties<HTMLSpanElement>): JSX.Element {
    const viewer = useViewer();
    const locale = useLocale();

    return renderPart("span", "viewer-count", properties, styles.count, {
        role: "status",
        get children() {
            return contentOr(properties, () =>
                locale.render(t`${viewer.index() + 1} of ${viewer.count()}`),
            );
        },
    });
}

/** Render the button that closes a viewer. */
export function ViewerClose(properties: ButtonProperties): JSX.Element {
    const viewer = useViewer();
    const locale = useLocale();

    return (
        <Button
            isIconOnly
            data-slot="viewer-close"
            variant="ghost"
            size="lg"
            aria-label={locale.render(t.context("action")`Close`)}
            {...mergeProperties(
                {
                    onClick: (event: MouseEvent) =>
                        viewer.dialog.close(new OpenChange("close-press", event)),
                },
                omit(properties, "children", "xstyle"),
            )}
            xstyle={[styles.action, properties.xstyle]}
        >
            {contentOr(properties, () => (
                <Icon icon={xIcon} />
            ))}
        </Button>
    );
}

/** The properties of a button of a viewer, which runs an action there. */
export type ViewerButtonProperties = Omit<ActionButtonProperties, "action" | "element">;

/** Render the button that shows a picture closer or whole, nothing for an item without one. */
export function ViewerZoomToggle(properties: ViewerButtonProperties): JSX.Element {
    const viewer = useViewer();

    return (
        <Show when={viewer.scale.isScalable()}>
            <ActionButton
                data-slot="viewer-zoom-toggle"
                variant="ghost"
                isIconOnly
                size="lg"
                icon={viewer.scale.isCloser() ? zoomOutIcon : zoomInIcon}
                {...omit(properties, "xstyle")}
                action={toggleZoom}
                xstyle={[styles.action, properties.xstyle]}
            />
        </Show>
    );
}

/** The properties of a viewer's download, the button's included. */
export interface ViewerDownloadProperties extends Omit<
    ViewerButtonProperties,
    "href" | "download"
> {
    /** The address of the item. */
    readonly href: string;
    /** The name it is saved under, the address's own by default. */
    readonly name?: string;
}

/** Render the link that saves a viewer's item. */
export function ViewerDownload(properties: ViewerDownloadProperties): JSX.Element {
    // save the item from anywhere while the viewer is open
    const viewer = useViewer();
    const rest = omit(properties, "href", "name", "xstyle");
    handleOnPage(downloadItem, {
        isEnabled: () => viewer.dialog.isOpen(),
        run: () => {
            // follow a link of its own, as a browser saves by one
            const link = document.createElement("a");
            link.href = properties.href;
            link.download = properties.name ?? "";
            link.target = "_blank";
            link.rel = "noreferrer";
            link.click();
        },
    });

    return (
        <ActionButton
            data-slot="viewer-download"
            variant="ghost"
            isIconOnly
            size="lg"
            icon={downloadIcon}
            {...rest}
            xstyle={[styles.action, properties.xstyle]}
            href={properties.href}
            download={properties.name ?? ""}
            action={downloadItem}
        />
    );
}

/** Render the button that shows the item before, nothing before the first. */
export function ViewerPrevious(properties: ViewerButtonProperties): JSX.Element {
    return <ViewerStep {...properties} step={-1} />;
}

/** Render the button that shows the item after, nothing after the last. */
export function ViewerNext(properties: ViewerButtonProperties): JSX.Element {
    return <ViewerStep {...properties} step={1} />;
}

/** Render the button that shows an item a step away. */
function ViewerStep(
    properties: ViewerButtonProperties & {
        /** The step. */
        readonly step: 1 | -1;
    },
): JSX.Element {
    // point the way the item is, mirrored in right-to-left text
    const viewer = useViewer();
    const locale = useLocale();
    const isNext = (): boolean => properties.step === 1;
    const icon = () => (isNext() === (locale.direction === "ltr") ? arrowRight : arrowLeft);

    return (
        <Show when={viewer.has(properties.step)}>
            <ActionButton
                data-slot={isNext() ? "viewer-next" : "viewer-previous"}
                variant="outline"
                isIconOnly
                size="lg"
                icon={icon()}
                weight="bold"
                {...omit(properties, "step", "xstyle")}
                action={isNext() ? nextItem : previousItem}
                xstyle={[
                    styles.step,
                    styles.chrome,
                    isNext() ? styles.next : styles.previous,
                    styles.swiped,
                    isBare(viewer) && styles.alone,
                    properties.xstyle,
                ]}
            />
        </Show>
    );
}

/** The properties of an element of a viewer, with its own attributes. */
export type ViewerElementProperties = Omit<JSX.HTMLAttributes<HTMLDivElement>, "class"> &
    ElementPartProperties;

/** The properties of a viewer's stage, with its element's attributes. */
export type ViewerStageProperties<Item> = Omit<ViewerElementProperties, "children"> & {
    /** The viewer. */
    readonly control: ViewerControl<Item>;
    /** Render an item: the one shown, and those beside it. */
    readonly children: (item: Accessor<Item>) => JSX.Element;
};

/** Whether the nearest item is the one shown, not one beside it. */
const PaneContext = createContext<Accessor<boolean>>(() => true);

/** Render where a viewer's items stand, which a finger moves beside each other or down to close. */
export function ViewerStage<Item>(properties: ViewerStageProperties<Item>): JSX.Element {
    // follow a finger, or move a picture that is closer
    const viewer = untrack(() => properties.control);
    const locale = useLocale();
    const { clock } = useEnvironment();
    const [offset, setOffset] = createSignal<Point>({ x: 0, y: 0 }, { ownedWrite: true });
    let tap:
        | {
              readonly time: number;
              readonly at: Point;
              readonly timer: ReturnType<typeof clock.setTimeout>;
          }
        | undefined;
    let hasMoved = false;
    let pointer = "";
    onCleanup(() => clock.clearTimeout(tap?.timer));
    const stepOf = (x: number): 1 | -1 => (x < 0 === (locale.direction === "ltr") ? 1 : -1);
    const zoom = new Zoom({
        onZoom: ({ scale, clientX, clientY, panX, panY }) => {
            viewer.scale.zoom(scale, { x: clientX, y: clientY });
            viewer.scale.move(panX, panY);
        },
    });
    const move = new Move(
        {
            keys: [],
            canStart: (event) =>
                !(event.target instanceof Element) || event.target.closest(OWN) === null,
            onMove: (event) => {
                // leave two fingers to the zoom, and give way a little past the last item
                if (zoom.isZooming()) {
                    setOffset({ x: 0, y: 0 });
                } else if (viewer.scale.isCloser()) {
                    viewer.scale.move(event.deltaX, event.deltaY);
                } else if (event.pointerType === "touch") {
                    const { x, y } = alongOne(event.offsetX, event.offsetY);
                    setOffset({ x: viewer.has(stepOf(x)) ? x : x * RESISTANCE, y });
                    viewer.setPull(Math.min(y / FADE_DISTANCE, 1));
                }
            },
            onMoveEnd: (event) => {
                // leave the item where the finger took it far or fast enough
                hasMoved = true;
                const moved = untrack(offset);
                const { width, height } = event.target.getBoundingClientRect();
                const isAcross = isLeaving(moved.x, width, event.velocityX);
                const isDown = moved.y > 0 && isLeaving(moved.y, height, event.velocityY);
                if (isDown) {
                    viewer.dialog.close(new OpenChange("swipe"));
                } else if (isAcross) {
                    viewer.step(stepOf(moved.x));
                }

                // put it back unless it leaves from where the finger took it
                if (!isDown) {
                    setSliding(true);
                    setOffset({ x: 0, y: 0 });
                    viewer.setPull(0);
                }
            },
        },
        clock,
    );
    const click = (event: MouseEvent & { readonly currentTarget: HTMLElement }): void => {
        // leave the click a drag ends with
        if (hasMoved) {
            return;
        }

        // close beside the item
        const at = { x: event.clientX, y: event.clientY };
        const isTouch = pointer === "touch";
        const isSecond =
            tap !== undefined &&
            event.timeStamp - tap.time < DOUBLE_TAP &&
            Math.hypot(at.x - tap.at.x, at.y - tap.at.y) < TAP_DISTANCE;
        if (!isWithin(itemOf(event.currentTarget), at)) {
            viewer.dialog.close(new OpenChange("outside-press", event));
        }
        // show it closer on a click or a double tap, once for a double click
        else if (isTouch ? isSecond : event.detail < 2) {
            clock.clearTimeout(tap?.timer);
            viewer.scale.toggle(at);
            tap = undefined;
        }
        // hide or show the rest on a tap no other follows
        else if (isTouch) {
            const timer = clock.setTimeout(
                () => viewer.setAlone(!untrack(viewer.isAlone)),
                DOUBLE_TAP,
            );
            tap = { time: event.timeStamp, at, timer };
        }
    };
    const rest = omit(properties, "control", "children");

    // slide between items while it stays open, standing at once on the one it opens with
    const [isSliding, setSliding] = createSignal(false, { ownedWrite: true });
    createEffect(
        () => ({ isOpen: viewer.dialog.isOpen(), key: viewer.key() }),
        ({ isOpen }, before) => {
            setSliding(isOpen && before?.isOpen === true);
            if (isOpen && before?.isOpen !== true) {
                setOffset({ x: 0, y: 0 });
                viewer.setPull(0);
            }
        },
    );

    // settle what the item before leaves: its focus, which goes away with it, and what it plays
    let element: HTMLElement | undefined;
    createEffect(
        () => viewer.key(),
        () => {
            queueMicrotask(() => {
                // take the focus back
                if (viewer.dialog.isOpen() && document.activeElement === document.body) {
                    element?.focus({ preventScroll: true });
                }

                // pause what plays beside it
                const beside = "[data-slot=viewer-pane]:not([data-current]) :is(video, audio)";
                for (const each of element?.querySelectorAll<HTMLMediaElement>(beside) ?? []) {
                    each.pause();
                }
            });
        },
    );

    return renderPart(
        "div",
        "viewer-stage",
        rest,
        () => [styles.stage, viewer.isAlone() && styles.still],
        mergeProperties(mergeProperties(zoom.attributes(), move.attributes()), {
            tabindex: -1,
            autofocus: true,
            ref: (target: HTMLElement) => (element = target),
            onPointerDown: (event: PointerEvent) => {
                hasMoved = false;
                pointer = event.pointerType;
            },
            onClick: click,
            // leave the pointer to the stage, as browsers drag pictures
            onDragStart: (event: DragEvent) => event.preventDefault(),
            get children() {
                return (
                    <For each={viewer.panes()} keyed={(pane) => pane.key}>
                        {(pane) => {
                            const isShown = (): boolean => pane().place === 0;

                            return (
                                <PaneContext value={isShown}>
                                    <div
                                        data-slot="viewer-pane"
                                        data-current={isShown() ? "" : undefined}
                                        aria-hidden={isShown() ? undefined : "true"}
                                        inert={!isShown()}
                                        {...style.attributes([
                                            styles.pane,
                                            isShown() && styles.shown,
                                            isSliding() && !move.isMoving() && styles.sliding,
                                            paneAt(pane().place, offset(), locale.direction),
                                        ])}
                                    >
                                        {properties.children(() => pane().item)}
                                    </div>
                                </PaneContext>
                            );
                        }}
                    </For>
                );
            },
        }),
    );
}

/** Return the styles of where an item stands, moved as far as a finger moves it. */
function paneAt(place: number, { x, y }: Point, direction: "ltr" | "rtl"): style.Styles {
    // shrink the one a finger takes down, beside which the others stay
    const across = place * (direction === "ltr" ? 1 : -1) * 100;
    const down = place === 0 ? y : 0;
    const scale = 1 - Math.min(down / FADE_DISTANCE, 1) * PULL_SHRINK;

    return styles.moved(`calc(${String(across)}% + ${String(x)}px)`, `${String(down)}px`, scale);
}

/** The properties of a picture a viewer scales, with its element's attributes. */
export type ViewerZoomProperties = ViewerElementProperties & {
    /** Its width divided by its height, which keeps its place while it loads. */
    readonly ratio: number;
};

/** Render a picture a viewer shows whole, or closer by a pinch, a double tap or keys. */
export function ViewerZoom(properties: ViewerZoomProperties): JSX.Element {
    // measure the picture where it fits, and its stage
    const viewer = useViewer();
    const rest = omit(properties, "ratio");
    let element: HTMLElement | undefined;
    const measure = (): Frame | undefined => {
        const stage = element?.closest("[data-slot=viewer-stage]");

        return element === undefined || stage === null || stage === undefined
            ? undefined
            : {
                  picture: { width: element.offsetWidth, height: element.offsetHeight },
                  box: stage.getBoundingClientRect(),
              };
    };
    const isShown = useContext(PaneContext);
    createEffect(isShown, (isCurrent) => {
        if (!isCurrent) {
            return undefined;
        }
        viewer.scale.attach(measure);

        return () => viewer.scale.detach(measure);
    });
    const place = (): Place => (isShown() ? viewer.scale.place() : FIT);

    return renderPart(
        "div",
        "viewer-zoom",
        rest,
        () => [
            styles.picture,
            styles.fit(properties.ratio),
            styles.placed(`${String(place().x)}px`, `${String(place().y)}px`, place().scale),
            viewer.isAlone() && styles.still,
        ],
        {
            ref: (target: HTMLElement) => (element = target),
            get "data-closer"() {
                return isShown() && viewer.scale.isCloser() ? "" : undefined;
            },
        },
    );
}

/** The properties of a viewer's strip, with its element's attributes. */
export type ViewerStripProperties<Item> = Omit<ViewerElementProperties, "children"> & {
    /** The viewer. */
    readonly control: ViewerControl<Item>;
    /** Read what an item is called. */
    readonly label: (item: Item) => string;
    /** Render an item's thumbnail. */
    readonly children: (item: Accessor<Item>) => JSX.Element;
};

/** Render a viewer's items as a strip of thumbnails, nothing for a lone item. */
export function ViewerStrip<Item>(properties: ViewerStripProperties<Item>): JSX.Element {
    return (
        <Show when={properties.control.count() > 1}>
            <Thumbnails {...properties} />
        </Show>
    );
}

/** Render a viewer's thumbnails, that of the item shown in the middle. */
function Thumbnails<Item>(properties: ViewerStripProperties<Item>): JSX.Element {
    // keep it in the middle, at once as the viewer opens
    const viewer = untrack(() => properties.control);
    const rest = omit(properties, "control", "label", "children");
    let strip: HTMLElement | undefined;
    createEffect(
        () => ({ isOpen: viewer.dialog.isOpen(), key: viewer.key() }),
        ({ isOpen }, before) => {
            const isFirst = before?.isOpen !== true;
            queueMicrotask(() => {
                if (isOpen && strip !== undefined) {
                    center(strip, isFirst || isMotionReduced(strip));
                }
            });
        },
    );

    return renderPart(
        "div",
        "viewer-strip",
        rest,
        () => [styles.strip, styles.chrome, isBare(viewer) && styles.alone],
        {
            role: "group",
            tabindex: -1,
            ref: (element: HTMLElement) => (strip = element),
            get children() {
                return (
                    <For each={viewer.items} keyed={(item) => viewer.keyOf(item)}>
                        {(item) => {
                            const isShown = (): boolean => viewer.key() === viewer.keyOf(item());

                            return (
                                <button
                                    type="button"
                                    data-slot="viewer-thumbnail"
                                    aria-label={properties.label(item())}
                                    aria-current={isShown() ? "true" : undefined}
                                    tabindex={isShown() ? 0 : -1}
                                    onClick={() => viewer.show(item())}
                                    {...style.attrs(
                                        state.focusInset,
                                        styles.thumbnail,
                                        isShown() && styles.current,
                                    )}
                                >
                                    {properties.children(item)}
                                </button>
                            );
                        }}
                    </For>
                );
            },
        },
    );
}

/** Report whether what floats over the item is hidden, as while a finger takes it down. */
function isBare(viewer: ViewerState): boolean {
    return viewer.isAlone() || viewer.pull() > 0;
}

/** Scroll a strip so the thumbnail of the item shown stands in its middle. */
function center(strip: HTMLElement, isInstant: boolean): void {
    const current = strip.querySelector("[aria-current]");
    if (current instanceof HTMLElement) {
        current.scrollIntoView({
            block: "nearest",
            inline: "center",
            behavior: isInstant ? "instant" : "smooth",
        });
    }
}

/** Move between items by the arrow keys, Home and End, or leave the key alone. */
function steer(event: KeyboardEvent, viewer: ViewerState, direction: "ltr" | "rtl"): void {
    // read the step or the end the key asks for, mirrored in right-to-left text
    const forward = direction === "ltr" ? "ArrowRight" : "ArrowLeft";
    const backward = direction === "ltr" ? "ArrowLeft" : "ArrowRight";
    if (event.key === forward) {
        viewer.step(1);
    } else if (event.key === backward) {
        viewer.step(-1);
    } else if (event.key === "Home") {
        viewer.end("first");
    } else if (event.key === "End") {
        viewer.end("last");
    }
    // leave any other key
    else {
        return;
    }
    event.preventDefault();
}

/** Find the element of a stage's item. */
function itemOf(stage: Element): Element | undefined {
    return (
        stage.querySelector("[data-slot=viewer-pane][data-current]")?.firstElementChild ?? undefined
    );
}

/** Report whether a point is on an element. */
function isWithin(element: Element | undefined, { x, y }: Point): boolean {
    const box = element?.getBoundingClientRect();

    return box !== undefined && x >= box.left && x <= box.right && y >= box.top && y <= box.bottom;
}

/** Keep a finger's way to the axis it moves most along, down alone. */
function alongOne(x: number, y: number): Point {
    return Math.abs(x) > Math.abs(y) ? { x, y: 0 } : { x: 0, y: Math.max(y, 0) };
}

/** Report whether a finger took an item far or fast enough to leave it. */
function isLeaving(distance: number, extent: number, speed: number): boolean {
    const isFast = Math.abs(speed) > LEAVE_SPEED && Math.sign(speed) === Math.sign(distance);

    return distance !== 0 && (Math.abs(distance) > extent * LEAVE_SHARE || isFast);
}

/** Grow a viewer's item from what stands for it on the page, once it is there. */
async function grow(element: HTMLElement, thumbnail: HTMLElement | undefined): Promise<void> {
    await Promise.resolve();
    morph(element, thumbnail, "normal");
}

/** Shrink a viewer's item back to what stands for it on the page. */
function shrink(element: HTMLElement | undefined, thumbnail: HTMLElement | undefined): void {
    if (element !== undefined) {
        morph(element, thumbnail, "reverse");
    }
}

/** Play an item between its place and its thumbnail's, cut to the thumbnail's shape there. */
function morph(
    element: HTMLElement,
    thumbnail: HTMLElement | undefined,
    direction: "normal" | "reverse",
): void {
    // leave it to the dialog's fade without a thumbnail, or where the person wants less motion
    const stage = element.querySelector("[data-slot=viewer-stage]");
    const item = stage === null ? undefined : itemOf(stage);
    if (
        thumbnail === undefined ||
        !(item instanceof HTMLElement) ||
        typeof item.animate !== "function" ||
        isMotionReduced(element)
    ) {
        return;
    }
    const to = item.getBoundingClientRect();
    const from = thumbnail.getBoundingClientRect();
    if (to.width === 0 || to.height === 0 || from.width === 0) {
        return;
    }

    // scale it to cover the thumbnail, its middle on the thumbnail's
    const scale = Math.max(from.width / to.width, from.height / to.height);
    const x = from.left + from.width / 2 - (to.left + to.width / 2);
    const y = from.top + from.height / 2 - (to.top + to.height / 2);
    const across = (to.width - from.width / scale) / 2;
    const down = (to.height - from.height / scale) / 2;
    item.animate(
        [
            {
                transform: `translate(${String(x)}px, ${String(y)}px) scale(${String(scale)})`,
                clipPath: `inset(${String(down)}px ${String(across)}px)`,
            },
            { transform: "none", clipPath: "inset(0px 0px)" },
        ],
        {
            ...new LayoutAnimation(element).timing("short"),
            direction,
            fill: direction === "reverse" ? "forwards" : "none",
        },
    );
}
