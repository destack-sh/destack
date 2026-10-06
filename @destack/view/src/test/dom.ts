import { onTestFinished } from "@destack/test";
import type { JSX } from "@solidjs/web";
import { render } from "@solidjs/web";
import type { Example } from "@destack/package/declare";
import { renderExample } from "../example/frame.ts";

/** The class attributes StyleX writes, whose names hash the styles. */
const CLASS_ATTRIBUTE = / class="[^"]*"/gu;

/** The empty comments Solid places around conditional content. */
const MARKER = /<!---->/gu;

/** The bodies of inline icons, which load on their own schedule. */
const ICON_BODY = /(<svg[^>]*>).*?(<\/svg>)/gu;

/** The ids Solid generates, numbered across every render in a test file. */
const GENERATED_ID = /cl-\d+/gu;

/** Render an element, or an example as its hosts render it, into a container in the document for the test's duration. */
export function draw<Properties extends object>(
    subject: (() => JSX.Element) | Example<Properties, JSX.Element>,
): HTMLElement {
    // mount into the document and unmount and remove after the test
    const container = document.createElement("div");
    document.body.append(container);
    onTestFinished(() => container.remove());
    onTestFinished(
        typeof subject === "function"
            ? render(subject, container)
            : renderExample(container, { example: subject }),
    );

    return container;
}

/** Press a key on the focused element, or on the body when nothing has the focus. */
export function press(key: string): void {
    const target = document.activeElement ?? document.body;
    target.dispatchEvent(new KeyboardEvent("keydown", { key, bubbles: true, cancelable: true }));
}

/** Return the text of the focused element. */
export function focused(): string {
    return document.activeElement?.textContent ?? "";
}

/** Return a container's markup without classes, markers and icon bodies, numbering generated ids by first use. */
export function markup(container: Element): string {
    // drop classes, markers and icon bodies
    const html = container.innerHTML
        .replaceAll(CLASS_ATTRIBUTE, "")
        .replaceAll(MARKER, "")
        .replaceAll(ICON_BODY, "$1$2");

    // number each generated id by its first use
    const ids = new Map<string, string>();

    return html.replaceAll(GENERATED_ID, (id) => {
        const numbered = ids.get(id) ?? `id-${ids.size + 1}`;
        ids.set(id, numbered);

        return numbered;
    });
}

/** Return the class attribute of each child of a container, in order. */
export function classes(container: Element): string[] {
    return [...container.children].map((child) => child.className);
}

/** Show and hide popovers by a `data-popover-open` attribute naming their anchor's slot, standing in for the Popover API the test DOM lacks. */
export function stubPopovers(): void {
    // keep the prototype's own members to restore
    const prototype = HTMLElement.prototype;
    const show = Object.getOwnPropertyDescriptor(prototype, "showPopover");
    const hide = Object.getOwnPropertyDescriptor(prototype, "hidePopover");

    // mark a shown popover with the slot of the element it anchors to, firing its toggle
    Object.defineProperty(prototype, "showPopover", {
        configurable: true,
        value(this: HTMLElement, options?: ShowPopoverOptions) {
            if (!this.hasAttribute("data-popover-open")) {
                this.setAttribute("data-popover-open", String(options?.source?.dataset["slot"]));
                toggle(this, "open");
            }
        },
    });
    Object.defineProperty(prototype, "hidePopover", {
        configurable: true,
        value(this: HTMLElement) {
            if (this.hasAttribute("data-popover-open")) {
                this.removeAttribute("data-popover-open");
                toggle(this, "closed");
            }
        },
    });

    // toggle the popover a clicked button targets, as its default action
    document.addEventListener("click", toggleTarget);

    // restore the prototype and the document after the test
    onTestFinished(() => {
        restore(prototype, "showPopover", show);
        restore(prototype, "hidePopover", hide);
        document.removeEventListener("click", toggleTarget);
    });
}

/** Toggle the popover a clicked button targets, unless a handler prevented the click. */
function toggleTarget(event: Event): void {
    const button = event.target instanceof Element ? event.target.closest("[popovertarget]") : null;
    const popover = document.getElementById(button?.getAttribute("popovertarget") ?? "");
    if (event.defaultPrevented || !(button instanceof HTMLElement) || popover === null) {
        return;
    } else if (popover.hasAttribute("data-popover-open")) {
        popover.hidePopover();
    } else {
        popover.showPopover({ source: button });
    }
}

/** Fire the toggle event a popover fires as it opens or closes. */
function toggle(popover: HTMLElement, newState: "open" | "closed"): void {
    const event = new Event("toggle");
    Object.assign(event, { oldState: newState === "open" ? "closed" : "open", newState });
    popover.dispatchEvent(event);
}

/** Put back a member of a prototype, or remove it when the prototype had none. */
function restore(
    prototype: object,
    name: string,
    descriptor: PropertyDescriptor | undefined,
): void {
    if (descriptor === undefined) {
        Reflect.deleteProperty(prototype, name);
    } else {
        Object.defineProperty(prototype, name, descriptor);
    }
}

/** Wait for timers to run, such as a popover's open delay. */
export function wait(milliseconds: number): Promise<void> {
    return new Promise((resolve) => {
        setTimeout(resolve, milliseconds);
    });
}
