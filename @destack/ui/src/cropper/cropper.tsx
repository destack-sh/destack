import arrowClockwiseIcon from "@destack/icon/phosphor/arrow-clockwise";
import arrowCounterClockwiseIcon from "@destack/icon/phosphor/arrow-counter-clockwise";
import flipHorizontalIcon from "@destack/icon/phosphor/flip-horizontal";
import flipVerticalIcon from "@destack/icon/phosphor/flip-vertical";
import { t } from "@destack/locale";
import * as style from "@destack/style";
import { media } from "@destack/style/media.stylex";
import { color, motion, radius, size, space, stroke } from "@destack/theme/tokens.stylex";
import { For, type JSX, omit, onSettled, Show, useLocale } from "@destack/view";
import { Button, type ButtonProperties } from "../button/index.ts";
import { Icon } from "../icon/index.ts";
import {
    contentOr,
    type ElementPartProperties,
    mergeProperties,
    renderPart,
    type Render,
} from "../part/index.ts";
import { Slider, type SliderProperties } from "../slider/index.ts";
import {
    CropperContext,
    CropperControl,
    type CropperOptions,
    type CropPosition,
    useCropper,
} from "./control.ts";

/** The share of the image an arrow key moves the part by. */
const STEP = 0.01;

/** The steps an arrow key takes with Shift. */
const LARGE_STEP = 10;

/** How much a key zooms in or out. */
const ZOOM_STEP = 0.1;

/** How much a turn of the wheel zooms, in shares for each pixel. */
const WHEEL_ZOOM = 0.002;

/** The sides and corners handles move. */
const POSITIONS = [
    "top",
    "top-right",
    "right",
    "bottom-right",
    "bottom",
    "bottom-left",
    "left",
    "top-left",
] as const;

/** The room around a handle that takes presses. */
const HANDLE = size[1];

/** The shade over the rest of the image, wider than any view. */
const SCRIM = "0 0 0 9999px rgb(0 0 0 / 0.5)";

/** The lines at the thirds of the frame, along one side. */
const LINES =
    "transparent calc(100% / 3), rgb(255 255 255 / 0.6) 0 calc(100% / 3 + 1px), transparent 0 calc(200% / 3), rgb(255 255 255 / 0.6) 0 calc(200% / 3 + 1px), transparent 0";

/** The lines at the thirds of the frame. */
const THIRDS = `linear-gradient(to right, ${LINES}), linear-gradient(to bottom, ${LINES})`;

/** The styles of a cropper and its parts. */
const styles = style.create({
    cropper: {
        display: "flex",
        flexDirection: "column",
        gap: space[3],
        minWidth: 0,
        width: "100%",
    },
    viewport: {
        position: "relative",
        width: "100%",
        aspectRatio: "4 / 3",
        overflow: "hidden",
        borderRadius: radius[4],
        backgroundColor: color.muted,
        touchAction: "none",
        userSelect: "none",
        // measure from the left, as images are
        direction: "ltr",
        opacity: { default: 1, ":is([data-disabled])": 0.5 },
    },
    grab: {
        cursor: {
            default: "grab",
            ":is([data-dragging])": "grabbing",
            ":is([data-disabled])": "default",
        },
    },
    image: {
        position: "absolute",
        maxWidth: "none",
        pointerEvents: "none",
    },
    imageAt: (left: string, top: string, width: string, height: string, transform: string) => ({
        left,
        top,
        width,
        height,
        transform,
    }),
    waiting: {
        visibility: "hidden",
    },
    selection: {
        position: "absolute",
        boxSizing: "border-box",
        // dim the rest of the image
        boxShadow: SCRIM,
        outlineStyle: "solid",
        outlineWidth: stroke.border,
        outlineColor: { default: "white", ":focus-visible": color.ring },
        outlineOffset: 0,
        cursor: "move",
        forcedColorAdjust: "none",
    },
    focusable: {
        outlineWidth: { default: stroke.border, ":focus-visible": stroke.ring },
    },
    still: {
        cursor: "inherit",
    },
    round: {
        borderRadius: radius.full,
    },
    at: (left: string, top: string, width: string, height: string) => ({
        left,
        top,
        width,
        height,
    }),
    grid: {
        position: "absolute",
        inset: 0,
        overflow: "hidden",
        borderRadius: "inherit",
        pointerEvents: "none",
        opacity: { default: 0, ":is([data-shown])": 1 },
        transitionProperty: "opacity",
        transitionDuration: motion.durationShort,
        transitionTimingFunction: motion.easingStandard,
        backgroundImage: THIRDS,
    },
    handle: {
        position: "absolute",
        width: HANDLE,
        height: HANDLE,
        translate: "-50% -50%",
        display: "flex",
        alignItems: "center",
        justifyContent: "center",
        "::before": {
            content: "''",
            width: space[2],
            height: space[2],
            borderRadius: radius.full,
            borderWidth: stroke.border,
            borderStyle: "solid",
            borderColor: color.border,
            backgroundColor: { default: "white", [media.forcedColors]: "Highlight" },
        },
    },
    zoom: {
        flexGrow: 1,
        minWidth: 0,
    },
});

