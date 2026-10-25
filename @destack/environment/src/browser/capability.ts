import type { Broadcast } from "./broadcast/broadcast.ts";
import type { Display } from "./display/display.ts";
import type { LockManager } from "./lock/lock.ts";
import type { Navigation } from "./navigation/navigation.ts";
import type { Storage } from "./storage/storage.ts";

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
        /** The browser's frames, idle periods and media features. */
        readonly display: Display;
    }
}

/** The capabilities only browsers have. */
export type BrowserCapability = "storage" | "locks" | "broadcast" | "navigation" | "display";
