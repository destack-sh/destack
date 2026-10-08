import { afterEach, beforeEach, expect, test, vi } from "@destack/test";
import { createRoot, flush } from "solid-js";
import {
    createNotification,
    createNotificationPermission,
    isNotificationSupported,
    makeNotification,
} from "./notification.ts";

/** The notifications shown in the current test. */
let shown: StubNotification[] = [];

/** A stand-in notification that records being closed. */
class StubNotification extends EventTarget {
    /** The permission the person gave. */
    static permission: NotificationPermission = "granted";
    /** Ask for permission, granting it. */
    static async requestPermission(): Promise<NotificationPermission> {
        StubNotification.permission = "granted";

        return "granted";
    }
    /** The title. */
    readonly title: string;
    /** The options. */
    readonly options: NotificationOptions | undefined;
    /** Whether it was closed. */
    isClosed = false;

    /** Show the notification. */
    constructor(title: string, options?: NotificationOptions) {
        super();
        this.title = title;
        this.options = options;
        shown.push(this);
    }

    /** Close the notification, telling its listeners. */
    close(): void {
        this.isClosed = true;
        this.dispatchEvent(new Event("close"));
    }
}

beforeEach(() => {
    shown = [];
    StubNotification.permission = "granted";
    vi.stubGlobal("Notification", StubNotification);
    Object.defineProperty(window, "Notification", { configurable: true, value: StubNotification });
    Object.defineProperty(navigator, "permissions", {
        configurable: true,
        value: {
            query: async () =>
                Object.assign(new EventTarget(), {
                    state: "granted",
                    name: "notifications",
                    onchange: null,
                }),
        },
    });
});

afterEach(() => {
    vi.unstubAllGlobals();
    Reflect.deleteProperty(window, "Notification");
    Reflect.deleteProperty(navigator, "permissions");
});

test("show a notification in place of the last, and close it", () => {
    const [show, close] = makeNotification("Saved", { body: "Your note is saved" });
    show();
    show();
    close();

    expect([
        isNotificationSupported(),
        shown.map((item) => [item.title, item.options?.body, item.isClosed]),
    ]).toEqual([
        true,
        [
            ["Saved", "Your note is saved", true],
            ["Saved", "Your note is saved", true],
        ],
    ]);
});

test("refuse to show a notification without permission", () => {
    StubNotification.permission = "default";
    const [show] = makeNotification("Saved");

    expect(() => show()).toThrow(new TypeError("notifications are not yet permitted"));
});

test("follow the shown notification through its events, closing it on cleanup", () => {
    // show with handlers, click it, and let the system close it
    const seen: string[] = [];
    let title = "First";
    const observed = createRoot((disposeRoot) => {
        const notifier = createNotification(() => title, undefined, {
            onClick: (item) => seen.push(`click ${item.title}`),
            onClose: (item) => seen.push(`close ${item.title}`),
        });
        notifier.show();
        flush();
        const first = notifier.notification()?.title;
        shown[0]?.dispatchEvent(new Event("click"));
        shown[0]?.close();
        flush();
        const afterClose = notifier.notification();

        // show another and dispose
        title = "Second";
        notifier.show();
        disposeRoot();

        return { first, afterClose };
    });

    expect({ ...observed, seen, closed: shown.map((item) => item.isClosed) }).toEqual({
        first: "First",
        afterClose: null,
        seen: ["click First", "close First", "close Second"],
        closed: [true, true],
    });
});

test("ask for the notification permission as an action", async () => {
    StubNotification.permission = "default";
    const { requestPermission, pending, dispose } = createRoot((disposeRoot) => ({
        ...createNotificationPermission(),
        dispose: disposeRoot,
    }));
    await requestPermission();
    flush();
    dispose();

    expect([StubNotification.permission, pending()]).toEqual(["granted", false]);
});