/** The place of each handle on the frame, and the way it moves. */
const handles = style.create({
    top: { left: "50%", top: 0, cursor: "ns-resize" },
    "top-right": { left: "100%", top: 0, cursor: "nesw-resize" },
    right: { left: "100%", top: "50%", cursor: "ew-resize" },
    "bottom-right": { left: "100%", top: "100%", cursor: "nwse-resize" },
    bottom: { left: "50%", top: "100%", cursor: "ns-resize" },
    "bottom-left": { left: 0, top: "100%", cursor: "nesw-resize" },
    left: { left: 0, top: "50%", cursor: "ew-resize" },
    "top-left": { left: 0, top: 0, cursor: "nwse-resize" },
});

/** The properties of an element of a cropper, the native element's attributes included. */
export type CropperElementProperties<Target extends HTMLElement = HTMLDivElement> = Omit<
    JSX.HTMLAttributes<Target>,
    "class"
> &
    ElementPartProperties;

/** The properties of a cropper. */
export type CropperProperties = CropperElementProperties & CropperOptions;

/** Share the part of an image a person crops with its parts. */
export function Cropper(properties: CropperProperties): JSX.Element {
    const control = new CropperControl(properties);
    const rest = omit(
        properties,
        "value",
        "defaultValue",
        "onValueChange",
        "onValueCommitted",
        "aspectRatio",
        "fixedCropArea",
        "rotation",
        "defaultRotation",
        "onRotationChange",
        "flip",
        "defaultFlip",
        "onFlipChange",
        "onZoomChange",
        "minWidth",
        "minHeight",
        "maxZoom",
        "disabled",
    );

    return (
        <CropperContext value={control}>
            {renderPart("div", "cropper", rest, styles.cropper, {
                role: "group" as const,
                get "data-disabled"() {
                    return properties.disabled === true ? "" : undefined;
                },
            })}
        </CropperContext>
    );
}

/** Render the view of the image, which a drag moves and a wheel or two fingers zoom. */
export function CropperViewport(properties: CropperElementProperties): JSX.Element {
    // follow the size of the view
    const control = useCropper();
    let element: HTMLElement | undefined;
    onSettled(() => {
        if (element === undefined) {
            return undefined;
        }
        const view = element;
        const measure = (): void =>
            control.resize({ width: view.clientWidth, height: view.clientHeight });
        measure();
        const resizes = new ResizeObserver(measure);
        resizes.observe(view);

        // zoom by the wheel in place of the page's scrolling
        const listening = new AbortController();
        view.addEventListener("wheel", (event) => zoomBy(event, control, view), {
            passive: false,
            signal: listening.signal,
        });

        return () => {
            resizes.disconnect();
            listening.abort();
        };
    });

    return renderPart(
        "div",
        "cropper-viewport",
        properties,
        () => [styles.viewport, control.options.fixedCropArea === true && styles.grab],
        {
            ref: (created: HTMLElement) => (element = created),
            get "data-dragging"() {
                return control.isMoving() ? "" : undefined;
            },
            get "data-disabled"() {
                return control.options.disabled === true ? "" : undefined;
            },
            onPointerDown: (event: PointerEvent & { readonly currentTarget: HTMLElement }) => {
                // move the image below a frame that stays, by one finger or two
                if (control.options.fixedCropArea === true) {
                    pan(event, control);
                }
            },
        },
    );
}

/** The properties of the image, the native element's attributes included. */
export type CropperImageProperties = Omit<
    JSX.ImgHTMLAttributes<HTMLImageElement>,
    "class" | "style"
> & {
    /** The StyleX styles applied after the image's styles. */
    readonly xstyle?: style.Styles;
    /** Render another element with its attributes, its own by default. */
    readonly render?: Render;
};

