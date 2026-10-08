import { expect, test } from "@destack/test";
import { flush } from "@destack/view";
import {
    AlertDialog,
    AlertDialogAction,
    AlertDialogCancel,
    AlertDialogContent,
    AlertDialogTitle,
} from "./index.ts";
import { markup, render } from "@destack/view/test";

/** Find the element of a container's first match, refusing none. */
function find(container: Element, selector: string): HTMLElement {
    const element = container.querySelector<HTMLElement>(selector);
    if (element === null) {
        throw new TypeError(`no element matches ${selector}`);
    }

    return element;
}

/** Find the dialog element of a container. */
function dialogOf(container: Element): HTMLDialogElement {
    const dialog = container.querySelector("dialog");
    if (dialog === null) {
        throw new TypeError("no dialog element");
    }

    return dialog;
}

test("render an alert dialog that only its buttons or a close request close", () => {
    const { container } = render(() => (
        <AlertDialog defaultOpen>
            <AlertDialogContent>
                <AlertDialogTitle>Delete note?</AlertDialogTitle>
                <AlertDialogCancel>Cancel</AlertDialogCancel>
                <AlertDialogAction variant="destructive">Delete</AlertDialogAction>
            </AlertDialogContent>
        </AlertDialog>
    ));
    const before = markup(container);
    find(container, "[data-slot=alert-dialog-action]").click();
    flush();

    // the cancel button takes the focus on open, and the action closes the dialog
    expect(before).toBe(
        '<dialog id="id-1" data-slot="alert-dialog-content" data-state="open" closedby="closerequest" aria-labelledby="id-2" role="alertdialog" open="">' +
            '<h2 id="id-2" data-slot="alert-dialog-title">Delete note?</h2>' +
            '<button data-slot="alert-dialog-cancel" data-variant="outline" data-size="default" autofocus="">Cancel</button>' +
            '<button data-slot="alert-dialog-action" data-variant="destructive" data-size="default">Delete</button></dialog>',
    );
    expect(dialogOf(container).open).toBe(false);
});
