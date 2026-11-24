import { Icon } from "@destack/ui/icon";
import { t } from "@destack/locale";
import * as style from "@destack/style";
import { transition } from "@destack/theme/motion";
import { color, motion, radius, shadow, space, stroke, weight } from "@destack/theme/tokens.stylex";
import { text } from "@destack/theme/text";
import {
    type Accessor,
    createContext,
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
import {
    Dismissal,
    LayerStack,
    controlOf,
    OpenChange,
    type OpenChildren,
    type OpenProperties,
    OpenState,
    TopLayer,
    type TriggerProperties,
} from "../layer/index.ts";
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
        top: `calc(${space[5]} - ${space[1]})`,
        insetInlineEnd: space[3],
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
export class DialogControl<Payload = unknown> {
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
    /** The open state, and the trigger it opened from. */
    readonly state: OpenState<Payload>;
    /** Why the platform closes the dialog. */
    readonly #dismissal: Dismissal;
    /** The descriptions on screen. */
    readonly #descriptions: CollectionBuilder<string>;

    /** Create the state of a dialog, open when its options ask for it. */
    constructor(options: DialogOptions = {}) {
        // follow the controlled state, else the dialog's own
        this.#dismissal = new Dismissal();
        this.id = createUniqueId();
        this.state = new OpenState(options, this.id);
        this.titleId = createUniqueId();
        this.descriptionId = createUniqueId();
        this.isOpen = this.state.isOpen;
        this.isDescribed = () => this.#descriptions.size() > 0;
        this.isModal = () => options.modal !== false;
        this.#descriptions = new CollectionBuilder();
    }

    /** Open the dialog, unless its handler cancels it. */
    open(change: OpenChange<Payload> = new OpenChange()): void {
        this.state.set(true, change);
    }

    /** Close the dialog, unless its handler cancels it. */
    close(change: OpenChange<Payload> = new OpenChange()): void {
        this.state.set(false, change);
    }

    /** Show or close the dialog element for the open state. */
    sync(element: HTMLDialogElement): void {
        // open it modally on top of the document's stack
        const isOpen = untrack(this.isOpen);
        if (isOpen && !element.open && untrack(this.isModal)) {
            element.showModal();
            LayerStack.of(element).open(element);
            this.#dismissal.start(element);
        }
        // open it beside the page
        else if (isOpen && !element.open) {
            element.show();
            this.#dismissal.start(element);
        }
        // close it
        else if (!isOpen && element.open) {
            element.close();
        }
    }

    /** Close the dialog as the platform asks for, such as on Escape or a press outside. */
    request(event: Event & { readonly currentTarget: HTMLDialogElement }): void {
        // close by the state, as the owner may refuse
        const change = this.#dismissal.request(event);
        if (change !== undefined) {
            this.close(change);
        }
    }

    /** Follow the platform closing the dialog, showing it again where its owner refuses. */
    follow(element: HTMLDialogElement): void {
        // leave the stack
        LayerStack.of(element).close(element);
        const change = this.#dismissal.change(element, undefined);
        this.#dismissal.stop();
        if (untrack(this.isOpen) && !this.state.follow(false, change)) {
            this.sync(element);
        }
    }

    /** Forget an element that unmounts. */
    detach(element: HTMLDialogElement): void {
        this.#dismissal.stop();
        LayerStack.of(element).close(element);
    }

    /** Describe the dialog by its description until the description unmounts. */
    describe(): void {
        this.#descriptions.add(this.descriptionId);
    }
}

/** How a dialog opens. */
export interface DialogOptions extends OpenProperties {
    /** Whether the page behind the open dialog is inert, true by default. */
    readonly modal?: boolean;
}

/** The properties of a dialog root. */
export interface DialogProperties<Payload = unknown> extends DialogOptions {
    /** The dialog that triggers elsewhere open, which holds the open state in the root's place. */
    readonly control?: DialogControl<Payload>;
    /** The trigger and content, or what it shows for the trigger it opened from. */
    readonly children?: OpenChildren<Payload>;
}

/** The properties of a dialog's buttons, a button's properties included. */
export type DialogButtonProperties = ElementPartProperties &
    Omit<JSX.ButtonHTMLAttributes<HTMLButtonElement>, "class" | "style">;

/** The properties of a dialog's trigger, a button's properties included. */
export type DialogTriggerProperties<Payload = unknown> = DialogButtonProperties &
    TriggerProperties<Payload, DialogControl<Payload>>;

/** The properties of a dialog's content, the native dialog's attributes included. */
export interface DialogContentProperties extends Omit<
    JSX.DialogHtmlAttributes<HTMLDialogElement>,
    "class" | "ref" | "onCancel" | "onClose"
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
export function Dialog<Payload = unknown>(properties: DialogProperties<Payload>): JSX.Element {
    // take the caller's dialog, else one of its own
    const control = controlOf(properties, () => new DialogControl<Payload>(properties));

    return (
        <DialogContext value={control}>{control.state.render(properties.children)}</DialogContext>
    );
}

/** Render the element that opens its dialog, a button by default. */
export function DialogTrigger<Payload = unknown>(
    properties: DialogTriggerProperties<Payload>,
): JSX.Element {
    // open its root's dialog, or the one it is given from elsewhere
    const control = untrack(() => properties.control) ?? useDialog();
    const rest = omit(properties, "control", "payload");
    let element: HTMLElement | undefined;

    return renderPart("button", "dialog-trigger", rest, [], {
        type: "button",
        "aria-haspopup": "dialog",
        get "aria-expanded"() {
            return control.state.isFrom(element) ? "true" : "false";
        },
        "aria-controls": control.id,
        ref: (target: HTMLElement) => (element = target),
        onClick: (event: MouseEvent & { readonly currentTarget: HTMLElement }) => {
            const trigger = { element: event.currentTarget, payload: properties.payload };
            control.open(new OpenChange("trigger-press", event, trigger));
        },
    });
}

/** Render the element that closes its dialog, a button by default. */
export function DialogClose(properties: DialogButtonProperties): JSX.Element {
    const control = useDialog();

    return renderPart("button", "dialog-close", properties, [], {
        type: "button",
        onClick: (event: MouseEvent) => control.close(new OpenChange("close-press", event)),
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
            control.detach(element);
        }
    });

    // show or close the dialog element as the state changes
    createEffect(
        () => ({ isOpen: control.isOpen(), isModal: control.isModal() }),
        () => {
            // wait for the dialog element to mount
            if (element !== undefined) {
                control.sync(element);
            }
        },
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
                onCancel={(event) => control.request(event)}
                onClose={(event) => control.follow(event.currentTarget)}
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
                        onClick={(event) => control.close(new OpenChange("close-press", event))}
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
