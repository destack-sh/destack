import { afterEach, beforeEach, expect, test } from "@destack/test";
import { createRoot, flush } from "solid-js";
import { createFullscreen, fullscreen, makeFullscreen } from "./fullscreen.ts";

/** The element the stand-in shows in fullscreen. */
const shown: { current: Element | null } = { current: null };

/** The options it was last asked to show with. */
let lastOptions: FullscreenOptions | undefined;

beforeEach(() => {
    shown.current = null;
    lastOptions = undefined;
    Object.defineProperty(document, "fullscreenElement", {
        configurable: true,
        get: () => shown.current,
    });
    Object.defineProperty(document, "exitFullscreen", {
        configurable: true,
        value: async () => {
            shown.current = null;
            document.dispatchEvent(new Event("fullscreenchange"));
        },
    });
    Object.defineProperty(HTMLElement.prototype, "requestFullscreen", {
        configurable: true,
        value: async function (this: HTMLElement, options?: FullscreenOptions) {
            shown.current = this;
            lastOptions = options;
            document.dispatchEvent(new Event("fullscreenchange"));
        },
    });
});

afterEach(() => {
    Reflect.deleteProperty(document, "fullscreenElement");
    Reflect.deleteProperty(document, "exitFullscreen");
    Reflect.deleteProperty(HTMLElement.prototype, "requestFullscreen");
});

test("enter fullscreen with the made options unless the call overrides them, and exit it", async () => {
    const element = document.createElement("div");
    const [enter, exit] = makeFullscreen(element, { navigationUI: "hide" });
    await enter();
    const made = [shown.current === element, lastOptions];
    await enter({ navigationUI: "show" });
    const overridden = lastOptions;
    await exit();

    expect([made, overridden, shown.current]).toEqual([
        [true, { navigationUI: "hide" }],
        { navigationUI: "show" },
        null,
    ]);
});

test("follow whether the element is fullscreen, from the start and as it changes", async () => {
    // start with the element already fullscreen
    const element = document.createElement("div");
    shown.current = element;
    const opened = createRoot((disposeRoot) => ({
        ...createFullscreen(element),
        dispose: disposeRoot,
    }));
    const initial = opened.isActive();

    // exit, enter, and let the system dismiss it
    await opened.exit();
    flush();
    const exited = opened.isActive();
    await opened.enter();
    flush();
    const entered = opened.isActive();
    shown.current = null;
    document.dispatchEvent(new Event("fullscreenchange"));
    flush();
    const dismissed = opened.isActive();
    opened.dispose();

    expect([initial, exited, entered, dismissed]).toEqual([true, false, true, false]);
});

test("exit fullscreen with the owner unless told not to", async () => {
    const element = document.createElement("div");
    const closes: boolean[] = [];
    for (const exitOnCleanup of [undefined, false]) {
        const opened = createRoot((disposeRoot) => ({
            ...createFullscreen(element, exitOnCleanup === undefined ? {} : { exitOnCleanup }),
            dispose: disposeRoot,
        }));
        await opened.enter();
        opened.dispose();
        await Promise.resolve();
        closes.push(shown.current === element);
        shown.current = null;
    }

    expect(closes).toEqual([false, true]);
});

test("refuse to enter fullscreen while an accessor gives no element", async () => {
    const opened = createRoot((disposeRoot) => ({
        ...createFullscreen(() => undefined),
        dispose: disposeRoot,
    }));
    const refused = await opened.enter().catch((error: unknown) => error);
    opened.dispose();

    expect(refused).toEqual(new TypeError("there is no element to show in fullscreen"));
});

test("toggle fullscreen on each click of a ref's element, until disposed", async () => {
    const element = document.createElement("div");
    const states: boolean[] = [];
    const dispose = createRoot((disposeRoot) => {
        fullscreen({ navigationUI: "hide" })(element);

        return disposeRoot;
    });
    for (let click = 0; click < 2; click++) {
        element.click();
        await Promise.resolve();
        states.push(shown.current === element);
    }
    const options = lastOptions;
    dispose();
    element.click();
    await Promise.resolve();

    expect([states, options, shown.current]).toEqual([
        [true, false],
        { navigationUI: "hide" },
        null,
    ]);
});