/** Render the image as it stands, turned, mirrored and zoomed. */
export function CropperImage(properties: CropperImageProperties): JSX.Element {
    // put its middle where the turned one's is
    const control = useCropper();
    const placed = () => {
        // place it by its middle
        const image = control.image() ?? { width: 0, height: 0 };
        const turned = control.turned();
        const scale = control.scale();
        const origin = control.origin();

        return styles.imageAt(
            `${String(origin.x + (turned.width * scale) / 2)}px`,
            `${String(origin.y + (turned.height * scale) / 2)}px`,
            `${String(image.width * scale)}px`,
            `${String(image.height * scale)}px`,
            `translate(-50%, -50%) scale(${control.mirror().horizontal ? "-1" : "1"}, ${control.mirror().vertical ? "-1" : "1"}) rotate(${String(control.rotation())}deg)`,
        );
    };

    // leave one without a size of its own, such as some drawings
    const load = (element: HTMLImageElement): void => {
        if (element.naturalWidth > 0 && element.naturalHeight > 0) {
            control.load(element);
        }
    };

    return renderPart(
        "img",
        "cropper-image",
        properties,
        () => [styles.image, control.image() === undefined ? styles.waiting : placed()],
        {
            alt: "",
            draggable: false,
            // take one that loaded before too, such as from the browser's store
            ref: (element: HTMLImageElement) => (element.complete ? load(element) : undefined),
            onLoad: (event) => load(event.currentTarget),
        },
    );
}

/** The properties of the frame around the part. */
export type CropperSelectionProperties = CropperElementProperties & {
    /** The shape of the frame, a rectangle by default, a circle as of a profile picture. */
    readonly cropShape?: "rectangle" | "circle";
};

/** Render the frame around the part, with its thirds and its handles. */
export function CropperSelection(properties: CropperSelectionProperties): JSX.Element {
    // move the part by a drag or the keys
    const control = useCropper();
    const locale = useLocale();
    const rest = omit(properties, "cropShape", "children");
    const isFixed = (): boolean => control.options.fixedCropArea === true;
    const placed = () => {
        const frame = control.frame();

        return styles.at(
            `${String(frame.x)}px`,
            `${String(frame.y)}px`,
            `${String(frame.width)}px`,
            `${String(frame.height)}px`,
        );
    };
    const words = (): string => {
        const { width, height } = control.area();

        return locale.render(
            t`${locale.number(Math.round(width))} by ${locale.number(Math.round(height))} pixels`,
        );
    };

    return (
        <Show when={control.image()}>
            {renderPart(
                "div",
                "cropper-selection",
                rest,
                () => [
                    styles.selection,
                    styles.focusable,
                    isFixed() && styles.still,
                    properties.cropShape === "circle" && styles.round,
                    placed(),
                ],
                {
                    role: "group" as const,
                    get tabindex() {
                        return control.options.disabled === true ? undefined : 0;
                    },
                    get "aria-label"() {
                        return locale.render(t`Crop area`);
                    },
                    get "aria-description"() {
                        return words();
                    },
                    onPointerDown: (
                        event: PointerEvent & { readonly currentTarget: HTMLElement },
                    ) => {
                        // move the part, the image where the frame stays
                        if (!isFixed()) {
                            drag(event, control, (x, y, from) => control.moveBy(x, y, from));
                        }
                    },
                    onKeyDown: (event: KeyboardEvent) => press(event, control),
                    get children() {
                        return contentOr(properties, () => (
                            <>
                                <CropperGrid />
                                <Show when={!isFixed()}>
                                    <For each={POSITIONS}>
                                        {(position) => <CropperHandle position={position} />}
                                    </For>
                                </Show>
                            </>
                        ));
                    },
                },
            )}
        </Show>
    );
}

/** The properties of the thirds of the frame. */
export type CropperGridProperties = CropperElementProperties & {
    /** Whether it is always there, only while something moves by default. */
    readonly keepShown?: boolean;
};

/** Render the thirds of the frame, shown while something moves. */
export function CropperGrid(properties: CropperGridProperties): JSX.Element {
    const control = useCropper();
    const rest = omit(properties, "keepShown");

    return renderPart("div", "cropper-grid", rest, styles.grid, {
        "aria-hidden": "true" as const,
        get "data-shown"() {
            return control.isMoving() || properties.keepShown === true ? "" : undefined;
        },
    });
}

/** The properties of a handle. */
export type CropperHandleProperties = CropperElementProperties & {
    /** The side or corner it moves. */
    readonly position: CropPosition;
};

