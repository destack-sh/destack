import { TopLayer } from "../layer/index.ts";
import type { OpenChange } from "../layer/index.ts";
import { type Anchor, Position } from "../position/index.ts";
import { expect, onTestFinished, test, vi } from "@destack/test";
import {
    Popover,
    PopoverAnchor,
    PopoverClose,
    PopoverContent,
    PopoverControl,
    PopoverDescription,
    PopoverTitle,
    PopoverTrigger,
} from "./index.ts";
import { present } from "@destack/schema";
import { createSignal, flush, Show } from "@destack/view";
import { markup, render, stubPopovers } from "@destack/view/test";

test("toggle a popover from its trigger through the platform's popover target, anchored to the trigger by name", () => {
    const { container } = render(() => (
        <Popover>
            <PopoverTrigger>Share</PopoverTrigger>
            <PopoverContent align="start">Anyone with the link can view.</PopoverContent>
        </Popover>
    ));
    expect(markup(container)).toBe(
        '<button data-slot="popover-trigger" type="button" id="id-1-trigger" popovertarget="id-1" aria-haspopup="dialog" aria-controls="id-1" data-anchor-name="--id-1">Share</button>' +
            '<div data-position-anchor="--id-1" data-slot="popover-content" id="id-1" popover="auto" role="dialog" tabindex="-1" aria-labelledby="id-1-trigger" data-closed="" data-side="bottom" data-align="start"></div>',
    );
});

test("move the focus into a shown popover, to its first control, unless one has it", () => {
    stubPopovers();

    // show a popover with controls, one without, and one whose second control took the focus
    const { container } = render(() => (
        <>
            <Popover defaultOpen>
                <PopoverContent id="controls">
                    <button type="button">First</button>
                </PopoverContent>
            </Popover>
            <Popover defaultOpen>
                <PopoverContent id="text">Nothing to press</PopoverContent>
            </Popover>
            <Popover defaultOpen>
                <PopoverContent id="taken">
                    <button type="button">First</button>
                    <button type="button">Second</button>
                </PopoverContent>
            </Popover>
        </>
    ));
    flush();
    const focused = ["controls", "text", "taken"].map((id) => {
        const content = container.ownerDocument.getElementById(id);
        if (id === "taken") {
            content?.querySelector<HTMLElement>("button:last-child")?.focus();
        }
        content?.dispatchEvent(
            Object.assign(new Event("toggle"), { newState: "open", oldState: "closed" }),
        );
        flush();

        return document.activeElement?.textContent;
    });

    expect(focused).toEqual(["First", "Nothing to press", "Second"]);
});

test("hand the focus back to the trigger as a popover closes, unless it moved on", () => {
    const { container } = render(() => (
        <>
            <Popover>
                <PopoverTrigger>Share</PopoverTrigger>
                <PopoverContent id="share">
                    <button type="button">Inside</button>
                </PopoverContent>
            </Popover>
            <button type="button">Elsewhere</button>
        </>
    ));
    flush();
    const content = container.ownerDocument.getElementById("share");
    const toggle = (newState: string) => {
        content?.dispatchEvent(Object.assign(new Event("toggle"), { newState }));
        flush();

        return document.activeElement?.textContent;
    };
    toggle("open");
    const back = toggle("closed");
    toggle("open");
    [...container.querySelectorAll("button")].at(-1)?.focus();

    expect([back, toggle("closed")]).toEqual(["Share", "Elsewhere"]);
});

test("drop a popover's anchor and its rule once the popover unmounts", () => {
    // render a popover, read its rule, then unmount it
    const { container, unmount } = render(() => (
        <Popover>
            <PopoverTrigger>Share</PopoverTrigger>
            <PopoverContent align="start">Anyone with the link can view.</PopoverContent>
        </Popover>
    ));
    const name = container
        .querySelector("[data-slot=popover-trigger]")
        ?.getAttribute("data-anchor-name");
    const rules = () =>
        document.head.querySelectorAll(`style[data-anchor-rule="${String(name)}"]`).length;
    const mounted = rules();
    unmount();

    expect({ mounted, unmounted: rules() }).toEqual({ mounted: 1, unmounted: 0 });
});

