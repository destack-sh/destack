import type { JSX } from "@destack/view";
import { Button, type ButtonProperties } from "../button/index.ts";
import { mergeProperties } from "../part/index.ts";
import {
    Dialog,
    DialogContent,
    DialogDescription,
    DialogFooter,
    DialogHeader,
    DialogTitle,
    DialogTrigger,
    type DialogButtonProperties,
    type DialogContentProperties,
    type DialogElementProperties,
    type DialogProperties,
    useDialog,
} from "../dialog/index.ts";

/** Hold the open state of a dialog that asks to confirm an action. */
export function AlertDialog(properties: DialogProperties): JSX.Element {
    return <Dialog {...properties} />;
}

/** Render a button that opens its alert dialog. */
export function AlertDialogTrigger(properties: DialogButtonProperties): JSX.Element {
    return <DialogTrigger data-slot="alert-dialog-trigger" {...properties} />;
}

/** Render the alert dialog, which only a close request or one of its buttons closes. */
export function AlertDialogContent(
    properties: Omit<DialogContentProperties, "showCloseButton">,
): JSX.Element {
    return (
        <DialogContent
            role="alertdialog"
            data-slot="alert-dialog-content"
            closedby="closerequest"
            {...properties}
            showCloseButton={false}
        />
    );
}

/** Render the top of an alert dialog with its title and description. */
export function AlertDialogHeader(
    properties: DialogElementProperties<HTMLDivElement>,
): JSX.Element {
    return <DialogHeader data-slot="alert-dialog-header" {...properties} />;
}

/** Render the bottom row of an alert dialog with its action and cancel buttons. */
export function AlertDialogFooter(
    properties: DialogElementProperties<HTMLDivElement>,
): JSX.Element {
    return <DialogFooter data-slot="alert-dialog-footer" {...properties} />;
}

/** Render the title that names its alert dialog. */
export function AlertDialogTitle(
    properties: DialogElementProperties<HTMLHeadingElement>,
): JSX.Element {
    return <DialogTitle data-slot="alert-dialog-title" {...properties} />;
}

/** Render the description that describes its alert dialog. */
export function AlertDialogDescription(
    properties: DialogElementProperties<HTMLParagraphElement>,
): JSX.Element {
    return <DialogDescription data-slot="alert-dialog-description" {...properties} />;
}

/** Render the button that confirms the action and closes its alert dialog. */
export function AlertDialogAction(properties: ButtonProperties): JSX.Element {
    const control = useDialog();

    return (
        <Button
            data-slot="alert-dialog-action"
            {...mergeProperties({ onClick: () => control.close() }, properties)}
        />
    );
}

/** Render the button that cancels the action, focused when the alert dialog opens. */
export function AlertDialogCancel(properties: ButtonProperties): JSX.Element {
    const control = useDialog();

    return (
        <Button
            data-slot="alert-dialog-cancel"
            variant="outline"
            autofocus
            {...mergeProperties({ onClick: () => control.close() }, properties)}
        />
    );
}
