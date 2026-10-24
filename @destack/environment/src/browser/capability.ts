import type { Broadcast } from "./broadcast/broadcast.ts";
import type { Capture } from "./capture/capture.ts";
import type { Clipboard } from "./clipboard/clipboard.ts";
import type { Connectivity } from "./connectivity/connectivity.ts";
import type { Display } from "./display/display.ts";
import type { Geolocation } from "./geolocation/geolocation.ts";
import type { LockManager } from "./lock/lock.ts";
import type { Navigation } from "./navigation/navigation.ts";
import type { Permissions } from "./permission/permission.ts";
import type { Share } from "./share/share.ts";
import type { Storage } from "./storage/storage.ts";
import type { Vibration } from "./vibration/vibration.ts";

/** The environment module, given the browser's capabilities in programs that run in browsers. */
declare module "../environment/environment.ts" {
    /** The effects some targets have beside the base, the browser's among them. */
    interface Capability {
        /** The browser's storage and its persistence. */
        readonly storage: Storage;
        /** The origin's named locks. */
        readonly locks: LockManager;
        /** The origin's broadcast channels. */
        readonly broadcast: Broadcast;
        /** The browser's session history. */
        readonly navigation: Navigation;
        /** The browser's clipboard. */
        readonly clipboard: Clipboard;
        /** The browser's frames, idle periods and media features. */
        readonly display: Display;
        /** Whether the browser reaches the network. */
        readonly connectivity: Connectivity;
        /** The browser's cameras, microphones and screens. */
        readonly capture: Capture;
        /** The browser's position. */
        readonly geolocation: Geolocation;
        /** The browser's share sheet. */
        readonly share: Share;
        /** The device's vibration motor. */
        readonly vibration: Vibration;
        /** The browser's permission states. */
        readonly permissions: Permissions;
    }
}

/** The capabilities only browsers have. */
export type BrowserCapability =
    | "storage"
    | "locks"
    | "broadcast"
    | "navigation"
    | "clipboard"
    | "display"
    | "connectivity"
    | "capture"
    | "geolocation"
    | "share"
    | "vibration"
    | "permissions";