test("open a modal popover as a modal dialog anchored by name on top of the stack, and refocus its trigger once the platform closes it", () => {
    const changes: boolean[] = [];
    const { container } = render(() => (
        <Popover modal onOpenChange={(open) => changes.push(open)}>
            <PopoverTrigger>Share</PopoverTrigger>
            <PopoverContent align="start">
                <button type="button">Copy link</button>
            </PopoverContent>
        </Popover>
    ));
    const closed = markup(container);
    const trigger = container.querySelector<HTMLElement>("[data-slot=popover-trigger]");
    const content = container.querySelector("dialog");
    trigger?.click();
    flush();
    const stack = TopLayer.of(document.body);
    const opened = [content?.open, stack.top() === content, markup(container)];

    // close the dialog as Escape or a click outside does
    content?.close();
    flush();
    expect([
        closed,
        opened,
        content?.open,
        stack.top(),
        document.activeElement === trigger,
        changes,
    ]).toEqual([
        '<button data-slot="popover-trigger" type="button" id="id-1-trigger" aria-haspopup="dialog" aria-expanded="false" aria-controls="id-1">Share</button>' +
            '<dialog id="id-1" closedby="any" aria-labelledby="id-1-trigger" data-slot="popover-content" data-closed="" data-side="bottom" data-align="start"></dialog>',
        [
            true,
            true,
            '<button data-slot="popover-trigger" type="button" id="id-1-trigger" aria-haspopup="dialog" aria-expanded="true" aria-controls="id-1" data-popup-open="" data-anchor-name="--id-1">Share</button>' +
                '<dialog id="id-1" closedby="any" aria-labelledby="id-1-trigger" data-slot="popover-content" data-side="bottom" data-align="start" data-open="" data-instant="" open="" data-position-anchor="--id-1"><button type="button">Copy link</button></dialog>',
        ],
        false,
        undefined,
        true,
        [true, false],
    ]);
});

test("place a modal popover beside its anchor, name it by its title, and close it from its close button", () => {
    const { container } = render(() => (
        <Popover modal>
            <PopoverAnchor>Row</PopoverAnchor>
            <PopoverTrigger>Edit</PopoverTrigger>
            <PopoverContent>
                <PopoverTitle>Rename</PopoverTitle>
                <PopoverDescription>Names show in every list.</PopoverDescription>
                <PopoverClose>Done</PopoverClose>
            </PopoverContent>
        </Popover>
    ));
    container.querySelector<HTMLElement>("[data-slot=popover-trigger]")?.click();
    flush();
    const content = container.querySelector("dialog");
    const anchored = [
        container.querySelector("[data-slot=popover-anchor]")?.getAttribute("data-anchor-name"),
        content?.getAttribute("aria-labelledby"),
        content?.getAttribute("aria-describedby"),
    ];
    container.querySelector<HTMLElement>("[data-slot=popover-close]")?.click();
    flush();

    const id = content?.id ?? "";
    expect([anchored, content?.open]).toEqual([
        [`--${id}`, `${id}-title`, `${id}-description`],
        false,
    ]);
});

/** Let elements show as popovers for a test, and collect the ids of those that do. */
function showPopovers(): string[] {
    const shown: string[] = [];
    Object.defineProperty(HTMLElement.prototype, "showPopover", {
        configurable: true,
        value(this: HTMLElement) {
            shown.push(this.id);
        },
    });
    Object.defineProperty(HTMLElement.prototype, "hidePopover", {
        configurable: true,
        value: () => undefined,
    });
    onTestFinished(() => {
        Reflect.deleteProperty(HTMLElement.prototype, "showPopover");
        Reflect.deleteProperty(HTMLElement.prototype, "hidePopover");
    });

    return shown;
}

