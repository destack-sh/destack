import * as style from "@destack/style";
import { transition } from "@destack/theme/motion";
import { color, motion, radius, shadow, size, space } from "@destack/theme/tokens.stylex";
import {
    type Accessor,
    createContext,
    createControllableSignal,
    createSignal,
    type JSX,
    omit,
    onSettled,
    Show,
    useContext,
    useEnvironment,
} from "@destack/view";
import {
    useDialog,
    type DialogButtonProperties,
    type DialogContentProperties,
    type DialogElementProperties,
    type DialogProperties,
    type DialogTriggerProperties,
} from "../dialog/index.ts";
import { scrim } from "../dialog/scrim.stylex.ts";
import { OpenChange } from "../layer/index.ts";
import {
    Sheet,
    SheetClose,
    SheetContent,
    SheetDescription,
    SheetFooter,
    SheetHeader,
    SheetTitle,
    SheetTrigger,
} from "../sheet/index.ts";
import { Swipe, type SwipeEvent } from "../interaction/index.ts";
import type { Side } from "../position/index.ts";
import { mergeProperties, renderPart } from "../part/index.ts";

/** The share of the viewport a drawer grows to at most. */
const DRAWER_EXTENT = "80dvh";

/** The time a released drag's speed moves it on, in milliseconds. */
const PROJECTION_TIME = 0.998 / (1 - 0.998);

/** The share of the smallest snap point a drag passes below it to close the drawer. */
const CLOSE_SHARE = 0.25;

/** The distance a drag passes below the smallest snap point to close the drawer, in pixels. */
const CLOSE_DISTANCE = 48;

/** The speed of a flick that closes a drawer, in pixels per millisecond, as Vaul's. */
const FLICK_SPEED = 0.4;

/** The room a drawer over a phone's keyboard leaves above it, in pixels, as Vaul's. */
const TOP_OFFSET = 26;

/** A CSS pixel length, as snap points write them. */
const PIXELS = /^(\d+(?:\.\d+)?)px$/u;

/** The drawer of the nearest drawer root, null outside one. */
const DrawerContext = createContext<DrawerControl | null>(null);

/** The styles of a drawer and its handle. */
const styles = style.create({
    drawer: {
        maxHeight: DRAWER_EXTENT,
    },
    handle: {
        width: size[4],
        height: space[1],
        marginInline: "auto",
        flexShrink: 0,
        borderRadius: radius.full,
        backgroundColor: color.border,
    },
    snapping: {
        maxHeight: "none",
        maxWidth: "none",
        transitionProperty: "transform, height, width, display, overlay",
        transitionDuration: motion.durationMedium,
        transitionTimingFunction: motion.easingEmphasised,
    },
    dragging: {
        transitionDuration: "0s",
        userSelect: "none",
    },
    fade: (share: string) => ({
        [scrim.share]: share,
        "::backdrop": { transitionDuration: "0s" },
    }),
    scrim: {
        "::backdrop": { transitionDuration: motion.durationLong },
    },
    fill: (x: string, y: string) => ({
        // cover the gap a drawer leaves at its edge as it gives way
        boxShadow: `${x} ${y} 0 0 ${color.popover}, ${shadow.overlay}`,
    }),
    above: (inset: string) => ({
        bottom: inset,
        maxHeight: `calc(100dvh - ${inset} - ${String(TOP_OFFSET)}px)`,
    }),
    height: (length: string) => ({ height: length }),
    width: (length: string) => ({ width: length }),
});

/** The rounded inner corners of a drawer against each edge. */
const corners = style.create({
    top: { borderBottomLeftRadius: radius[5], borderBottomRightRadius: radius[5] },
    right: { borderTopLeftRadius: radius[5], borderBottomLeftRadius: radius[5] },
    bottom: { borderTopLeftRadius: radius[5], borderTopRightRadius: radius[5] },
    left: { borderTopRightRadius: radius[5], borderBottomRightRadius: radius[5] },
});

/** The border of a drawer against each edge, hidden over the gap it leaves there. */
const seams = style.create({
    top: { borderTopColor: color.popover },
    right: { borderRightColor: color.popover },
    bottom: { borderBottomColor: color.popover },
    left: { borderLeftColor: color.popover },
});

