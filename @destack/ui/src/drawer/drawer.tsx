import * as style from "@destack/style";
import { color, motion, radius, size, space } from "@destack/theme/tokens.stylex";
import {
    type Accessor,
    createContext,
    createControllableSignal,
    type JSX,
    omit,
    Show,
    useContext,
} from "@destack/view";
import {
    useDialog,
    type DialogButtonProperties,
    type DialogContentProperties,
    type DialogElementProperties,
    type DialogProperties,
} from "../dialog/index.ts";
import {
    Sheet,
    SheetClose,
    SheetContent,
    SheetDescription,
    SheetFooter,
    SheetHeader,
    SheetTitle,
    SheetTrigger,
    type SheetSide,
} from "../sheet/index.ts";
import {
    isDismissal,
    Swipe,
    type SwipeDirection,
    type SwipeRelease,
    swipeStyle,
} from "../swipe/index.ts";

/** The share of the viewport a drawer grows to at most. */
const DRAWER_EXTENT = "80vh";

/** The time a released drag's speed carries it on, in milliseconds, as touch screens decelerate scrolls at 0.998 per millisecond. */
const PROJECTION_TIME = 0.998 / (1 - 0.998);

/** The share of the smallest snap point a drag passes below it to close the drawer. */
const CLOSE_SHARE = 0.25;

/** The distance a drag passes below the smallest snap point to close the drawer however small it is, in pixels. */
const CLOSE_DISTANCE = 48;

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
        backgroundColor: color.muted,
    },
    header: {
        textAlign: "center",
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
    height: (length: string) => ({ height: length }),
    width: (length: string) => ({ width: length }),
});

/** The direction a swipe closes a drawer against each edge. */
const SWIPES: Readonly<Record<SheetSide, SwipeDirection>> = {
    top: "up",
    right: "right",
    bottom: "down",
    left: "left",
};

/** The rounded inner corners of a drawer against each edge. */
const corners = style.create({
    top: { borderBottomLeftRadius: radius[5], borderBottomRightRadius: radius[5] },
    right: { borderTopLeftRadius: radius[5], borderBottomLeftRadius: radius[5] },
    bottom: { borderTopLeftRadius: radius[5], borderTopRightRadius: radius[5] },
    left: { borderTopRightRadius: radius[5], borderBottomRightRadius: radius[5] },
});

/** A length a drawer snaps to: a fraction of the viewport from 0 to 1, or CSS pixels such as "320px". */
export type DrawerSnapPoint = number | string;

/** The properties of a drawer root. */
export interface DrawerProperties extends DialogProperties {
    /** The edge the drawer slides in from, bottom by default. */
    readonly direction?: SheetSide;
    /** The lengths a drag settles the drawer at, from the smallest to the largest. */
    readonly snapPoints?: readonly DrawerSnapPoint[];
    /** The snap point the drawer rests at, which makes it controlled. */
    readonly activeSnapPoint?: DrawerSnapPoint;
    /** The snap point the drawer rests at first when uncontrolled, the largest by default. */
    readonly defaultSnapPoint?: DrawerSnapPoint;
    /** Handle a drag settling the drawer at another snap point. */
    readonly onSnapPointChange?: (point: DrawerSnapPoint) => void;
}

/** The edge and snap points of a drawer, which its content reads. */
export class DrawerControl {
    /** The edge the drawer slides in from. */
    readonly direction: Accessor<SheetSide>;
    /** The lengths a drag settles the drawer at, from the smallest to the largest, none without snapping. */
    readonly snapPoints: Accessor<readonly DrawerSnapPoint[]>;
    /** The snap point the drawer rests at, undefined without snap points. */
    readonly activeSnapPoint: Accessor<DrawerSnapPoint | undefined>;
    /** Replace the snap point and tell the change handler. */
    readonly #setActive: (point: DrawerSnapPoint | undefined) => void;

