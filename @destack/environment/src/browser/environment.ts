import { Broadcast } from "./broadcast/broadcast.ts";
import { Display } from "./display/display.ts";
import { Environment } from "../environment/environment.ts";
import type { BrowserCapability } from "./capability.ts";
import { BrowserError } from "./error.ts";
import { LockManager } from "./lock/lock.ts";
import { Navigation } from "./navigation/navigation.ts";
import { Storage } from "./storage/storage.ts";
import { WorkerHost } from "../worker/worker.ts";
import { Network } from "../network/network.ts";

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
            network: { ...Network.browser, origin: location.origin },
            workers: WorkerHost.system,
            storage: Storage.system,
            locks: LockManager.system,
            broadcast: Broadcast.system,
            navigation: Navigation.system,
            display: Display.system,
        };

        return system;
    },

    /** Build a server render's environment at a URL, refusing each browser capability. */
    server(base: Environment, url: string): BrowserEnvironment {
        return {
            ...base,
            navigation: Navigation.rendered(url),
            get storage(): never {
                return refuse("storage");
            },
            get locks(): never {
                return refuse("locks");
            },
            get broadcast(): never {
                return refuse("broadcast");
            },
            get display(): never {
                return refuse("display");
            },
        };
    },
};

/** Refuse a browser capability read in a server render. */
function refuse(capability: BrowserCapability): never {
    throw BrowserError.of(
        "CAPABILITY_BLOCKED",
        `cannot use browser '${capability}': a server render has no browser`,
    );
}
