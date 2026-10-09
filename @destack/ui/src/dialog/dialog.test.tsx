import { expect, onTestFinished, test } from "@destack/test";
import { createSignal, flush } from "@destack/view";
import {
    Dialog,
    DialogClose,
    DialogContent,
    DialogDescription,
    DialogTitle,
    DialogTrigger,
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

/** The close button every dialog shows by default, its icon body left out. */
const CLOSE_BUTTON =
    '<button data-slot="dialog-close" data-variant="ghost" data-size="icon-sm" aria-label="Close">' +
    '<svg viewBox="0 0 256 256" fill="currentColor" width="1em" height="1em" aria-hidden="true" data-slot="icon"></svg></button>';

test("open a modal dialog from its trigger, named by its title and described by its description", () => {
    const { container } = render(() => (
        <Dialog>
            <DialogTrigger>Rename</DialogTrigger>
            <DialogContent>
                <DialogTitle>Rename note</DialogTitle>
                <DialogDescription>Pick a new title.</DialogDescription>
            </DialogContent>
        </Dialog>
    ));
    find(container, "[data-slot=dialog-trigger]").click();
    flush();

    // the trigger reports the open dialog it controls, and the dialog is open
    expect(markup(container)).toBe(
        '<button data-slot="dialog-trigger" data-variant="default" data-size="default" aria-haspopup="dialog" aria-expanded="true" aria-controls="id-1">Rename</button>' +
            '<dialog id="id-1" data-slot="dialog-content" data-state="open" closedby="any" aria-labelledby="id-2" aria-describedby="id-3" open="">' +
            '<h2 data-slot="dialog-title" id="id-2">Rename note</h2>' +
            '<p data-slot="dialog-description" id="id-3">Pick a new title.</p>' +
            `${CLOSE_BUTTON}</dialog>`,
    );
});

test("close a dialog from a close button and report each change", () => {
    const changes: boolean[] = [];
    const { container } = render(() => (
        <Dialog defaultOpen onOpenChange={(open) => changes.push(open)}>
            <DialogContent showCloseButton={false}>
                <DialogTitle>Rename note</DialogTitle>
                <DialogClose>Done</DialogClose>
            </DialogContent>
        </Dialog>
    ));
    const dialog = dialogOf(container);
    const wasOpen = dialog.open;
    find(container, "[data-slot=dialog-close]").click();
    flush();

    // start the dialog open and close it once, without a description to point at
    expect([wasOpen, dialog.open, changes]).toEqual([true, false, [false]]);
    expect(dialog.hasAttribute("aria-describedby")).toBe(false);
});

test("follow the platform closing a dialog, such as on Escape or a click outside", () => {
    const changes: boolean[] = [];
    const { container } = render(() => (
        <Dialog defaultOpen onOpenChange={(open) => changes.push(open)}>
            <DialogTrigger>Rename</DialogTrigger>
            <DialogContent>
                <DialogTitle>Rename note</DialogTitle>
            </DialogContent>
        </Dialog>
    ));
    dialogOf(container).close();
    flush();

    // the close event closes the state the trigger reports
    expect(changes).toEqual([false]);
    expect(find(container, "[data-slot=dialog-trigger]").getAttribute("aria-expanded")).toBe(
        "false",
    );
});

test("open and close a controlled dialog with its open property", () => {
    const [open, setOpen] = createSignal(false);
    const { container } = render(() => (
        <Dialog open={open()}>
            <DialogContent>
                <DialogTitle>Rename note</DialogTitle>
            </DialogContent>
        </Dialog>
    ));
    const dialog = dialogOf(container);
    const states = [dialog.open];
    setOpen(true);
    flush();
    states.push(dialog.open);
    setOpen(false);
    flush();
    states.push(dialog.open);
    expect(states).toEqual([false, true, false]);
});

test("open a dialog from a trigger rendered as another element, which keeps the trigger's state and behaviour", () => {
    const { container } = render(() => (
        <Dialog>
            <DialogTrigger render={(part) => <span tabindex={0} {...part} />}>Rename</DialogTrigger>
            <DialogContent>
                <DialogTitle>Rename note</DialogTitle>
            </DialogContent>
        </Dialog>
    ));
    find(container, "[data-slot=dialog-trigger]").click();
    flush();
    expect([
        find(container, "[data-slot=dialog-trigger]").tagName,
        find(container, "[data-slot=dialog-trigger]").textContent,
        find(container, "[data-slot=dialog-trigger]").getAttribute("aria-expanded"),
        dialogOf(container).open,
    ]).toEqual(["SPAN", "Rename", "true", true]);
});

test("open a non-modal dialog beside the page, and a modal one over it", () => {
    // record how each dialog element opens, as the test DOM keeps no top layer
    const opened: string[] = [];
    const prototype = HTMLDialogElement.prototype;
    const original = Object.getOwnPropertyDescriptors(prototype);
    const record = (how: string) =>
        function (this: HTMLDialogElement) {
            opened.push(`${this.textContent ?? ""} ${how}`);
            this.toggleAttribute("open", true);
        };
    Object.assign(prototype, {
        show: record("beside the page"),
        showModal: record("over the page"),
    });
    onTestFinished(() => {
        Object.defineProperties(prototype, { show: original.show, showModal: original.showModal });
    });

    render(() => (
        <>
            <Dialog defaultOpen modal={false}>
                <DialogContent showCloseButton={false}>
                    <DialogTitle>Find</DialogTitle>
                </DialogContent>
            </Dialog>
            <Dialog defaultOpen>
                <DialogContent showCloseButton={false}>
                    <DialogTitle>Rename note</DialogTitle>
                </DialogContent>
            </Dialog>
        </>
    ));
    flush();
    expect(opened).toEqual(["Find beside the page", "Rename note over the page"]);
});
