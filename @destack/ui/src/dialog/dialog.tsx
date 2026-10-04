import { Icon } from "@destack/icon";
import { t } from "@destack/locale";
import * as style from "@destack/style";
import { transition } from "@destack/theme/motion";
import {
    color,
    radius,
    shadow,
    space,
    stroke,
    surface,
    weight,
} from "@destack/theme/tokens.stylex";
import { useLocale } from "@destack/locale/solid";
import { text } from "@destack/theme/text";
import type { JSX } from "@solidjs/web";
import {
    createContext,
    createEffect,
    createSignal,
    createUniqueId,
    omit,
    onCleanup,
    Show,
    useContext,
    type Accessor,
    type Setter,
} from "solid-js";
import { Button, type ButtonProperties } from "../button/index.ts";
import { TopLayer } from "../layer/index.ts";

/** The widest a dialog grows, Tailwind's lg width that shadcn/ui's dialog stops at. */
const DIALOG_WIDTH = "32rem";

/** The styles of a dialog and its elements. */
const styles = style.create({
    content: {
        display: { default: "none", ":modal": "grid" },
        gap: space[4],
        width: `min(100% - 2 * ${space[4]}, ${DIALOG_WIDTH})`,
        maxWidth: "none",
        padding: space[5],
        borderStyle: "solid",
        borderWidth: stroke.border,
        borderColor: color.border,
        borderRadius: radius[4],
        backgroundColor: surface.overlay,
        color: color.foreground,
        boxShadow: shadow.overlay,
        "::backdrop": {
            backgroundColor: color.scrim,
        },
    },
    close: {
        position: "absolute",
        top: space[3],
        right: space[3],
    },
    header: {
        display: "flex",
        flexDirection: "column",
        gap: space[2],
        textAlign: "start",
    },
    footer: {
        display: "flex",
        flexWrap: "wrap",
        justifyContent: "flex-end",
        gap: space[2],
    },
    title: {
        margin: 0,
        fontWeight: weight.semibold,
        lineHeight: 1,
    },
    description: {
        margin: 0,
        color: color.mutedForeground,
    },
});

/** The dialog of the nearest dialog root, null outside one. */
const DialogContext = createContext<DialogControl | null>(null);

/** The open state and ids of a dialog, which its trigger, content and text share. */
export class DialogControl {
    /** The id of the dialog element. */
    readonly id: string;
    /** The id of the dialog's title. */
    readonly titleId: string;
    /** The id of the dialog's description. */
    readonly descriptionId: string;
    /** Whether the dialog is open. */
    readonly isOpen: Accessor<boolean>;
    /** Whether a description is on screen. */
    readonly isDescribed: Accessor<boolean>;
    /** The properties of the dialog root, read for its controlled state and change handler. */
    readonly #properties: DialogProperties;
    /** Replace the uncontrolled open state. */
    readonly #setOpen: Setter<boolean>;
    /** Replace whether a description is on screen. */
    readonly #setDescribed: Setter<boolean>;

    /** Create the state of a dialog root, open when its properties ask for it. */
    constructor(properties: DialogProperties) {
        // follow the controlled state, else the dialog's own
        const [isOpen, setOpen] = createSignal(properties.defaultOpen === true);
        const [isDescribed, setDescribed] = createSignal(false, { ownedWrite: true });
        this.id = createUniqueId();
        this.titleId = createUniqueId();
        this.descriptionId = createUniqueId();
        this.isOpen = () => properties.open ?? isOpen();
        this.isDescribed = isDescribed;
        this.#properties = properties;
        this.#setOpen = setOpen;
        this.#setDescribed = setDescribed;
    }

    /** Open the dialog and tell the root's change handler. */
    open(): void {
        this.#setOpen(true);
        this.#properties.onOpenChange?.(true);
    }

    /** Close the dialog and tell the root's change handler. */
    close(): void {
        this.#setOpen(false);
        this.#properties.onOpenChange?.(false);
    }

