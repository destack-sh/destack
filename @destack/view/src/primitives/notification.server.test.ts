import { expect, test } from "@destack/test";
import {
    createNotification,
    createNotificationPermission,
    isNotificationSupported,
    makeNotification,
} from "./notification.ts";

test("show no notifications on the server", () => {
    const notifier = createNotification("Saved");

    expect([
        isNotificationSupported(),
        makeNotification("Saved")[0](),
        notifier.show(),
        notifier.supported,
        createNotificationPermission().permission(),
    ]).toEqual([false, null, null, false, "unknown"]);
});
