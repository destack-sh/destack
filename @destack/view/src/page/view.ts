import type { ObjectConnection, ObjectNode } from "@destack/object/client";
import { type Client, type Package, type ProcedureRouter, PackageMount } from "@destack/object";
import { type ClientOptions, createClient, type Endpoint } from "@destack/object/client";
import { ScopePath } from "../declare/context.ts";
import { ViewError } from "../error/error.ts";
import { useEnvironment, useMount } from "./context.ts";

/** Read how the mounted view calls a path below its page's origin with the page's credential. */
export function useOrigin(): (path: string) => Endpoint {
    return useMount().origin;
}

/** Read how the mounted view connects to a store below its page's origin, as the page. */
export function useConnect(): (path: string, scope: string) => ObjectConnection {
    return useMount().connect;
}

/** Read the open client of one of the mounted view's scopes. */
export function useClient(scope: string): ObjectNode {
    const client = useMount().clients.get(scope);
    if (client === undefined) {
        throw ViewError.of(
            "SCOPE_BLOCKED",
            `cannot open objects in '${scope}': the view does not open that scope`,
        );
    }

    return client;
}

/** Connect to a package the mounted view declares, refusing any other. */
export function usePackage<Definition extends ProcedureRouter>(
    handle: Package & { readonly router: Definition },
): Client<Definition> {
    // connect where the host mounted the declared package
    const { packages } = useMount();
    const environment = useEnvironment();
    const options = packages.get(handle.id);
    if (options === undefined) {
        throw ViewError.of(
            "PACKAGE_BLOCKED",
            `cannot use package '${handle.name}': the view does not declare it`,
        );
    }

    return createClient(handle, options, environment);
}

/** Mount the packages a view declares below its space's path on its origin. */
export function mountPackages(
    handles: readonly Package[],
    space: string,
    origin: (path: string) => ClientOptions,
): ReadonlyMap<string, ClientOptions> {
    return new Map(
        handles.map((handle) => [
            handle.id,
            origin(`${ScopePath.of(space)}${PackageMount.path(handle.id)}`),
        ]),
    );
}
