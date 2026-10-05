import { createContext, useContext } from "solid-js";
import type { ObjectClient } from "@destack/object/client";
import type { ViewScope } from "@destack/package/manifest";
import type { ObjectReference } from "@destack/sync";
import { type Client, type Service, type ServiceRouter, ServiceMount } from "@destack/service";
import { type ClientOptions, createClient } from "@destack/service/client";
import {
    OBJECT_PARAMETER,
    OPEN_PATH,
    PLATFORM_PATH,
    VIEW_PARAMETER,
    type ViewContext,
} from "../declare/context.ts";

/** A mounted view: what its host gives it, the open clients of its scopes and the connections of the platform services it calls. */
export interface ViewMount {
    /** What the host gives the view. */
    readonly context: ViewContext;
    /** The clients of the scopes the view opens, by scope. */
    readonly clients: ReadonlyMap<string, ObjectClient>;
    /** How the view reaches the platform services it declares, by package. */
    readonly services: ReadonlyMap<string, ClientOptions>;
}

/** The mounted view. */
export const ViewMountContext = createContext<ViewMount>();

/** Read what the host gives the mounted view. */
export function useView(): ViewContext {
    return useContext(ViewMountContext).context;
}

/** Read the identifier of one of a view's scopes, refusing a home the host opens none for. */
export function scopeOf(context: ViewContext, scope: ViewScope): string {
    // require a scope the host opens
    const found = context[scope];
    if (found === undefined) {
        throw new TypeError(`the host opens no ${scope} for this view`);
    }

    return found;
}

/** Read the open client of one of the mounted view's scopes. */
export function useClient(scope: string): ObjectClient {
    const client = useContext(ViewMountContext).clients.get(scope);
    if (client === undefined) {
        throw new TypeError(`the view opens no objects in ${scope}`);
    }

    return client;
}

/** Connect to a platform service the mounted view declares, refusing one it does not declare. */
export function useService<Definition extends ServiceRouter>(
    service: Service<Definition>,
): Client<Definition> {
    // connect where the host mounted the declared service
    const options = useContext(ViewMountContext).services.get(service.package.id);
    if (options === undefined) {
        throw new TypeError(`the view declares no ${service.name} service`);
    }

    return createClient(service, options);
}

/** Mount the platform services a view declares below the platform path of its origin, by package. */
export function mountServices(
    services: readonly Pick<Service, "package">[],
    origin: string,
): ReadonlyMap<string, ClientOptions> {
    return new Map(
        services.map((service) => {
            const path = `${PLATFORM_PATH}${ServiceMount.path(service.package.id)}`;

            return [service.package.id, { url: new URL(path, origin).href }];
        }),
    );
}

/** Write the same-origin address at which the host opens an object in the view presenting it, or in the view asked for. */
export function urlOf(
    reference: ObjectReference,
    options: { readonly view?: string } = {},
): string {
    // name the object, and the view when asked for
    const query = new URLSearchParams({ [OBJECT_PARAMETER]: JSON.stringify(reference) });
    if (options.view !== undefined) {
        query.set(VIEW_PARAMETER, options.view);
    }

    return `${OPEN_PATH}?${query.toString()}`;
}
