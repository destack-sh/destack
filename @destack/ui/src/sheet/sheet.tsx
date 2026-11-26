import { t } from "@destack/locale";
import { field } from "@destack/object";
import { SIDES } from "../position/index.ts";
import * as style from "@destack/style";
import { transition } from "@destack/theme/motion";
import { color, stroke, width } from "@destack/theme/tokens.stylex";
import { type JSX, omit } from "@destack/view";
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
    type DialogContentAttributes,
    type DialogElementProperties,
    type DialogProperties,
    type DialogTriggerProperties,
} from "../dialog/index.ts";
import {
    type ComponentProperties,
    defineComponent,
    type RenderProperties,
} from "@destack/view/declare";

/** The widest a left or right sheet grows. */
const SHEET_WIDTH = width.sheet;

/** The properties people change in a sheet's controls and inspector: its dialog's and its side. */
const SHEET_CONTENT_PROPERTIES = {
    ...DialogContent.properties,
    side: field
        .enum(SIDES)
        .labeled(t.context("label")`Side`)
        .described(t`The edge it slides in from, mirrored right to left.`)
        .default("right"),
};

/** The styles every sheet shares. */
const styles = style.create({
    sheet: {
        display: { default: "none", ":is([open])": "flex" },
        flexDirection: "column",
        maxWidth: "none",
        maxHeight: "none",
        borderRadius: 0,
    },
    footer: {
        marginTop: "auto",
    },
});

/** The placement and border of a sheet on each side. */
const sides = style.create({
    top: {
        top: 0,
        insetInlineEnd: 0,
        bottom: "auto",
        insetInlineStart: 0,
        width: "100%",
        height: "auto",
        borderBottomWidth: stroke.border,
        borderBottomStyle: "solid",
        borderBottomColor: color.border,
    },
    right: {
        top: 0,
        insetInlineEnd: 0,
        bottom: 0,
        insetInlineStart: "auto",
        width: `min(75%, ${SHEET_WIDTH})`,
        height: "100%",
        borderInlineStartWidth: stroke.border,
        borderInlineStartStyle: "solid",
        borderInlineStartColor: color.border,
    },
    bottom: {
        top: "auto",
        insetInlineEnd: 0,
        bottom: 0,
        insetInlineStart: 0,
        width: "100%",
        height: "auto",
        borderTopWidth: stroke.border,
        borderTopStyle: "solid",
        borderTopColor: color.border,
    },
    left: {
        top: 0,
        insetInlineEnd: "auto",
        bottom: 0,
        insetInlineStart: 0,
        width: `min(75%, ${SHEET_WIDTH})`,
        height: "100%",
        borderInlineEndWidth: stroke.border,
        borderInlineEndStyle: "solid",
        borderInlineEndColor: color.border,
    },
});

/** The slide of a sheet entering from each side. */
const enters = style.create({
    top: { transform: { default: "none", "@starting-style": "translateY(-100%)" } },
    right: {
        transform: {
            default: "none",
            "@starting-style": { default: "translateX(100%)", ":dir(rtl)": "translateX(-100%)" },
        },
    },
    bottom: { transform: { default: "none", "@starting-style": "translateY(100%)" } },
    left: {
        transform: {
            default: "none",
            "@starting-style": { default: "translateX(-100%)", ":dir(rtl)": "translateX(100%)" },
        },
    },
});

/** The slide of a sheet leaving to each side. */
const exits = style.create({
    top: { transform: "translateY(-100%)" },
    right: { transform: { default: "translateX(100%)", ":dir(rtl)": "translateX(-100%)" } },
    bottom: { transform: "translateY(100%)" },
    left: { transform: { default: "translateX(-100%)", ":dir(rtl)": "translateX(100%)" } },
});

/** The properties of a sheet's content besides its declared ones: its dialog content's. */
export type SheetContentAttributes = DialogContentAttributes;

/** Hold the open state of a panel that slides in from an edge of the screen. */
export function Sheet<Payload = unknown>(properties: DialogProperties<Payload>): JSX.Element {
    return <Dialog {...properties} />;
}

/** Render a button that opens its sheet. */
export function SheetTrigger<Payload = unknown>(
    properties: DialogTriggerProperties<Payload>,
): JSX.Element {
    return <DialogTrigger data-slot="sheet-trigger" {...properties} />;
}

/** Render a button that closes its sheet. */
export function SheetClose(properties: DialogButtonProperties): JSX.Element {
    return <DialogClose data-slot="sheet-close" {...properties} />;
}

/** Render the sheet as a modal dialog against one edge of the screen. */
export const SheetContent = defineComponent({
    name: "sheetContent",
    properties: SHEET_CONTENT_PROPERTIES,
    render(
        properties: RenderProperties<SheetContentAttributes, typeof SHEET_CONTENT_PROPERTIES>,
    ): JSX.Element {
        // read the dialog and the sheet's side
        const control = useDialog();
        const rest = omit(properties, "side", "xstyle", "style");

        return (
            <DialogContent
                data-slot="sheet-content"
                data-side={properties.side}
                {...rest}
                xstyle={[
                    transition.sheet,
                    styles.sheet,
                    sides[properties.side],
                    control.isOpen() ? enters[properties.side] : exits[properties.side],
                    properties.xstyle,
                ]}
            />
        );
    },
});

/** The properties of a sheet's content, its dialog's included. */
export type SheetContentProperties = ComponentProperties<typeof SheetContent>;

/** Render the top of a sheet with its title and description. */
export function SheetHeader(properties: DialogElementProperties<HTMLDivElement>): JSX.Element {
    return <DialogHeader data-slot="sheet-header" {...properties} />;
}

/** Render the bottom of a sheet with its actions. */
export function SheetFooter(properties: DialogElementProperties<HTMLDivElement>): JSX.Element {
    const rest = omit(properties, "xstyle", "style");

    return (
        <DialogFooter
            data-slot="sheet-footer"
            {...rest}
            xstyle={[styles.footer, properties.xstyle]}
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