test("keep a popover open or closed as its owner cancels, with the focus where it was", () => {
    // refuse the platform opening one popover and closing another, which the page shows itself
    const shown = showPopovers();
    const asked: unknown[] = [];
    const ask = (open: boolean, change: OpenChange) => {
        asked.push([open, change.reason]);
        change.cancel();
    };
    const { container } = render(() => (
        <>
            <Popover onOpenChange={ask}>
                <PopoverContent id="shut">Shut</PopoverContent>
            </Popover>
            <Popover defaultOpen onOpenChange={ask}>
                <PopoverContent id="open">
                    <button type="button">First</button>
                    <button type="button">Second</button>
                </PopoverContent>
            </Popover>
        </>
    ));
    flush();
    const toggle = (id: string, type: string, newState: string) => {
        const element = container.ownerDocument.getElementById(id);
        const event = Object.assign(new Event(type, { cancelable: true }), { newState });
        const isTaken = element?.dispatchEvent(event);
        flush();

        return isTaken;
    };
    const isOpenTaken = toggle("shut", "beforetoggle", "open");
    toggle("open", "toggle", "open");
    container.querySelector<HTMLElement>("#open button:last-child")?.focus();
    document.body.dispatchEvent(new KeyboardEvent("keydown", { key: "Escape", bubbles: true }));
    toggle("open", "beforetoggle", "closed");
    document.body.focus();
    toggle("open", "toggle", "closed");
    container.querySelector<HTMLElement>("#open button:last-child")?.blur();
    toggle("open", "toggle", "open");

    expect({
        shown,
        asked,
        isOpenTaken,
        state: container.querySelector("#open")?.hasAttribute("data-open"),
        focused: document.activeElement?.textContent,
    }).toEqual({
        shown: ["open", "open"],
        asked: [
            [true, "trigger-press"],
            [false, "escape-key"],
        ],
        isOpenTaken: false,
        state: true,
        focused: "Second",
    });
});

test("close a popover another takes the place of, as no press or key lets its owner cancel", () => {
    showPopovers();
    const asked: unknown[] = [];
    const { container } = render(() => (
        <Popover
            defaultOpen
            onOpenChange={(open, change) => {
                change.cancel();
                asked.push([open, change.reason, change.isCancelable, change.isCanceled]);
            }}
        >
            <PopoverContent>Draft</PopoverContent>
        </Popover>
    ));
    flush();
    const content = container.querySelector("[data-slot=popover-content]");
    for (const newState of ["open", "closed"]) {
        content?.dispatchEvent(Object.assign(new Event("toggle"), { newState }));
        flush();
    }

    expect({ asked, isOpen: content?.hasAttribute("data-open") }).toEqual({
        asked: [[false, "none", false, false]],
        isOpen: false,
    });
});

test("open one popover from triggers elsewhere, showing each one's payload and labelled by it", () => {
    // open from one person, move to another, and close from that one
    showPopovers();
    const changes: unknown[] = [];
    const { container } = render(() => {
        const card = new PopoverControl<string>({
            onOpenChange: (open, { reason, trigger }) =>
                changes.push([open, reason, trigger?.payload]),
        });

        return (
            <>
                <PopoverTrigger control={card} payload="Ada">
                    Ada
                </PopoverTrigger>
                <PopoverTrigger control={card} payload="Ben">
                    Ben
                </PopoverTrigger>
                <Popover control={card}>
                    {(name) => <PopoverContent>{name()}</PopoverContent>}
                </Popover>
            </>
        );
    });
    flush();
    const triggers = [...container.querySelectorAll<HTMLElement>("[data-slot=popover-trigger]")];
    const content = present(container.querySelector("[data-slot=popover-content]"), "the popover");
    const steps = [0, 1, 1].map((at) => {
        triggers[at]?.click();
        flush();

        return [
            content.hasAttribute("data-open"),
            content.textContent,
            content.getAttribute("aria-labelledby") === triggers[at]?.id,
            triggers.map((trigger) => trigger.getAttribute("aria-expanded")),
        ];
    });

    expect({ steps, changes, ids: new Set(triggers.map(({ id }) => id)).size }).toEqual({
        steps: [
            [true, "Ada", true, ["true", "false"]],
            [true, "Ben", true, ["false", "true"]],
            [false, "Ben", true, ["false", "false"]],
        ],
        changes: [
            [true, "trigger-press", "Ada"],
            [true, "trigger-press", "Ben"],
            [false, "trigger-press", "Ben"],
        ],
        ids: 2,
    });
});