/** A length a drawer snaps to. */
export type DrawerSnapPoint = number | string;

/** Where a drawer opens from and rests. */
export interface DrawerOptions {
    /** The edge the drawer slides in from, bottom by default. */
    readonly direction?: Side;
    /** The lengths a drag settles the drawer at, from the smallest to the largest. */
    readonly snapPoints?: readonly DrawerSnapPoint[];
    /** The snap point the drawer rests at, which makes it controlled. */
    readonly activeSnapPoint?: DrawerSnapPoint;
    /** The snap point the drawer rests at first when uncontrolled, the largest by default. */
    readonly defaultActiveSnapPoint?: DrawerSnapPoint;
    /** Handle a drag settling the drawer at another snap point. */
    readonly onActiveSnapPointChange?: (point: DrawerSnapPoint) => void;
}

/** The properties of a drawer root. */
export interface DrawerProperties<Payload = unknown>
    extends DialogProperties<Payload>, DrawerOptions {}

/** The edge and snap points of a drawer, which its content reads. */
export class DrawerControl {
    /** The edge the drawer slides in from. */
    readonly direction: Accessor<Side>;
    /** The lengths a drag settles the drawer at, from the smallest to the largest. */
    readonly snapPoints: Accessor<readonly DrawerSnapPoint[]>;
    /** The snap point the drawer rests at, undefined without snap points. */
    readonly activeSnapPoint: Accessor<DrawerSnapPoint | undefined>;
    /** Replace the snap point and tell the change handler. */
    readonly #setActive: (point: DrawerSnapPoint | undefined) => void;

    /** Create the state of a drawer root. */
    constructor(properties: DrawerOptions) {
        // follow the controlled snap point, else the drawer's own
        const [active, setActive] = createControllableSignal<DrawerSnapPoint | undefined>({
            isControlled: () => properties.activeSnapPoint !== undefined,
            value: () => properties.activeSnapPoint,
            defaultValue: properties.defaultActiveSnapPoint,
            onChange: (point) => {
                if (point !== undefined) {
                    properties.onActiveSnapPointChange?.(point);
                }
            },
        });
        this.direction = () => properties.direction ?? "bottom";
        this.snapPoints = () => properties.snapPoints ?? [];
        this.activeSnapPoint = () => active() ?? properties.snapPoints?.at(-1);
        this.#setActive = setActive;
    }

    /** Read the position of the active snap point among the snap points, undefined without one. */
    activeIndex(): number | undefined {
        const point = this.activeSnapPoint();

        return point === undefined ? undefined : this.snapPoints().indexOf(point);
    }

    /** Report whether the drawer's extent runs along the vertical axis. */
    isVertical(): boolean {
        return this.direction() === "top" || this.direction() === "bottom";
    }

    /** Settle a released drag at the snap point nearest its projected extent. */
    settle(release: SwipeEvent, viewport: number, close: () => void): void {
        // spring back from a swipe the browser took over
        const point = this.activeSnapPoint();
        if (release.isCancelled) {
            return;
        }
        // close on a far swipe or a flick without snap points
        else if (point === undefined) {
            if (release.isDismissal) {
                close();
            }

            return;
        }

        // project how far the drag's speed moves the drawer
        const points = this.snapPoints();
        const extents = points.map((entry) => extentOf(entry, viewport));
        const projected =
            extentOf(point, viewport) - release.travel - release.speed * PROJECTION_TIME;
        const smallest = Math.min(...extents);

        // close below the smallest, else rest at the nearest snap point
        if (projected < smallest - Math.max(smallest * CLOSE_SHARE, CLOSE_DISTANCE)) {
            close();
        } else {
            const distances = extents.map((extent) => Math.abs(extent - projected));
            const nearest = points[distances.indexOf(Math.min(...distances))];
            if (nearest !== point) {
                this.#setActive(nearest);
            }
        }
    }
}

/** Read the drawer of the nearest drawer root, refusing elements outside one. */
export function useDrawer(): DrawerControl {
    const drawer = useContext(DrawerContext);
    if (drawer === null) {
        throw new TypeError("drawer elements need a drawer root around them");
    }

    return drawer;
}

