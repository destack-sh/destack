import * as style from "@destack/style";
import { color, radius, size, space } from "@destack/theme/tokens.stylex";
import type { JSX } from "@solidjs/web";
import { createContext, omit, Show, useContext } from "solid-js";
import {
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

/** The share of the viewport a drawer grows to at most, after vaul's drawer that shadcn/ui wraps. */
const DRAWER_EXTENT = "80vh";

/** The edge the nearest drawer slides in from. */
const DrawerContext = createContext<SheetSide>("bottom");

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
});

/** The rounded inner corners of a drawer against each edge. */
const corners = style.create({
    top: { borderBottomLeftRadius: radius[5], borderBottomRightRadius: radius[5] },
    right: { borderTopLeftRadius: radius[5], borderBottomLeftRadius: radius[5] },
    bottom: { borderTopLeftRadius: radius[5], borderTopRightRadius: radius[5] },
    left: { borderTopRightRadius: radius[5], borderBottomRightRadius: radius[5] },
});

/** The properties of a drawer root. */
export interface DrawerProperties extends DialogProperties {
    /** The edge the drawer slides in from, bottom by default. */
    readonly direction?: SheetSide;
}

/** Hold the open state of a drawer that slides in from an edge with a handle. */
export function Drawer(properties: DrawerProperties): JSX.Element {
    const rest = omit(properties, "direction");

    return (
        <DrawerContext value={properties.direction ?? "bottom"}>
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

/** Render the drawer against its edge, with a handle when it rises from the bottom. */
export function DrawerContent(properties: DialogContentProperties): JSX.Element {
    const direction = useContext(DrawerContext);
    const rest = omit(properties, "style", "children");

    return (
        <SheetContent
            data-slot="drawer-content"
            side={direction}
            showCloseButton={false}
            {...rest}
            style={[styles.drawer, corners[direction], properties.style]}
        >
            <Show when={direction === "bottom"}>
                <div data-slot="drawer-handle" aria-hidden="true" {...style.attrs(styles.handle)} />
            </Show>
            {properties.children}
        </SheetContent>
    );
}

/** Render the top of a drawer that holds its title and description. */
export function DrawerHeader(properties: DialogElementProperties<HTMLDivElement>): JSX.Element {
    const rest = omit(properties, "style");

    return (
        <SheetHeader
            data-slot="drawer-header"
            {...rest}
            style={[styles.header, properties.style]}
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
