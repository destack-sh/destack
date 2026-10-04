import * as style from "@destack/style";
import { color, motion, stroke } from "@destack/theme/tokens.stylex";
import type { JSX } from "@solidjs/web";
import { merge, omit } from "solid-js";
import {
    Dialog,
    DialogClose,
    DialogContent,
    DialogDescription,
    DialogFooter,
    DialogHeader,
    DialogTitle,
    DialogTrigger,
    useDialog,
    type DialogButtonProperties,
    type DialogContentProperties,
    type DialogElementProperties,
    type DialogProperties,
} from "../dialog/index.ts";

/** The widest a left or right sheet grows, Tailwind's sm width that shadcn/ui's sheet stops at. */
const SHEET_WIDTH = "24rem";

/** The side of a sheet that sets none. */
const DEFAULTS: Required<Pick<SheetContentProperties, "side">> = { side: "right" };

/** The styles every sheet shares. */
const styles = style.create({
    sheet: {
        display: { default: "none", ":modal": "flex" },
        flexDirection: "column",
        margin: 0,
        maxWidth: "none",
        maxHeight: "none",
        borderWidth: 0,
        borderRadius: 0,
        transitionProperty: "transform, display, overlay",
        transitionDuration: motion.durationMedium,
        transitionTimingFunction: motion.easingEmphasised,
        transitionBehavior: "allow-discrete",
    },
    footer: {
        marginTop: "auto",
    },
});

/** The placement and border of a sheet on each side. */
const sides = style.create({
    top: {
        inset: "0 0 auto 0",
        width: "100%",
        height: "auto",
        borderBottomWidth: stroke.border,
        borderBottomStyle: "solid",
        borderBottomColor: color.border,
    },
    right: {
        inset: "0 0 0 auto",
        width: `min(75%, ${SHEET_WIDTH})`,
        height: "100%",
        borderLeftWidth: stroke.border,
        borderLeftStyle: "solid",
        borderLeftColor: color.border,
    },
    bottom: {
        inset: "auto 0 0 0",
        width: "100%",
        height: "auto",
        borderTopWidth: stroke.border,
        borderTopStyle: "solid",
        borderTopColor: color.border,
    },
    left: {
        inset: "0 auto 0 0",
        width: `min(75%, ${SHEET_WIDTH})`,
        height: "100%",
        borderRightWidth: stroke.border,
        borderRightStyle: "solid",
        borderRightColor: color.border,
    },
});

/** The slide of a sheet entering from each side. */
const enters = style.create({
    top: { opacity: 1, transform: { default: "none", "@starting-style": "translateY(-100%)" } },
    right: { opacity: 1, transform: { default: "none", "@starting-style": "translateX(100%)" } },
    bottom: { opacity: 1, transform: { default: "none", "@starting-style": "translateY(100%)" } },
    left: { opacity: 1, transform: { default: "none", "@starting-style": "translateX(-100%)" } },
});

/** The slide of a sheet leaving to each side. */
const exits = style.create({
    top: { opacity: 1, transform: "translateY(-100%)" },
    right: { opacity: 1, transform: "translateX(100%)" },
    bottom: { opacity: 1, transform: "translateY(100%)" },
    left: { opacity: 1, transform: "translateX(-100%)" },
});

/** The edge of the screen a sheet slides in from. */
export type SheetSide = "top" | "right" | "bottom" | "left";

/** The properties of a sheet's content. */
export interface SheetContentProperties extends DialogContentProperties {
    /** The edge it slides in from, right by default. */
    readonly side?: SheetSide;
}

/** Hold the open state of a panel that slides in from an edge of the screen. */
export function Sheet(properties: DialogProperties): JSX.Element {
    return <Dialog {...properties} />;
}

/** Render a button that opens its sheet. */
export function SheetTrigger(properties: DialogButtonProperties): JSX.Element {
    return <DialogTrigger data-slot="sheet-trigger" {...properties} />;
}

/** Render a button that closes its sheet. */
export function SheetClose(properties: DialogButtonProperties): JSX.Element {
    return <DialogClose data-slot="sheet-close" {...properties} />;
}

/** Render the sheet as a modal dialog against one edge of the screen. */
export function SheetContent(properties: SheetContentProperties): JSX.Element {
    // read the dialog and the sheet's side
    const control = useDialog();
    const sheet = merge(DEFAULTS, properties);
    const rest = omit(sheet, "side", "style");

    return (
        <DialogContent
            data-slot="sheet-content"
            data-side={sheet.side}
            {...rest}
            style={[
                styles.sheet,
                sides[sheet.side],
                control.isOpen() ? enters[sheet.side] : exits[sheet.side],
                sheet.style,
            ]}
        />
    );
}

/** Render the top of a sheet that holds its title and description. */
export function SheetHeader(properties: DialogElementProperties<HTMLDivElement>): JSX.Element {
    return <DialogHeader data-slot="sheet-header" {...properties} />;
}

/** Render the bottom of a sheet that holds its actions. */
export function SheetFooter(properties: DialogElementProperties<HTMLDivElement>): JSX.Element {
    const rest = omit(properties, "style");

    return (
        <DialogFooter
            data-slot="sheet-footer"
            {...rest}
            style={[styles.footer, properties.style]}
        />
    );
}

/** Render the title that names its sheet. */
export function SheetTitle(properties: DialogElementProperties<HTMLHeadingElement>): JSX.Element {
    return <DialogTitle data-slot="sheet-title" {...properties} />;
}

/** Render the description that describes its sheet. */
export function SheetDescription(
    properties: DialogElementProperties<HTMLParagraphElement>,
): JSX.Element {
    return <DialogDescription data-slot="sheet-description" {...properties} />;
}