/** Render a handle that moves a side or a corner of the frame. */
export function CropperHandle(properties: CropperHandleProperties): JSX.Element {
    const control = useCropper();
    const rest = omit(properties, "position");

    return renderPart(
        "div",
        "cropper-handle",
        rest,
        () => [styles.handle, handles[properties.position]],
        {
            "aria-hidden": "true" as const,
            get "data-position"() {
                return properties.position;
            },
            onPointerDown: (event: PointerEvent & { readonly currentTarget: HTMLElement }) => {
                event.stopPropagation();
                drag(event, control, (x, y, from) =>
                    control.resizeBy(properties.position, x, y, from),
                );
            },
        },
    );
}

/** Render a slider that zooms the image. */
export function CropperZoom(
    properties: Omit<SliderProperties, "value" | "min" | "max" | "onValueChange">,
): JSX.Element {
    const control = useCropper();
    const locale = useLocale();

    return (
        <Slider
            aria-label={locale.render(t`Zoom`)}
            step={0.01}
            format={{ style: "percent" }}
            {...properties}
            data-slot="cropper-zoom"
            disabled={properties.disabled === true || control.options.disabled === true}
            min={1}
            max={control.maxZoom()}
            value={[Math.min(control.zoom(), control.maxZoom())]}
            onValueChange={([zoom]) => {
                if (zoom !== undefined) {
                    control.zoomTo(zoom);
                }
            }}
            onValueCommitted={() => control.commit()}
            xstyle={[styles.zoom, properties.xstyle]}
        />
    );
}

/** Render the button that turns the image a quarter. */
export function CropperRotate(
    properties: ButtonProperties & {
        /** The way it turns, to the right by default. */
        readonly direction?: 1 | -1;
    },
): JSX.Element {
    // turn it the way the owner says
    const control = useCropper();
    const locale = useLocale();
    const rest = omit(properties, "direction");

    return (
        <Button
            data-slot="cropper-rotate"
            variant="ghost"
            size="icon"
            aria-label={locale.render(
                properties.direction === -1 ? t`Rotate left` : t`Rotate right`,
            )}
            disabled={control.options.disabled === true}
            {...mergeProperties({ onClick: () => control.rotate(properties.direction) }, rest)}
        >
            {contentOr(properties, () => (
                <Icon
                    icon={
                        properties.direction === -1 ? arrowCounterClockwiseIcon : arrowClockwiseIcon
                    }
                />
            ))}
        </Button>
    );
}

/** Render the button that mirrors the image. */
export function CropperFlip(
    properties: ButtonProperties & {
        /** The axis it mirrors along, from left to right by default. */
        readonly axis?: "horizontal" | "vertical";
    },
): JSX.Element {
    // mirror it the way the owner says
    const control = useCropper();
    const locale = useLocale();
    const rest = omit(properties, "axis");
    const isVertical = (): boolean => properties.axis === "vertical";

    return (
        <Button
            data-slot="cropper-flip"
            variant="ghost"
            size="icon"
            aria-label={locale.render(isVertical() ? t`Flip vertically` : t`Flip horizontally`)}
            disabled={control.options.disabled === true}
            {...mergeProperties({ onClick: () => control.flip(properties.axis) }, rest)}
        >
            {contentOr(properties, () => (
                <Icon icon={isVertical() ? flipVerticalIcon : flipHorizontalIcon} />
            ))}
        </Button>
    );
}

/** Follow a pointer that drags something, in pixels of the image. */
function drag(
    event: PointerEvent & { readonly currentTarget: HTMLElement },
    control: CropperControl,
    follow: (x: number, y: number, from: ReturnType<CropperControl["area"]>) => void,
): void {
    // leave other buttons, and follow until the pointer lets go
    if (event.button !== 0 || control.options.disabled === true) {
        return;
    }
    const { clientX, clientY, pointerId } = event;
    const target: HTMLElement = event.currentTarget;
    event.preventDefault();
    target.setPointerCapture(pointerId);
    const from = control.area();
    const scale = control.scale();
    control.setMoving(true);
    const listening = new AbortController();
    const { signal } = listening;
    const end = (): void => {
        listening.abort();
        control.setMoving(false);
    };
    target.addEventListener(
        "pointermove",
        (step) => follow((step.clientX - clientX) / scale, (step.clientY - clientY) / scale, from),
        { signal },
    );
    target.addEventListener("pointerup", end, { signal });
    target.addEventListener(
        "pointercancel",
        () => {
            follow(0, 0, from);
            end();
        },
        { signal },
    );
}

