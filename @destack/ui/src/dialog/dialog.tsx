import { Icon } from "@destack/ui/icon";
import { t } from "@destack/locale";
import * as style from "@destack/style";
import { transition } from "@destack/theme/motion";
import { color, motion, radius, shadow, space, stroke, weight } from "@destack/theme/tokens.stylex";
import { text } from "@destack/theme/text";
import {
    type Accessor,
    createContext,
    createControllableSignal,
    createEffect,
    createUniqueId,
    type JSX,
    omit,
    onCleanup,
    Show,
    untrack,
    useContext,
    useLocale,
} from "@destack/view";
import { Button } from "../button/index.ts";
import { LayerStack, TopLayer } from "../layer/index.ts";
import { type ElementPartProperties, renderPart } from "../part/index.ts";
import { CollectionBuilder } from "../collection/index.ts";
import { scrim } from "./scrim.stylex.ts";

/** The widest a dialog grows. */
const DIALOG_WIDTH = "32rem";

/** The opacity of a backdrop over another dialog, half as the page is dimmed already. */
const NESTED_SCRIM = "0.5";

/** The time a backdrop fades for, kept when reduced motion drops movement. */
const SCRIM_FADE = `max(calc(${motion.durationMedium} * 2 / 3), 150ms)`;

/** The styles of a dialog and its elements. */
const styles = style.create({
    content: {
        display: { default: "none", ":modal": "grid" },
        gap: space[4],
        width: `min(100% - 2 * ${space[4]}, ${DIALOG_WIDTH})`,
        maxWidth: "none",
        margin: "auto",
        padding: space[5],
        borderWidth: stroke.border,
        borderColor: color.border,
        borderRadius: radius[5],
        backgroundColor: color.popover,
        color: color.foreground,
        boxShadow: shadow.overlay,
        // take the focus without a ring where nothing inside takes it
        outlineStyle: "none",
        // fade the backdrop with the dialog, which a drag may fade further
        [scrim.opacity]: {
            default: "0",
            ":modal": { default: "1", ":is(:modal *)": NESTED_SCRIM },
        },
        "::backdrop": {
            backgroundColor: color.scrim,
            opacity: { default: `calc(${scrim.opacity} * ${scrim.share})`, "@starting-style": 0 },
            transitionProperty: "opacity, display, overlay",
            transitionDuration: SCRIM_FADE,
            transitionBehavior: "allow-discrete",
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
        fontWeight: weight.medium,
    },
    description: {
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
    /** Whether the page behind the open dialog is inert. */
    readonly isModal: Accessor<boolean>;
    /** Replace the open state and tell the change handler. */
    readonly #setOpen: (isOpen: boolean) => void;
    /** The descriptions on screen. */
    readonly #descriptions: CollectionBuilder<string>;

    /** Create the state of a dialog root, open when its properties ask for it. */
    constructor(properties: DialogProperties) {
        // follow the controlled state, else the dialog's own
        const [isOpen, setOpen] = createControllableSignal({
            isControlled: () => properties.open !== undefined,
            value: () => properties.open === true,
            defaultValue: untrack(() => properties.defaultOpen === true),
            onChange: (isNext) => properties.onOpenChange?.(isNext),
        });
        this.id = createUniqueId();
        this.titleId = createUniqueId();
        this.descriptionId = createUniqueId();
        this.isOpen = isOpen;
        this.isDescribed = () => this.#descriptions.size() > 0;
        this.isModal = () => properties.modal !== false;
        this.#setOpen = setOpen;
        this.#descriptions = new CollectionBuilder();
    }

    /** Open the dialog and tell the root's change handler. */
    open(): void {
        this.#setOpen(true);
    }

    /** Close the dialog and tell the root's change handler. */
    close(): void {
        this.#setOpen(false);
    }

    /** Describe the dialog by its description until the description unmounts. */
    describe(): void {
        this.#descriptions.add(this.descriptionId);
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
    /** Whether the page behind the open dialog is inert, true by default. */
    readonly modal?: boolean;
    /** The trigger and content. */
    readonly children?: JSX.Element;
}

/** The properties of a dialog's trigger and close buttons, a button's properties included. */
export type DialogButtonProperties = ElementPartProperties &
    Omit<JSX.ButtonHTMLAttributes<HTMLButtonElement>, "class" | "style">;

/** The properties of a dialog's content, the native dialog's attributes included. */
export interface DialogContentProperties extends Omit<
    JSX.DialogHtmlAttributes<HTMLDialogElement>,
    "class" | "ref" | "onClose"
> {
    /** Whether to show a close button in the top corner, true by default. */
    readonly showCloseButton?: boolean;
    /** The StyleX styles applied after the dialog's styles. */
    readonly xstyle?: style.Styles;
}

/** The properties of an element of a dialog, the native element's attributes included. */
export type DialogElementProperties<Target extends HTMLElement> = Omit<
    JSX.HTMLAttributes<Target>,
    "class" | "style"
> &
    ElementPartProperties;

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

/** Render the element that opens its dialog, a button by default. */
export function DialogTrigger(properties: DialogButtonProperties): JSX.Element {
    const control = useDialog();

    return renderPart("button", "dialog-trigger", properties, [], {
        type: "button",
        "aria-haspopup": "dialog",
        get "aria-expanded"() {
            return control.isOpen() ? "true" : "false";
        },
        "aria-controls": control.id,
        onClick: () => control.open(),
    });
}

/** Render the element that closes its dialog, a button by default. */
export function DialogClose(properties: DialogButtonProperties): JSX.Element {
    const control = useDialog();

    return renderPart("button", "dialog-close", properties, [], {
        type: "button",
        onClick: () => control.close(),
    });
}

/** Render the modal dialog, shown in the top layer with the page behind it inert while open. */
export function DialogContent(properties: DialogContentProperties): JSX.Element {
    // read the dialog and store its element for the effect below
    const control = useDialog();
    const locale = useLocale();
    const rest = omit(properties, "showCloseButton", "xstyle", "style", "children");
    let element: HTMLDialogElement | undefined;
    onCleanup(() => {
        // leave the stack when the dialog unmounts open
        if (element !== undefined) {
            LayerStack.of(element).close(element);
        }
    });

    // show or close the dialog element as the state changes
    createEffect(
        () => ({ isOpen: control.isOpen(), isModal: control.isModal() }),
        ({ isOpen, isModal }) =>
            untrack(() => {
                // wait for the dialog element to mount
                if (element === undefined) {
                    return;
                }

                // open it modally on top of the document's stack
                if (isOpen && !element.open && isModal) {
                    element.showModal();
                    LayerStack.of(element).open(element);
                }
                // open it beside the page
                else if (isOpen && !element.open) {
                    element.show();
                }
                // close it
                else if (!isOpen && element.open) {
                    element.close();
                }
            }),
    );

    return (
        <TopLayer>
            <dialog
                id={control.id}
                data-slot="dialog-content"
                data-open={control.isOpen() ? "" : undefined}
                data-closed={control.isOpen() ? undefined : ""}
                closedby="any"
                aria-labelledby={control.titleId}
                aria-describedby={control.isDescribed() ? control.descriptionId : undefined}
                {...rest}
                ref={(dialog) => (element = dialog)}
                onClose={(event) => {
                    // follow the platform closing the dialog, such as on Escape, leaving the stack
                    LayerStack.of(event.currentTarget).close(event.currentTarget);
                    if (control.isOpen()) {
                        control.close();
                    }
                }}
                {...style.attributes(
                    [styles.content, transition.dialog, properties.xstyle],
                    properties.style,
                )}
            >
                {properties.children}
                <Show when={properties.showCloseButton !== false}>
                    <Button
                        variant="ghost"
                        size="icon"
                        data-slot="dialog-close"
                        aria-label={locale.render(t`Close`)}
                        onClick={() => control.close()}
                        xstyle={styles.close}
                    >
                        <Icon name="x" />
                    </Button>
                </Show>
            </dialog>
        </TopLayer>
    );
}

/** Render the top of a dialog with its title and description. */
export function DialogHeader(properties: DialogElementProperties<HTMLDivElement>): JSX.Element {
    return renderPart("div", "dialog-header", properties, styles.header);
}

/** Render the bottom row of a dialog with its actions. */
export function DialogFooter(properties: DialogElementProperties<HTMLDivElement>): JSX.Element {
    return renderPart("div", "dialog-footer", properties, styles.footer);
}

/** Render the title that names its dialog. */
export function DialogTitle(properties: DialogElementProperties<HTMLHeadingElement>): JSX.Element {
    const control = useDialog();

    return renderPart("h2", "dialog-title", properties, [text.lg, styles.title], {
        id: control.titleId,
    });
}

/** Render the description that describes its dialog. */
export function DialogDescription(
    properties: DialogElementProperties<HTMLParagraphElement>,
): JSX.Element {
    // describe the dialog for as long as the description renders
    const control = useDialog();
    control.describe();

    return renderPart("p", "dialog-description", properties, [text.sm, styles.description], {
        id: control.descriptionId,
    });
}
