import arrowLeft from "@destack/icon/phosphor/caret-left";
import arrowRight from "@destack/icon/phosphor/caret-right";
import downloadIcon from "@destack/icon/phosphor/download-simple";
import zoomInIcon from "@destack/icon/phosphor/magnifying-glass-plus";
import zoomOutIcon from "@destack/icon/phosphor/magnifying-glass-minus";
import xIcon from "@destack/icon/phosphor/x";
import { t } from "@destack/locale";
import * as style from "@destack/style";
import { media } from "@destack/style/media.stylex";
import { text } from "@destack/theme/text";
import { color, motion, space, weight } from "@destack/theme/tokens.stylex";
import {
    type Accessor,
    createContext,
    createEffect,
    createSignal,
    For,
    type JSX,
    omit,
    Show,
    untrack,
    useContext,
    useEnvironment,
    useLocale,
} from "@destack/view";
import { isMotionReduced } from "@destack/view/motion";
import { Button, type ButtonProperties } from "../button/index.ts";
import {
    Dialog,
    DialogContent,
    type DialogElementProperties,
    DialogTitle,
} from "../dialog/index.ts";
import { Icon } from "../icon/index.ts";
import { Move, Zoom } from "../interaction/index.ts";
import { OpenChange } from "../layer/index.ts";
import { type ElementPartProperties, mergeProperties, renderPart } from "../part/index.ts";
import { GLIDE } from "../position/index.ts";
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

/** The distance over which an item a finger takes down fades to half, in pixels. */
const FADE_DISTANCE = 600;

/** How far a key scales the picture. */
const KEY_SCALE = 1.5;

/** What takes the pointer and the keys itself, such as a video's controls. */
const OWN = "a, button, input, select, textarea, video, audio, iframe, [contenteditable]";

/** What takes the keys itself. */
const OWN_KEYS = "input, select, textarea, video, audio, [contenteditable]";

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
        "::backdrop": {
            backgroundColor: color.background,
        },
    },
    header: {
        zIndex: 1,
        display: "flex",
        alignItems: "center",
        gap: space[2],
        padding: space[3],
        backgroundColor: color.background,
    },
    title: {
        flex: 1,
        minWidth: 0,
        overflow: "hidden",
        whiteSpace: "nowrap",
        textOverflow: "ellipsis",
        fontWeight: weight.medium,
    },
    count: {
        color: color.mutedForeground,
        fontVariantNumeric: "tabular-nums",
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
        padding: space[3],
        paddingBlockStart: 0,
    },
    shown: {
        zIndex: 1,
    },
    sliding: {
        transitionProperty: "transform, opacity",
        transitionDuration: motion.durationMedium,
        transitionTimingFunction: motion.easingStandard,
    },
    moved: (x: string, y: string, opacity: number) => ({
        transform: `translate(${x}, ${y})`,
        opacity,
    }),
    picture: {
        cursor: { default: "zoom-in", ":is([data-closer])": "grab" },
        transformOrigin: "center",
        willChange: "transform",
    },
    fit: (ratio: number) => ({
        aspectRatio: ratio,
        width: `min(100cqw - 2 * ${space[3]}, (100cqh - ${space[3]}) * ${ratio})`,
    }),
    placed: (x: string, y: string, scale: number) => ({
        transform: `translate(${x}, ${y}) scale(${scale})`,
    }),
    step: {
        position: "absolute",
        zIndex: 1,
        top: `calc(50% - ${space[4]})`,
        backgroundColor: {
            default: color.card,
            ":hover": { default: color.card, [media.hover]: color.muted },
        },
    },
    previous: {
        insetInlineStart: space[3],
    },
    next: {
        insetInlineEnd: space[3],
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
    // move, scale and close by keys
    const control = untrack(() => properties.control);
    const locale = useLocale();
    let element: HTMLElement | undefined;
    const press = (event: KeyboardEvent): void => {
        // leave keys something inside takes itself
        const isOwn = event.target instanceof Element && event.target.matches(OWN_KEYS);
        const isPlain = !event.altKey && !event.ctrlKey && !event.metaKey;
        if (!event.defaultPrevented && !isOwn && isPlain) {
            steer(event, control, locale.direction);
        }
    };

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
                    xstyle={styles.dialog}
                    onKeyDown={press}
                >
                    <div ref={(created) => (element = created)} style={{ display: "contents" }}>
                        <Show when={control.item()}>{(item) => properties.children(item)}</Show>
                    </div>
                </DialogContent>
            </Dialog>
        </ViewerContext>
    );
}

/** Render the row over a viewer's item, such as its title and buttons. */
export function ViewerHeader(properties: DialogElementProperties<HTMLDivElement>): JSX.Element {
    return renderPart("div", "viewer-header", properties, [text.sm, styles.header]);
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
            return (
                properties.children ?? locale.render(t`${viewer.index() + 1} of ${viewer.count()}`)
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
            data-slot="viewer-close"
            variant="ghost"
            size="icon"
            aria-label={locale.render(t`Close`)}
            {...mergeProperties(
                {
                    onClick: (event: MouseEvent) =>
                        viewer.dialog.close(new OpenChange("close-press", event)),
                },
                omit(properties, "children"),
            )}
        >
            {properties.children ?? <Icon icon={xIcon} />}
        </Button>
    );
}

