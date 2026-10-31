import { createContext, useContext } from "solid-js";
import type { BrowserEnvironment } from "@destack/environment/browser";
import type { TelemetryScope } from "@destack/telemetry";
import type { ClientOptions, ObjectClient } from "@destack/object/client";
import type { ViewContext } from "../declare/context.ts";

/** A mounted view. */
export interface ViewMount {
    /** What the host gives the view. */
    readonly context: ViewContext;
    /** The clients of the scopes the view opens, by scope. */
    readonly clients: ReadonlyMap<string, ObjectClient>;
    /** How the view calls the packages it declares, by package identifier. */
    readonly packages: ReadonlyMap<string, ClientOptions>;
    /** Address a path below the page's origin with the page's credential. */
    readonly origin: (path: string) => ClientOptions;
}

/** The mounted view. */
export const ViewMountContext = createContext<ViewMount>();

/** The environment views and components run in, which every render provides. */
export const EnvironmentContext = createContext<BrowserEnvironment>();

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

/** Read what the host gives the mounted view. */
export function useView(): ViewContext {
    return useContext(ViewMountContext).context;
}
