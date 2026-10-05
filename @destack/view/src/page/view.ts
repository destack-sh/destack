import { createContext, useContext } from "solid-js";
import type { ObjectClient } from "@destack/object/client";
import type { ViewScope } from "@destack/package/manifest";
import type { ObjectReference } from "@destack/sync";
import {
    OBJECT_PARAMETER,
    OPEN_PATH,
    VIEW_PARAMETER,
    type ViewContext,
} from "../declare/context.ts";

/** A mounted view: what its host gives it and the open clients of its scopes. */
export interface ViewMount {
    /** What the host gives the view. */
    readonly context: ViewContext;
    /** The clients of the scopes the view opens, by scope. */
    readonly clients: ReadonlyMap<string, ObjectClient>;
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