    /** Create the state of a drawer root, resting at the largest snap point unless told otherwise. */
    constructor(properties: DrawerProperties) {
        // follow the controlled snap point, else the drawer's own
        const [active, setActive] = createControllableSignal<DrawerSnapPoint | undefined>({
            isControlled: () => properties.activeSnapPoint !== undefined,
            value: () => properties.activeSnapPoint,
            defaultValue: properties.defaultSnapPoint,
            onChange: (point) => {
                if (point !== undefined) {
                    properties.onSnapPointChange?.(point);
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

    /** Report whether the drawer's extent runs along the vertical axis, as for top and bottom drawers. */
    isVertical(): boolean {
        return this.direction() === "top" || this.direction() === "bottom";
    }

    /** Settle a released drag at the snap point nearest its projected extent, closing the drawer past the smallest or on a dismissing swipe without snap points. */
    settle(release: SwipeRelease, viewport: number, close: () => void): void {
        // close on a far swipe or a flick without snap points
        const point = this.activeSnapPoint();
        if (point === undefined) {
            if (isDismissal(release)) {
                close();
            }

            return;
        }

        // project the extent the drag's speed carries the drawer to
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

/** Hold the open state of a drawer that slides in from an edge with a handle, resting at its snap points. */
export function Drawer(properties: DrawerProperties): JSX.Element {
    const rest = omit(
        properties,
        "direction",
        "snapPoints",
        "activeSnapPoint",
        "defaultSnapPoint",
        "onSnapPointChange",
    );

    return (
        <DrawerContext value={new DrawerControl(properties)}>
            <Sheet {...rest} />
        </DrawerContext>
    );
}

/** Render a button that opens its drawer. */
export function DrawerTrigger(properties: DialogButtonProperties): JSX.Element {
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

/** Render the drawer against its edge, with a handle when it rises from the bottom, which a drag toward its edge closes or settles at a snap point. */
export function DrawerContent(properties: DrawerContentProperties): JSX.Element {
    // close the drawer on a swipe toward its edge from unscrolled content, or settle it at a snap point
    const drawer = useDrawer();
    const dialog = useDialog();
    const rest = omit(properties, "showHandle", "xstyle", "style", "children");
    const swipe = new Swipe(
        () => SWIPES[drawer.direction()],
        (release) => drawer.settle(release, viewportOf(drawer), () => dialog.close()),
        (element) => element.scrollTop === 0,
    );

    return (
        <SheetContent
            data-slot="drawer-content"
            data-swiping={swipe.offset() === undefined ? undefined : ""}
            data-snap-point={drawer.activeIndex()}
            side={drawer.direction()}
            showCloseButton={false}
            {...rest}
            onPointerDown={(event) => swipe.start(event)}
            onPointerMove={(event) => swipe.follow(event)}
            onPointerUp={() => swipe.release()}
            onPointerCancel={() => swipe.cancel()}
            xstyle={[
                styles.drawer,
                corners[drawer.direction()],
                extentStyle(drawer, swipe),
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
    const rest = omit(properties, "xstyle", "style");

    return (
        <div
            data-slot="drawer-handle"
            aria-hidden="true"
            {...rest}
            {...style.attributes([styles.handle, properties.xstyle], properties.style)}
        />
    );
}

/** Render the top of a drawer that holds its title and description. */
export function DrawerHeader(properties: DialogElementProperties<HTMLDivElement>): JSX.Element {
    const rest = omit(properties, "xstyle", "style");

    return (
        <SheetHeader
            data-slot="drawer-header"
            {...rest}
            xstyle={[styles.header, properties.xstyle]}
        />
    );
}

/** Render the bottom of a drawer that holds its actions. */
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

/** Return the StyleX styles that size a drawer to its active snap point or to a running drag, else move it with a swipe. */
function extentStyle(drawer: DrawerControl, swipe: Swipe): style.Styles {
    // move a drawer without snap points with the swipe
    const point = drawer.activeSnapPoint();
    const offset = swipe.offset();
    const extent = drawer.isVertical() ? styles.height : styles.width;
    if (point === undefined) {
        return swipeStyle(swipe.translate());
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

/** Read the viewport's extent along a drawer's axis, in pixels. */
function viewportOf(drawer: DrawerControl): number {
    return drawer.isVertical() ? window.innerHeight : window.innerWidth;
}

/** Read the pixels a snap point spans in a viewport, refusing lengths other than fractions and pixels. */
function extentOf(point: DrawerSnapPoint, viewport: number): number {
    // scale a fraction of the viewport
    if (typeof point === "number" && point >= 0 && point <= 1) {
        return point * viewport;
    }

    // read a pixel length
    const pixels = typeof point === "string" ? PIXELS.exec(point)?.[1] : undefined;
    if (pixels === undefined) {
        throw new TypeError(
            `a drawer snap point is a fraction from 0 to 1 or a pixel length: ${String(point)}`,
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
