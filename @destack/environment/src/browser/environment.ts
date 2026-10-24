import { Broadcast } from "./broadcast/broadcast.ts";
import { Capture } from "./capture/capture.ts";
import { Clipboard } from "./clipboard/clipboard.ts";
import { Connectivity } from "./connectivity/connectivity.ts";
import { Display } from "./display/display.ts";
import { Environment } from "../environment/environment.ts";
import type { BrowserCapability } from "./capability.ts";
import { Geolocation } from "./geolocation/geolocation.ts";
import { LockManager } from "./lock/lock.ts";
import { Navigation } from "./navigation/navigation.ts";
import { Permissions } from "./permission/permission.ts";
import { Share } from "./share/share.ts";
import { Storage } from "./storage/storage.ts";
import { Vibration } from "./vibration/vibration.ts";
import { Workers } from "../worker/worker.ts";

/** The browser's environment: the base and the browser's capabilities. */
export const BrowserEnvironment = {
    /** The browser's own effects. */
    system: {
        ...Environment.system,
        workers: Workers.system,
        storage: Storage.system,
        locks: LockManager.system,
        broadcast: Broadcast.system,
        navigation: Navigation.system,
        clipboard: Clipboard.system,
        display: Display.system,
        connectivity: Connectivity.system,
        capture: Capture.system,
        geolocation: Geolocation.system,
        share: Share.system,
        vibration: Vibration.system,
        permissions: Permissions.system,
    } satisfies Environment<"workers" | BrowserCapability>,
};