    /** Describe the dialog by its description until the description unmounts. */
    describe(): void {
        this.#setDescribed(true);
        onCleanup(() => this.#setDescribed(false));
    }
}

/** The properties of a dialog root. */
export interface DialogProperties {
    /** Whether the dialog is open, which makes the open state controlled. */
    readonly open?: boolean;
    /** Whether the dialog starts open when its state is uncontrolled. */
    readonly defaultOpen?: boolean;
    /** Handle the dialog opening or closing. */
    readonly onOpenChange?: (open: boolean) => void;
    /** The trigger and content. */
    readonly children?: JSX.Element;
}

/** The properties of a dialog's trigger and close buttons, a button's properties included. */
export type DialogButtonProperties = Omit<ButtonProperties, "onClick"> & {
    /** Handle a click before the button opens or closes its dialog. */
    readonly onClick?: (event: MouseEvent) => void;
};

/** The properties of a dialog's content, the native dialog's attributes included. */
export interface DialogContentProperties extends Omit<
    JSX.DialogHtmlAttributes<HTMLDialogElement>,
    "class" | "style" | "ref" | "onClose"
> {
    /** Whether to show a close button in the top corner, true by default. */
    readonly showCloseButton?: boolean;
    /** The StyleX styles applied after the dialog's styles. */
    readonly style?: style.Styles;
}

/** The properties of an element of a dialog, the native element's attributes included. */
export type DialogElementProperties<Target extends HTMLElement> = Omit<
    JSX.HTMLAttributes<Target>,
    "class" | "style"
> & {
    /** The StyleX styles applied after the element's styles. */
    readonly style?: style.Styles;
};

/** Read the dialog of the nearest dialog root, refusing elements outside one. */
export function useDialog(): DialogControl {
    const control = useContext(DialogContext);
    if (control === null) {
        throw new TypeError("dialog elements need a dialog root around them");
    }

    return control;
}

/** Hold the open state a dialog's trigger, content and text share. */
export function Dialog(properties: DialogProperties): JSX.Element {
    return (
        <DialogContext value={new DialogControl(properties)}>{properties.children}</DialogContext>
    );
}

/** Render a button that opens its dialog. */
export function DialogTrigger(properties: DialogButtonProperties): JSX.Element {
    const control = useDialog();

    return (
        <Button
            data-slot="dialog-trigger"
            aria-haspopup="dialog"
            aria-expanded={control.isOpen() ? "true" : "false"}
            aria-controls={control.id}
            {...properties}
            onClick={(event) => {
                // run the caller's handler, then open the dialog
                properties.onClick?.(event);
                control.open();
            }}
        />
    );
}

/** Render a button that closes its dialog. */
export function DialogClose(properties: DialogButtonProperties): JSX.Element {
    const control = useDialog();

    return (
        <Button
            data-slot="dialog-close"
            {...properties}
            onClick={(event) => {
                // run the caller's handler, then close the dialog
                properties.onClick?.(event);
                control.close();
            }}
        />
    );
}

/** Render the modal dialog, shown in the top layer with the page behind it inert while open. */
export function DialogContent(properties: DialogContentProperties): JSX.Element {
    // read the dialog and keep its element for the effect below
    const control = useDialog();
    const locale = useLocale();
    const rest = omit(properties, "showCloseButton", "style", "children");
    let element: HTMLDialogElement | undefined;

    // show or close the dialog element as the state changes
    createEffect(control.isOpen, (isOpen) => {
        // wait for the dialog element to mount
        if (element === undefined) {
            return;
        }

        // open it modally or close it
        if (isOpen && !element.open) {
            element.showModal();
        } else if (!isOpen && element.open) {
            element.close();
        }
    });

    return (
        <TopLayer>
            <dialog
                id={control.id}
                data-slot="dialog-content"
                closedby="any"
                aria-labelledby={control.titleId}
                aria-describedby={control.isDescribed() ? control.descriptionId : undefined}
                {...rest}
                ref={(dialog) => (element = dialog)}
                onClose={() => {
                    // follow the platform closing the dialog, such as on Escape
                    if (control.isOpen()) {
                        control.close();
                    }
                }}
                {...style.attrs(
                    styles.content,
                    control.isOpen() ? transition.enter : transition.exit,
                    properties.style,
                )}
            >
                {properties.children}
                <Show when={properties.showCloseButton !== false}>
                    <Button
                        variant="ghost"
                        size="icon-sm"
                        data-slot="dialog-close"
                        aria-label={locale.render(t`Close`)}
                        onClick={() => control.close()}
                        style={styles.close}
                    >
                        <Icon name="x" />
                    </Button>
                </Show>
            </dialog>
        </TopLayer>
    );
}

/** Render the top of a dialog that holds its title and description. */
export function DialogHeader(properties: DialogElementProperties<HTMLDivElement>): JSX.Element {
    const rest = omit(properties, "style");

    return (
        <div
            data-slot="dialog-header"
            {...rest}
            {...style.attrs(styles.header, properties.style)}
        />
    );
}

/** Render the bottom row of a dialog that holds its actions. */
export function DialogFooter(properties: DialogElementProperties<HTMLDivElement>): JSX.Element {
    const rest = omit(properties, "style");

    return (
        <div
            data-slot="dialog-footer"
            {...rest}
            {...style.attrs(styles.footer, properties.style)}
        />
    );
}

/** Render the title that names its dialog. */
export function DialogTitle(properties: DialogElementProperties<HTMLHeadingElement>): JSX.Element {
    const control = useDialog();
    const rest = omit(properties, "style");

    return (
        <h2
            id={control.titleId}
            data-slot="dialog-title"
            {...rest}
            {...style.attrs(text.headline, styles.title, properties.style)}
        />
    );
}

/** Render the description that describes its dialog. */
export function DialogDescription(
    properties: DialogElementProperties<HTMLParagraphElement>,
): JSX.Element {
    // describe the dialog for as long as the description renders
    const control = useDialog();
    const rest = omit(properties, "style");
    control.describe();

    return (
        <p
            id={control.descriptionId}
            data-slot="dialog-description"
            {...rest}
            {...style.attrs(text.footnote, styles.description, properties.style)}
        />
    );
}
