import { isServer } from "@solidjs/web";
import {
    type Accessor,
    action,
    affects,
    createOptimistic,
    createSignal,
    onCleanup,
} from "solid-js";
import { createPermission } from "./permission.ts";
import { access, type MaybeAccessor } from "./utils.ts";

/** Handlers of a notification's events. */
export type NotificationEventHandlers = {
    /** Handle a click on the notification. */
    readonly onClick?: (notification: Notification) => void;
    /** Handle the notification closing, by the person, the system or `close`. */
    readonly onClose?: (notification: Notification) => void;
    /** Handle the notification failing to show. */
    readonly onError?: (notification: Notification) => void;
};

/** Report whether the device has the Notifications API. */
export function isNotificationSupported(): boolean {
    return !isServer && "Notification" in window;
}

/** Make functions that show a notification, replacing the last, and close it, showing nothing where the device has no notifications. */
export function makeNotification(
    title: string,
    options?: NotificationOptions,
): [show: () => Notification | null, close: () => void] {
    // show nothing where the device cannot
    if (!isNotificationSupported()) {
        return [() => null, () => {}];
    }

    // close the current notification, forgetting it
    let current: Notification | undefined;
    const forget = (): void => {
        current = undefined;
    };
    const close = (): void => {
        current?.removeEventListener("close", forget);
        current?.close();
        current = undefined;
    };

    // show a new one in place of the last, refusing without permission
    const show = (): Notification => {
        // replace the current notification
        requirePermission();
        close();
        const shown = new Notification(title, options);
        current = shown;
        shown.addEventListener("close", forget, { once: true });

        return shown;
    };

    return [show, close];
}

/** Show notifications reactively, reading the title and options on each show, following the shown one, closing it on cleanup. */
export function createNotification(
    title: MaybeAccessor<string>,
    options?: MaybeAccessor<NotificationOptions>,
    handlers?: NotificationEventHandlers,
): {
    show: () => Notification | null;
    close: () => void;
    notification: Accessor<Notification | null>;
    supported: boolean;
} {
    // show nothing where the device cannot
    const supported = isNotificationSupported();
    if (!supported) {
        return { show: () => null, close: () => {}, notification: () => null, supported };
    }

    // hold the shown notification and how to stop listening to it
    const [notification, setNotification] = createSignal<Notification | null>(null, {
        ownedWrite: true,
    });
    let current: Notification | null = null;
    let detach: (() => void) | undefined;
    const release = (shown: Notification): void => {
        // stop listening and forget the shown one
        detach?.();
        detach = undefined;
        current = null;
        setNotification(null);
        handlers?.onClose?.(shown);
    };

    // close the shown one, telling the handler
    const close = (): void => {
        const shown = current;
        if (shown !== null) {
            release(shown);
            shown.close();
        }
    };

    // show a new one in place of the last, refusing without permission, and listen to its events
    const show = (): Notification => {
        // replace the shown notification
        requirePermission();
        close();
        const shown = new Notification(access(title), access(options));
        current = shown;
        const listeners: [string, () => void][] = [
            ["close", () => current === shown && release(shown)],
            ["click", () => handlers?.onClick?.(shown)],
            ["error", () => handlers?.onError?.(shown)],
        ];
        for (const [name, listener] of listeners) {
            shown.addEventListener(name, listener);
        }
        detach = () => {
            for (const [name, listener] of listeners) {
                shown.removeEventListener(name, listener);
            }
        };
        setNotification(shown);

        return shown;
    };
    onCleanup(close);

    return { show, close, notification, supported };
}

/** Follow the notification permission and ask for it as an action, with whether the request is pending. */
export function createNotificationPermission(): {
    permission: Accessor<PermissionState | "unknown">;
    requestPermission: () => Promise<void>;
    pending: Accessor<boolean>;
} {
    // know nothing where the device has no notifications
    if (!isNotificationSupported()) {
        return {
            permission: () => "unknown",
            requestPermission: async () => {},
            pending: () => false,
        };
    }

    // ask as an action, marking the permission pending until the answer
    const permission = createPermission("notifications");
    const [pending, setPending] = createOptimistic(false, { ownedWrite: true });
    const requestPermission = action(function* () {
        setPending(true);
        affects(permission);
        try {
            yield Notification.requestPermission();
        } finally {
            setPending(false);
        }
    });

    return { permission, requestPermission, pending };
}

/** Refuse to show a notification the person has not permitted. */
function requirePermission(): void {
    if (Notification.permission !== "granted") {
        throw new TypeError(
            `notifications are ${Notification.permission === "denied" ? "denied" : "not yet permitted"}`,
        );
    }
}
