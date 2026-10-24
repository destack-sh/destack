import { Broadcast } from "./broadcast/broadcast.ts";
import { Capture } from "./capture/capture.ts";
import { Clipboard } from "./clipboard/clipboard.ts";
import { Connectivity } from "./connectivity/connectivity.ts";
import { Display } from "./display/display.ts";
import { Environment } from "../environment/environment.ts";
import type { BrowserCapability } from "./capability.ts";
import { BrowserError } from "./error.ts";
import { Geolocation } from "./geolocation/geolocation.ts";
import { LockManager } from "./lock/lock.ts";
import { Navigation } from "./navigation/navigation.ts";
import { Permissions } from "./permission/permission.ts";
import { Share } from "./share/share.ts";
import { Storage } from "./storage/storage.ts";
import { Vibration } from "./vibration/vibration.ts";
import { Workers } from "../worker/worker.ts";

/** The browser's environment: the base and the browser's capabilities. */
export type BrowserEnvironment = Environment<BrowserCapability>;

/** The browser's own environment, built on its first read. */
let system: Environment<"workers" | BrowserCapability> | undefined;

/** Browser environments. */
export const BrowserEnvironment = {
    /** The browser's own effects. */
    get system(): Environment<"workers" | BrowserCapability> {
        system ??= {
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
        };

        return system;
    },

    /** Build a server render's environment: the server's base, refusing each browser capability. */
    server(base: Environment): BrowserEnvironment {
        return {
            ...base,
            get storage(): never {
                return refuse("storage");
            },
            get locks(): never {
                return refuse("locks");
            },
            get broadcast(): never {
                return refuse("broadcast");
            },
            get navigation(): never {
                return refuse("navigation");
            },
            get clipboard(): never {
                return refuse("clipboard");
            },
            get display(): never {
                return refuse("display");
            },
            get connectivity(): never {
                return refuse("connectivity");
            },
            get capture(): never {
                return refuse("capture");
            },
            get geolocation(): never {
                return refuse("geolocation");
            },
            get share(): never {
                return refuse("share");
            },
            get vibration(): never {
                return refuse("vibration");
            },
            get permissions(): never {
                return refuse("permissions");
            },
        };
    },
};

/** Refuse a browser capability read in a server render. */
function refuse(capability: BrowserCapability): never {
    throw BrowserError.of("CAPABILITY_UNAVAILABLE", `a server render has no browser ${capability}`);
}