/** Keep the open state of a drawer that slides in from an edge with a handle. */
export function Drawer<Payload = unknown>(properties: DrawerProperties<Payload>): JSX.Element {
    const rest = omit(
        properties,
        "direction",
        "snapPoints",
        "activeSnapPoint",
        "defaultActiveSnapPoint",
        "onActiveSnapPointChange",
    );

    return (
        <DrawerContext value={new DrawerControl(properties)}>
            <Sheet {...rest} />
        </DrawerContext>
    );
}

/** Render a button that opens its drawer. */
export function DrawerTrigger<Payload = unknown>(
    properties: DialogTriggerProperties<Payload>,
): JSX.Element {
    return <SheetTrigger data-slot="drawer-trigger" {...properties} />;
}

/** Render a button that closes its drawer. */
export function DrawerClose(properties: DialogButtonProperties): JSX.Element {
    return <SheetClose data-slot="drawer-close" {...properties} />;
}

/** The properties of a drawer's content, the dialog content's included. */
export interface DrawerContentProperties extends DialogContentProperties {
    /** Whether to show a handle at the top edge, true when the drawer rises from the bottom. */
    readonly showHandle?: boolean;
}

/** Render the drawer against its edge, with a handle when it rises from the bottom. */
export function DrawerContent(properties: DrawerContentProperties): JSX.Element {
    // close the drawer on a swipe toward its edge from unscrolled content, or settle it at a snap point
    const drawer = useDrawer();
    const dialog = useDialog();
    const rest = omit(properties, "showHandle", "xstyle", "style", "children");
    const swipe = new Swipe(
        {
            side: () => drawer.direction(),
            onSwipeEnd: (event) =>
                drawer.settle(event, viewportOf(drawer), () =>
                    dialog.close(new OpenChange("swipe")),
                ),
            canStart: (event) => canSwipe(drawer, event),
            flickSpeed: FLICK_SPEED,
            isElastic: true,
        },
        useEnvironment().clock,
    );

    // stay above a phone's keyboard
    const keyboard = createKeyboardInset();
    let content: HTMLElement | undefined;
    const measure = (element: HTMLElement) => (content = element);

    return (
        <SheetContent
            data-slot="drawer-content"
            data-snap-point={drawer.activeIndex()}
            side={drawer.direction()}
            showCloseButton={false}
            {...mergeProperties({ ref: measure }, rest)}
            {...swipe.attributes()}
            xstyle={[
                styles.drawer,
                transition.drawer,
                styles.scrim,
                corners[drawer.direction()],
                extentStyle(drawer, swipe, content),
                drawer.direction() === "bottom" &&
                    keyboard() > 0 &&
                    styles.above(`${String(keyboard())}px`),
                properties.xstyle,
            ]}
        >
            <Show when={properties.showHandle ?? drawer.direction() === "bottom"}>
                <DrawerHandle />
            </Show>
            {properties.children}
        </SheetContent>
    );
}

/** Render the bar a drawer shows at its top edge as the place to drag it by. */
export function DrawerHandle(properties: DialogElementProperties<HTMLDivElement>): JSX.Element {
    return renderPart("div", "drawer-handle", properties, styles.handle, { "aria-hidden": "true" });
}

/** Render the top of a drawer with its title and description. */
export function DrawerHeader(properties: DialogElementProperties<HTMLDivElement>): JSX.Element {
    return <SheetHeader data-slot="drawer-header" {...properties} />;
}

/** Render the bottom of a drawer with its actions. */
export function DrawerFooter(properties: DialogElementProperties<HTMLDivElement>): JSX.Element {
    return <SheetFooter data-slot="drawer-footer" {...properties} />;
}

/** Render the title that names its drawer. */
export function DrawerTitle(properties: DialogElementProperties<HTMLHeadingElement>): JSX.Element {
    return <SheetTitle data-slot="drawer-title" {...properties} />;
}

/** Render the description that describes its drawer. */
export function DrawerDescription(
    properties: DialogElementProperties<HTMLParagraphElement>,
): JSX.Element {
    return <SheetDescription data-slot="drawer-description" {...properties} />;
}