test("close a popover as the trigger it is open from goes away", () => {
    showPopovers();
    const [isThere, setThere] = createSignal(true);
    const { container } = render(() => {
        const card = new PopoverControl<string>();

        return (
            <>
                <Show when={isThere()}>
                    <PopoverTrigger control={card}>Ada</PopoverTrigger>
                </Show>
                <PopoverTrigger control={card}>Ben</PopoverTrigger>
                <Popover control={card}>
                    <PopoverContent>Card</PopoverContent>
                </Popover>
            </>
        );
    });
    flush();
    container.querySelector<HTMLElement>("[data-slot=popover-trigger]")?.click();
    flush();
    const isOpen = () =>
        container.querySelector("[data-slot=popover-content]")?.hasAttribute("data-open");
    const was = isOpen();
    setThere(false);
    flush();

    expect([was, isOpen()]).toEqual([true, false]);
});

test("place a popover beside an element or a box in place of its trigger", () => {
    // anchor beside an element, then where a selection is, then at the trigger again
    showPopovers();
    const heading = document.createElement("h2");
    document.body.append(heading);
    onTestFinished(() => heading.remove());
    const [anchor, setAnchor] = createSignal<Anchor | undefined>(heading);
    const placed: string[] = [];
    vi.spyOn(Position, "attach").mockImplementation((_content, beside) => {
        placed.push(beside.dataset["slot"] ?? beside.tagName);

        return () => undefined;
    });
    onTestFinished(() => void vi.restoreAllMocks());
    const { container } = render(() => (
        <Popover defaultOpen>
            <PopoverTrigger>Open</PopoverTrigger>
            <PopoverContent anchor={anchor()}>Card</PopoverContent>
        </Popover>
    ));
    flush();
    setAnchor({ x: 10, y: 20, width: 30, height: 5 });
    flush();
    const box = container.querySelector<HTMLElement>("[data-slot=position-anchor]");
    const isHidden = box?.getAttribute("aria-hidden");
    const steps = [undefined, { x: 1, y: 2, width: 3, height: 4 }, heading, undefined].map(
        (next) => {
            setAnchor(() => next);
            flush();

            return placed.at(-1);
        },
    );

    expect({
        placed: placed.filter((each, i) => each !== placed[i - 1]).slice(0, 3),
        steps,
        isHidden,
        isGone: container.querySelector("[data-slot=position-anchor]") === null,
    }).toEqual({
        placed: ["popover-trigger", "H2", "position-anchor"],
        steps: ["popover-trigger", "position-anchor", "H2", "popover-trigger"],
        isHidden: "true",
        isGone: true,
    });
});

test("label a popover by its trigger's own id, such as a field's control", () => {
    // render the trigger as a button that keeps the id a field gives it
    const { container } = render(() => (
        <Popover>
            <PopoverTrigger
                render={(attributes) => (
                    <button type="button" {...attributes} id="paint-field">
                        Paint
                    </button>
                )}
            />
            <PopoverContent>Solid</PopoverContent>
        </Popover>
    ));
    const content = present(
        container.querySelector("[data-slot=popover-content]"),
        "the popover's content",
    );

    expect(content.getAttribute("aria-labelledby")).toBe("paint-field");
});
