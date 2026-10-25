import type { BrowserEnvironment } from "@destack/environment/browser";
import { setContext } from "@solidjs/signals";
import { createComponent, createRoot } from "solid-js";
import { EnvironmentContext } from "../page/view.ts";
import { onTestFinished } from "@destack/test";
import type { JSX } from "@solidjs/web";
import * as web from "@solidjs/web";
import type { Example } from "@destack/package/declare";
import { present } from "@destack/schema";
import { renderExample } from "../example/frame.ts";
import { testBrowser, testWorld } from "./browser.ts";

/** The class attributes StyleX writes, whose names hash the styles. */
const CLASS_ATTRIBUTE = / class="[^"]*"/gu;

/** The empty comments Solid places around conditional content. */
const MARKER = /<!---->/gu;

/** The bodies of inline icons, which load on their own schedule. */
const ICON_BODY = /(<svg[^>]*>).*?(<\/svg>)/gu;

/** The ids Solid generates, numbered across every render in a test file. */
const GENERATED_ID = /cl-\d+/gu;

/** How `render` mounts. */
export interface RenderOptions {
    /** The element to render into, a new one in the document by default. */
    readonly container?: HTMLElement;
    /** Hydrate the server's HTML the container holds instead of rendering anew. */
    readonly hydrate?: boolean;
    /** The environment an element renders in, the test's simulated tab by default. */
    readonly environment?: BrowserEnvironment;
}

/** A rendered element: its container, and how to unmount it before the test ends. */
export interface RenderResult {
    /** The element rendered into. */
    readonly container: HTMLElement;
    /** Unmount the element, which happens after the test otherwise. */
    readonly unmount: () => void;
}

/** The bootstrap the document's hydration script installs before a page hydrates. */
interface HydrationBootstrap {
    /** The events replayed once their elements hydrate. */
    readonly events: unknown[];
    /** The elements already hydrated. */
    readonly completed: WeakSet<object>;
    /** The resources the server streamed. */
    readonly r: Record<string, unknown>;
    /** Mark a hydration step. */
    readonly fe: () => void;
}

/** Render an element, or an example as its hosts render it, into a container in the document. */
export function render<Properties extends object>(
    ui: (() => JSX.Element) | Example<Properties, JSX.Element>,
    options: RenderOptions = {},
): RenderResult {
    // take the given container, or a new one in the document
    const container = options.container ?? document.createElement("div");
    if (!container.isConnected) {
        document.body.append(container);
    }

    // render an element under the given environment, the test's simulated tab by default
    const environment = options.environment ?? testBrowser();
    const element =
        typeof ui !== "function"
            ? ui
            : () =>
                  createComponent(EnvironmentContext, {
                      value: environment,
                      get children() {
                          return ui();
                      },
                  });

    // hydrate the server's HTML under the bootstrap its document installs, or render anew
    let dispose: () => void;
    if (options.hydrate === true) {
        if (typeof element !== "function") {
            throw new TypeError("an example renders anew, so it cannot hydrate");
        }
        const bootstrap: HydrationBootstrap = {
            events: [],
            completed: new WeakSet(),
            r: {},
            fe() {},
        };
        Reflect.set(globalThis, "_$HY", bootstrap);
        const disposeHydration = web.hydrate(element, container);
        dispose = () => {
            disposeHydration();
            Reflect.deleteProperty(globalThis, "_$HY");
        };
    } else if (typeof element === "function") {
        dispose = web.render(element, container);
    } else {
        dispose = renderExample(container, { example: element }, environment);
    }

    // unmount and remove once, by hand or after the test
    let isMounted = true;
    const unmount = (): void => {
        if (isMounted) {
            isMounted = false;
            dispose();
            container.remove();
        }
    };
    onTestFinished(unmount);

    return { container, unmount };
}

/** A hook run under an environment, as Solid Testing Library's `renderHook` answers it. */
export interface HookResult<Result> {
    /** What the hook returned. */
    readonly result: Result;
    /** Dispose the hook's owner, which happens after the test otherwise. */
    readonly cleanup: () => void;
}

/** Run a hook under an environment, the DOM's browser by default, disposing it after the test. */
export function renderHook<Result>(
    hook: () => Result,
    options: Pick<RenderOptions, "environment"> = {},
): HookResult<Result> {
    // run the hook in a page root of its own
    const { result, dispose } = createPageRoot(
        (disposeRoot) => ({ result: hook(), dispose: disposeRoot }),
        options.environment,
    );

    // dispose once, by hand or after the test
    let isRunning = true;
    const cleanup = (): void => {
        if (isRunning) {
            isRunning = false;
            dispose();
        }
    };
    onTestFinished(cleanup);

    return { result, cleanup };
}

/** Run a function in a root under an environment, as Solid's `createRoot` does. */
export function createPageRoot<Result>(
    run: (dispose: () => void) => Result,
    environment: BrowserEnvironment = testBrowser(),
): Result {
    return createRoot((dispose) => {
        setContext(EnvironmentContext, environment);

        return run(dispose);
    });
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

/** Return a container's markup without classes, markers and icon bodies. */
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

/** Show and hide popovers by a `data-popover-open` attribute with their anchor's slot. */
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

/** Wait until a check passes after a change within an element. */
export async function changed(
    element: Element,
    check: () => boolean,
    milliseconds: number,
): Promise<void> {
    // answer a check that passes already
    if (check()) {
        return;
    }

    // watch the element in its own window
    const view = present(element.ownerDocument.defaultView, "the element's window");
    const { promise, resolve, reject } = Promise.withResolvers<void>();
    const observer = new view.MutationObserver(() => {
        if (check()) {
            resolve();
        }
    });
    observer.observe(element, {
        subtree: true,
        childList: true,
        attributes: true,
        characterData: true,
    });

    // give up once the time passes on the test's clock, moving time until either happens
    const { simulation, tab } = testWorld();
    const { clock } = tab;
    const timer = clock.setTimeout(
        () => reject(new TypeError(`the element never changed within ${milliseconds} ms`)),
        milliseconds,
    );
    try {
        await simulation.drive(promise);
    } finally {
        observer.disconnect();
        clock.clearTimeout(timer);
    }
}

/** Move the test's clock to the tab's next frame, running what the frame starts. */
export async function frame(): Promise<void> {
    const { simulation, tab } = testWorld();
    await simulation.drive(tab.display.frame());
}

/** Move the test's clock on, running the timers due meanwhile, such as a popover's open delay. */
export function wait(milliseconds: number): Promise<void> {
    const { simulation } = testWorld();

    return simulation.run({ to: simulation.clock.now() + milliseconds });
}