/** Move the image by a finger or a mouse, and zoom it by two fingers. */
function pan(
    event: PointerEvent & { readonly currentTarget: HTMLElement },
    control: CropperControl,
): void {
    // leave other buttons, and fingers while some are followed
    const view: HTMLElement = event.currentTarget;
    if (event.button !== 0 || control.options.disabled === true || control.isMoving()) {
        return;
    }
    event.preventDefault();
    control.setMoving(true);
    const points = new Map([[event.pointerId, { x: event.clientX, y: event.clientY }]]);
    view.setPointerCapture(event.pointerId);
    const middle = () => {
        // read the middle of the fingers, and how far apart they are
        const [first = { x: 0, y: 0 }, second = first] = [...points.values()];

        return {
            x: (first.x + second.x) / 2,
            y: (first.y + second.y) / 2,
            spread: Math.hypot(first.x - second.x, first.y - second.y),
        };
    };
    let last = middle();
    const listening = new AbortController();
    const { signal } = listening;
    const leave = ({ pointerId }: PointerEvent): void => {
        points.delete(pointerId);
        last = middle();
        if (points.size === 0) {
            listening.abort();
            control.setMoving(false);
        }
    };
    view.addEventListener(
        "pointerdown",
        (added) => {
            // follow a second finger too
            if (points.size === 1) {
                points.set(added.pointerId, { x: added.clientX, y: added.clientY });
                view.setPointerCapture(added.pointerId);
                last = middle();
            }
        },
        { signal },
    );
    view.addEventListener(
        "pointermove",
        (step) => {
            // zoom by how far apart the fingers are, then move by their middle
            if (!points.has(step.pointerId)) {
                return;
            }
            points.set(step.pointerId, { x: step.clientX, y: step.clientY });
            const next = middle();
            if (last.spread > 0 && next.spread > 0) {
                control.zoomTo(
                    (control.zoom() * next.spread) / last.spread,
                    pointOf(next, control, view),
                );
            }
            control.moveBy(
                (last.x - next.x) / control.scale(),
                (last.y - next.y) / control.scale(),
            );
            last = next;
        },
        { signal },
    );
    view.addEventListener("pointerup", leave, { signal });
    view.addEventListener("pointercancel", leave, { signal });
}

/** Zoom by a wheel about the pointer. */
function zoomBy(event: WheelEvent, control: CropperControl, view: HTMLElement): void {
    if (control.options.fixedCropArea === true && control.options.disabled !== true) {
        event.preventDefault();
        const point = pointOf({ x: event.clientX, y: event.clientY }, control, view);
        control.zoomTo(control.zoom() * (1 - event.deltaY * WHEEL_ZOOM), point);
    }
}

/** Read the point of the image below a point of the screen. */
function pointOf(
    point: { readonly x: number; readonly y: number },
    control: CropperControl,
    view: HTMLElement,
) {
    const box = view.getBoundingClientRect();
    const origin = control.origin();

    return {
        x: (point.x - box.left - origin.x) / control.scale(),
        y: (point.y - box.top - origin.y) / control.scale(),
    };
}

/** Move, resize or zoom the part by a key. */
function press(event: KeyboardEvent, control: CropperControl): void {
    // read the way an arrow goes, and how far
    const ways: Readonly<Record<string, readonly [number, number]>> = {
        ArrowLeft: [-1, 0],
        ArrowRight: [1, 0],
        ArrowUp: [0, -1],
        ArrowDown: [0, 1],
    };
    const way = ways[event.key];
    const turned = control.turned();
    const distance = STEP * Math.min(turned.width, turned.height);
    const zoom = event.key === "+" || event.key === "=" ? 1 : event.key === "-" ? -1 : 0;
    if (
        (way === undefined && zoom === 0) ||
        event.defaultPrevented ||
        event.metaKey ||
        event.ctrlKey
    ) {
        return;
    }
    event.preventDefault();

    // resize by Alt, zoom by plus and minus, else move
    const steps = event.shiftKey && way !== undefined ? LARGE_STEP : 1;
    const [x, y] = [(way?.[0] ?? 0) * steps * distance, (way?.[1] ?? 0) * steps * distance];
    if (zoom !== 0) {
        control.zoomTo(control.zoom() + zoom * ZOOM_STEP);
    } else if (event.altKey && control.options.fixedCropArea !== true) {
        control.resizeBy(x === 0 ? "bottom" : "right", x, y);
    } else {
        control.moveBy(x, y);
    }
    control.commit();
}