/** Return the StyleX styles that size a drawer to its active snap point or a running drag. */
function extentStyle(
    drawer: DrawerControl,
    swipe: Swipe,
    content: HTMLElement | undefined,
): style.Styles {
    // move a drawer without snap points with the swipe
    const point = drawer.activeSnapPoint();
    const offset = swipe.offset();
    const extent = drawer.isVertical() ? styles.height : styles.width;
    if (point === undefined) {
        return [swipe.style(), dragStyle(drawer, swipe, content)];
    }
    // rest at the active snap point
    else if (offset === undefined) {
        return [styles.snapping, extent(lengthOf(point, drawer.isVertical()))];
    }

    // follow the drag between none and the largest snap point
    const viewport = viewportOf(drawer);
    const largest = Math.max(...drawer.snapPoints().map((entry) => extentOf(entry, viewport)));
    const dragged = Math.min(Math.max(extentOf(point, viewport) - offset, 0), largest);

    return [styles.snapping, styles.dragging, extent(`${String(dragged)}px`)];
}

/** Return the StyleX styles of a drawer's backdrop and edge while a drag moves it. */
function dragStyle(
    drawer: DrawerControl,
    swipe: Swipe,
    content: HTMLElement | undefined,
): style.Styles {
    // rest without a drag
    const offset = swipe.offset();
    if (offset === undefined || content === undefined) {
        return null;
    }
    // fade the backdrop with the part of the drawer dragged off
    else if (offset > 0) {
        const extent = drawer.isVertical() ? content.offsetHeight : content.offsetWidth;

        return styles.fade(String(Math.max(0, 1 - offset / extent)));
    }

    // fill the gap at the edge the drawer gives way from
    const direction = drawer.direction();
    const gap = swipe.resisted() * (direction === "bottom" || direction === "right" ? 1 : -1);
    const length = `${String(gap)}px`;

    return [
        seams[direction],
        drawer.isVertical() ? styles.fill("0px", length) : styles.fill(length, "0px"),
    ];
}

/** Follow the height of a phone's keyboard over the page, in pixels, zero without one. */
function createKeyboardInset(): Accessor<number> {
    const [inset, setInset] = createSignal(0, { ownedWrite: true });
    onSettled(() => {
        // measure what the keyboard covers, leaving a zoomed page alone
        const viewport = window.visualViewport ?? undefined;
        if (viewport === undefined) {
            return;
        }
        const measure = () => {
            const covered = window.innerHeight - viewport.height - viewport.offsetTop;
            setInset(viewport.scale === 1 ? Math.max(0, Math.round(covered)) : 0);
        };
        measure();
        viewport.addEventListener("resize", measure);
        viewport.addEventListener("scroll", measure);

        return () => {
            viewport.removeEventListener("resize", measure);
            viewport.removeEventListener("scroll", measure);
        };
    });

    return inset;
}

/** Report whether a press may swipe a drawer. */
function canSwipe(drawer: DrawerControl, event: PointerEvent): boolean {
    // swipe a drawer against a left or right edge from anywhere
    const content = event.currentTarget;
    const target = event.target;
    if (!drawer.isVertical() || !(content instanceof Element) || !(target instanceof Element)) {
        return !drawer.isVertical();
    }

    return (
        content.scrollHeight <= content.clientHeight ||
        target.closest("[data-slot=drawer-handle]") !== null
    );
}

/** Read the viewport's extent along a drawer's axis, in pixels. */
function viewportOf(drawer: DrawerControl): number {
    return drawer.isVertical() ? window.innerHeight : window.innerWidth;
}

/** Read the pixels a snap point spans in a viewport. */
function extentOf(point: DrawerSnapPoint, viewport: number): number {
    // scale a fraction of the viewport
    if (typeof point === "number" && point >= 0 && point <= 1) {
        return point * viewport;
    }

    // read a pixel length
    const pixels = typeof point === "string" ? PIXELS.exec(point)?.[1] : undefined;
    if (pixels === undefined) {
        throw new TypeError(
            `a drawer snap point is a fraction from 0 to 1 or a pixel length: '${String(point)}'`,
        );
    }

    return Number(pixels);
}

/** Write a snap point as the CSS length of a drawer's extent along its axis. */
function lengthOf(point: DrawerSnapPoint, isVertical: boolean): string {
    return typeof point === "number"
        ? `${String(point * 100)}${isVertical ? "dvh" : "dvw"}`
        : point;
}
