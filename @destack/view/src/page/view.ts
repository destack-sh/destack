import {
    type Accessor,
    createContext,
    createSignal,
    onCleanup,
    sharedConfig,
    useContext,
} from "solid-js";
import { isServer } from "@solidjs/web";
import type { BrowserEnvironment } from "@destack/environment/browser";
import type { TelemetryScope } from "@destack/telemetry";
import { Duration } from "@destack/schema";
import type { ObjectClient } from "@destack/object/client";
import {
    type Client,
    type PackageHandle,
    type ProcedureRouter,
    PackageMount,
} from "@destack/object";
import { type ClientOptions, createClient } from "@destack/object/client";
import { PLATFORM_PATH, type ViewContext } from "../declare/context.ts";
import { ViewError } from "../error/error.ts";

/** A mounted view. */
export interface ViewMount {
    /** What the host gives the view. */
    readonly context: ViewContext;
    /** The clients of the scopes the view opens, by scope. */
    readonly clients: ReadonlyMap<string, ObjectClient>;
    /** How the view calls the packages it declares, by package identifier. */
    readonly packages: ReadonlyMap<string, ClientOptions>;
    /** Reach a path below the page's origin with the page's credential. */
    readonly origin: (path: string) => ClientOptions;
}

/** How often the time a view reads moves on: each second, finer than any relative time it shows. */
const NOW_INTERVAL: Duration = { seconds: 1 };

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

/** Read the time on the view's clock once mounted, absent before, moving on at each interval. */
export function useNow(interval: Duration = NOW_INTERVAL): Accessor<number | undefined> {
    // read no time on the server and while hydrating, so both render the same, else the clock's
    const { clock } = useEnvironment();
    const isDeferred = isServer || sharedConfig.hydrating;
    const [now, setNow] = createSignal<number | undefined>(isDeferred ? undefined : clock.now(), {
        ownedWrite: true,
    });
    if (isServer) {
        return now;
    }

    // read a deferred time a task after mounting, once hydration completed, then at each interval
    const first = isDeferred ? clock.setTimeout(() => setNow(clock.now()), 0) : undefined;
    const handle = clock.setInterval(() => setNow(clock.now()), Duration.milliseconds(interval));
    onCleanup(() => {
        clock.clearTimeout(first);
        clock.clearInterval(handle);
    });

    return now;
}

/** Read how the mounted view calls a path below its page's origin with the page's credential. */
export function useOrigin(): (path: string) => ClientOptions {
    return useContext(ViewMountContext).origin;
}

/** Read the open client of one of the mounted view's scopes. */
export function useClient(scope: string): ObjectClient {
    const client = useContext(ViewMountContext).clients.get(scope);
    if (client === undefined) {
        throw ViewError.of("SCOPE_NOT_OPENED", `the view does not open objects in ${scope}`);
    }

    return client;
}

/** Connect to a package the mounted view declares, refusing any other. */
export function usePackage<Definition extends ProcedureRouter>(
    handle: PackageHandle & { readonly router: Definition },
): Client<Definition> {
    // connect where the host mounted the declared package
    const { packages } = useContext(ViewMountContext);
    const environment = useEnvironment();
    const options = packages.get(handle.id);
    if (options === undefined) {
        throw ViewError.of(
            "PACKAGE_NOT_DECLARED",
            `the view does not declare package ${handle.name}`,
        );
    }

    return createClient(handle, options, environment);
}

/** Mount the packages a view declares below the platform path of its origin. */
export function mountPackages(
    handles: readonly PackageHandle[],
    origin: (path: string) => ClientOptions,
): ReadonlyMap<string, ClientOptions> {
    return new Map(
        handles.map((handle) => [
            handle.id,
            origin(`${PLATFORM_PATH}${PackageMount.path(handle.id)}`),
        ]),
    );
}