/** Render the button that shows a picture closer or whole, nothing for an item without one. */
export function ViewerZoomToggle(properties: ButtonProperties): JSX.Element {
    // tell whether it shows the picture closer
    const viewer = useViewer();
    const locale = useLocale();
    const isCloser = (): boolean => viewer.scale.isCloser();

    return (
        <Show when={viewer.scale.isScalable()}>
            <Button
                data-slot="viewer-zoom-toggle"
                variant="ghost"
                size="icon"
                aria-label={locale.render(isCloser() ? t`Zoom out` : t`Zoom in`)}
                {...mergeProperties(
                    { onClick: () => viewer.scale.toggle() },
                    omit(properties, "children"),
                )}
            >
                {properties.children ?? <Icon icon={isCloser() ? zoomOutIcon : zoomInIcon} />}
            </Button>
        </Show>
    );
}

/** The properties of a viewer's download, the button's included. */
export interface ViewerDownloadProperties extends ButtonProperties {
    /** The address of the item. */
    readonly href: string;
    /** The name it is saved under, the address's own by default. */
    readonly name?: string;
}

/** Render the link that saves a viewer's item. */
export function ViewerDownload(properties: ViewerDownloadProperties): JSX.Element {
    const locale = useLocale();
    const rest = omit(properties, "href", "name", "children");

    return (
        <Button
            data-slot="viewer-download"
            variant="ghost"
            size="icon"
            aria-label={locale.render(t`Download`)}
            render={(attributes) => (
                <a {...attributes} href={properties.href} download={properties.name ?? ""} />
            )}
            {...rest}
        >
            {properties.children ?? <Icon icon={downloadIcon} />}
        </Button>
    );
}

/** Render the button that shows the item before, nothing before the first. */
export function ViewerPrevious(properties: ButtonProperties): JSX.Element {
    return <ViewerStep step={-1} {...properties} />;
}

/** Render the button that shows the item after, nothing after the last. */
export function ViewerNext(properties: ButtonProperties): JSX.Element {
    return <ViewerStep step={1} {...properties} />;
}

/** Render the button that shows an item a step away. */
function ViewerStep(properties: ButtonProperties & { readonly step: 1 | -1 }): JSX.Element {
    // point the way the item is, mirrored in right-to-left text
    const viewer = useViewer();
    const locale = useLocale();
    const isNext = (): boolean => properties.step === 1;
    const icon = () => (isNext() === (locale.direction === "ltr") ? arrowRight : arrowLeft);

    return (
        <Show when={viewer.has(properties.step)}>
            <Button
                data-slot={isNext() ? "viewer-next" : "viewer-previous"}
                variant="outline"
                size="icon"
                aria-label={locale.render(isNext() ? t`Next` : t`Previous`)}
                {...mergeProperties(
                    { onClick: () => viewer.step(properties.step) },
                    omit(properties, "children", "step", "xstyle"),
                )}
                xstyle={[styles.step, isNext() ? styles.next : styles.previous, properties.xstyle]}
            >
                {properties.children ?? <Icon icon={icon()} />}
            </Button>
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
    let tap: { readonly time: number; readonly at: Point } | undefined;
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
                }
            },
            onMoveEnd: (event) => {
                // leave the item where the finger took it far or fast enough
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
                }
            },
        },
        clock,
    );
    const click = (event: MouseEvent & { readonly currentTarget: HTMLElement }): void => {
        // close beside the item, and show it closer on a double tap
        const at = { x: event.clientX, y: event.clientY };
        const isOn = isWithin(itemOf(event.currentTarget), at);
        const isSecond =
            tap !== undefined &&
            event.timeStamp - tap.time < DOUBLE_TAP &&
            Math.hypot(at.x - tap.at.x, at.y - tap.at.y) < TAP_DISTANCE;
        if (isOn && isSecond) {
            viewer.scale.toggle(at);
        } else if (!isOn) {
            viewer.dialog.close(new OpenChange("outside-press", event));
        }
        tap = isOn && !isSecond ? { time: event.timeStamp, at } : undefined;
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
            }
        },
    );

    // take the focus back where what had it goes away with its item
    let element: HTMLElement | undefined;
    createEffect(
        () => viewer.key(),
        () => {
            queueMicrotask(() => {
                if (viewer.dialog.isOpen() && document.activeElement === document.body) {
                    element?.focus({ preventScroll: true });
                }
            });
        },
    );

    return renderPart(
        "div",
        "viewer-stage",
        rest,
        styles.stage,
        mergeProperties(mergeProperties(zoom.attributes(), move.attributes()), {
            tabindex: -1,
            autofocus: true,
            ref: (target: HTMLElement) => (element = target),
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
    // fade the one a finger takes down, beside which the others stay
    const across = place * (direction === "ltr" ? 1 : -1) * 100;
    const down = place === 0 ? y : 0;
    const opacity = 1 - Math.min(down / FADE_DISTANCE, 0.5);

    return styles.moved(`calc(${String(across)}% + ${String(x)}px)`, `${String(down)}px`, opacity);
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
        ],
        {
            ref: (target: HTMLElement) => (element = target),
            get "data-closer"() {
                return isShown() && viewer.scale.isCloser() ? "" : undefined;
            },
        },
    );
}

/** Move between items, scale the picture, or leave the key alone. */
function steer(event: KeyboardEvent, viewer: ViewerState, direction: "ltr" | "rtl"): void {
    // read the step, the end or the scale the key asks for
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
    } else if (event.key === "+" || event.key === "=") {
        viewer.scale.zoom(KEY_SCALE);
    } else if (event.key === "-") {
        viewer.scale.zoom(1 / KEY_SCALE);
    } else if (event.key === "0") {
        viewer.scale.reset();
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
        { ...GLIDE, direction, fill: direction === "reverse" ? "forwards" : "none" },
    );
}
