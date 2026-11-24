import { expect, onTestFinished, test } from "@destack/test";
import { createSignal, flush } from "@destack/view";
import { OpenChange } from "../layer/index.ts";
import {
    Dialog,
    DialogClose,
    DialogContent,
    DialogControl,
    DialogDescription,
    DialogTitle,
    DialogTrigger,
} from "./index.ts";
import { markup, render } from "@destack/view/test";

/** Find the element of a container's first match, refusing none. */
function find(container: Element, selector: string): HTMLElement {
    const element = container.querySelector<HTMLElement>(selector);
    if (element === null) {
        throw new TypeError(`no element matches '${selector}'`);
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

/** Ask an element to cancel as the platform does, reporting whether nothing prevented it. */
function cancel(target: EventTarget, isCancelable = true): boolean {
    return target.dispatchEvent(new Event("cancel", { bubbles: true, cancelable: isCancelable }));
}

/** The close button every dialog shows by default, its icon body left out. */
const CLOSE_BUTTON =
    '<button data-slot="dialog-close" data-variant="ghost" data-size="icon" type="button" aria-label="Close">' +
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
        '<button data-slot="dialog-trigger" type="button" aria-haspopup="dialog" aria-expanded="true" aria-controls="id-1">Rename</button>' +
            '<dialog id="id-1" data-slot="dialog-content" closedby="any" aria-labelledby="id-2" aria-describedby="id-3" data-open="" open="">' +
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

test("open a dialog from a trigger rendered as another element, with the trigger's state and behaviour", () => {
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
    // record how each dialog element opens, as the test DOM has no top layer
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

test("keep a dialog whose owner cancels its close open, telling why it would close", () => {
    // cancel every close, asked for by Escape, a press outside, a button and the platform itself
    const asked: unknown[] = [];
    const { container } = render(() => (
        <Dialog
            defaultOpen
            onOpenChange={(open, change) => {
                asked.push([open, change.reason, change.event?.type]);
                change.cancel();
            }}
        >
            <DialogContent showCloseButton={false}>
                <DialogTitle>Draft</DialogTitle>
                <DialogClose>Done</DialogClose>
                <input type="file" />
            </DialogContent>
        </Dialog>
    ));
    const dialog = dialogOf(container);
    document.body.dispatchEvent(new KeyboardEvent("keydown", { key: "Escape", bubbles: true }));
    const isEscapeTaken = cancel(dialog);
    document.body.dispatchEvent(new PointerEvent("pointerdown", { bubbles: true }));
    cancel(dialog);
    find(container, "[data-slot=dialog-close]").click();
    document.body.dispatchEvent(new KeyboardEvent("keydown", { key: "Escape", bubbles: true }));
    const isForcedTaken = cancel(dialog, false);
    dialog.close();
    flush();

    // leave a file picker's own cancel alone, and show the dialog again after the platform closed it
    const isPickerTaken = cancel(find(container, "input"));
    expect({ asked, isEscapeTaken, isForcedTaken, isPickerTaken, isOpen: dialog.open }).toEqual({
        asked: [
            [false, "escape-key", "keydown"],
            [false, "outside-press", "pointerdown"],
            [false, "close-press", "click"],
            [false, "escape-key", "keydown"],
        ],
        isEscapeTaken: false,
        isForcedTaken: true,
        isPickerTaken: true,
        isOpen: true,
    });
});

test("show a controlled dialog for as long as its owner has it open", () => {
    const [isOpen, setOpen] = createSignal(true);
    const { container } = render(() => (
        <Dialog open={isOpen()} onOpenChange={() => {}}>
            <DialogContent>
                <DialogTitle>Draft</DialogTitle>
            </DialogContent>
        </Dialog>
    ));
    const dialog = dialogOf(container);
    dialog.close();
    flush();
    const isLeft = dialog.open;
    setOpen(false);
    flush();

    expect([isLeft, dialog.open]).toEqual([true, false]);
});

test("open one dialog from triggers elsewhere, showing each one's payload", () => {
    // open from the second picture, then show the third while it is open
    const { container } = render(() => {
        const viewer = new DialogControl<string>();

        return (
            <>
                <DialogTrigger control={viewer} payload="Lake">
                    Lake
                </DialogTrigger>
                <DialogTrigger control={viewer} payload="Tram">
                    Tram
                </DialogTrigger>
                <button
                    type="button"
                    id="next"
                    onClick={(event) => {
                        const trigger = { element: event.currentTarget, payload: "Pass" };
                        viewer.open(new OpenChange("none", event, trigger));
                    }}
                >
                    Next
                </button>
                <Dialog control={viewer}>
                    {(name) => (
                        <DialogContent showCloseButton={false}>
                            <DialogTitle>{name()}</DialogTitle>
                        </DialogContent>
                    )}
                </Dialog>
            </>
        );
    });
    flush();
    const dialog = dialogOf(container);
    const steps = ["[data-slot=dialog-trigger]:nth-of-type(2)", "#next"].map((selector) => {
        find(container, selector).click();
        flush();

        return [dialog.open, dialog.textContent];
    });

    expect(steps).toEqual([
        [true, "Tram"],
        [true, "Pass"],
    ]);
});
