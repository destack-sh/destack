import { type Accessor, createContext, NotReadyError, useContext } from "solid-js";
import type { BrowserEnvironment } from "@destack/environment/browser";
import type { TelemetryScope } from "@destack/telemetry";
import type { ClientOptions, Endpoint, ObjectNode, ObjectConnection } from "@destack/object/client";
import type { ViewContext } from "../declare/context.ts";
import type { ActionRegistry } from "./action.ts";

/** A mounted view. */
export interface ViewMount {
    /** What the host gives the view. */
    readonly context: ViewContext;
    /** The clients of the scopes the view opens, by scope: the server's, then the device's. */
    readonly clients: Accessor<ReadonlyMap<string, ObjectNode>>;
    /** Open another space with the objects the view opens in spaces, until the signal aborts. */
    readonly open: (scope: string, signal: AbortSignal) => Promise<ObjectNode>;
    /** How the view calls the packages it declares, by package identifier. */
    readonly packages: ReadonlyMap<string, ClientOptions>;
    /** Address a path below the page's origin with the page's credential. */
    readonly origin: (path: string) => Endpoint;
    /** Connect to a scope at the store below a path of the page's origin, as the page. */
    readonly connect: (path: string, scope: string) => ObjectConnection;
}

/** What an unmounted view's reads wait on: a mount that never comes, as at build. */
const UNMOUNTED = new Promise<never>(() => {});

/** The mounted view, null until a host mounts it. */
export const ViewMountContext = createContext<ViewMount | null>(null);

/** The environment views and components run in, which every render provides. */
export const EnvironmentContext = createContext<BrowserEnvironment>();

/** The actions a page's components handle while they are mounted, which every render provides. */
export const ActionRegistryContext = createContext<ActionRegistry>();

/** Read the environment the rendering runs in. */
export function useEnvironment(): BrowserEnvironment {
    return useContext(EnvironmentContext);
}

/** The telemetry of the package rendering, on the server and in the browser alike. */
export const TelemetryContext = createContext<TelemetryScope>();

/** Read the telemetry of the package rendering. */
export function useTelemetry(): TelemetryScope {
    return useContext(TelemetryContext);
}

/** Read the mounted view, suspending until a host mounts it, as prerenders run before any. */
export function useMount(): ViewMount {
    // suspend to the nearest loading boundary while no host mounted the view
    const mount = useContext(ViewMountContext);
    if (mount === null) {
        throw new NotReadyError(UNMOUNTED);
    }

    return mount;
}

/** Read what the host gives the mounted view. */
export function useView(): ViewContext {
    return useMount().context;
}
