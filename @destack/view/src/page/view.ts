import { createContext, useContext } from "solid-js";
import type { ObjectClient } from "@destack/object/client";
import { type PermissionScope } from "@destack/access";
import type { ObjectReference } from "@destack/sync";
import {
    type Client,
    type PackageHandle,
    type ProcedureRouter,
    PackageMount,
} from "@destack/runtime";
import { type ClientOptions, createClient } from "@destack/runtime/client";
import {
    OBJECT_PARAMETER,
    OPEN_PATH,
    PLATFORM_PATH,
    VIEW_PARAMETER,
    type ViewContext,
} from "../declare/context.ts";

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

/** The mounted view. */
export const ViewMountContext = createContext<ViewMount>();

/** Read what the host gives the mounted view. */
export function useView(): ViewContext {
    return useContext(ViewMountContext).context;
}

/** Read the identifier of one of a view's scopes, refusing a home the host opens none for. */
export function requireScope(context: ViewContext, scope: PermissionScope): string {
    // require a scope the host opens
    const found = context[scope];
    if (found === undefined) {
        throw new TypeError(`the host does not open ${scope} for this view`);
    }

    return found;
}

/** Read how the mounted view calls a path below its page's origin with the page's credential. */
export function useOrigin(): (path: string) => ClientOptions {
    return useContext(ViewMountContext).origin;
}

/** Read the open client of one of the mounted view's scopes. */
export function useClient(scope: string): ObjectClient {
    const client = useContext(ViewMountContext).clients.get(scope);
    if (client === undefined) {
        throw new TypeError(`the view does not open objects in ${scope}`);
    }

    return client;
}

/** Connect to a package the mounted view declares, refusing any other. */
export function usePackage<Definition extends ProcedureRouter>(
    handle: PackageHandle & { readonly router: Definition },
): Client<Definition> {
    // connect where the host mounted the declared package
    const options = useContext(ViewMountContext).packages.get(handle.id);
    if (options === undefined) {
        throw new TypeError(`the view does not declare package ${handle.name}`);
    }

    return createClient(handle, options);
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

/** Write the same-origin address where the host opens an object. */
export function objectUrl(
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
