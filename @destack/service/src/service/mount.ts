import { copyRequest } from "../request/request.ts";

/** The path a host mounts each package's service under. */
const MOUNTED = /^\/service\/([^/]+)(\/.*)?$/u;

/** The path a cell takes calls to each installation it serves under. */
const INSTALLED = /^\/installation\/([^/]+)(\/.*)?$/u;

/** The paths hosts serve packages' services at, one mount per package. */
export const ServiceMount = {
    /** Name the path a package's service is mounted at. */
    path(packageId: string): `/service/${string}` {
        return `/service/${packageId}`;
    },

    /** Name the URL of a package's service on a host's origin. */
    url(origin: string, packageId: string): string {
        return new URL(ServiceMount.path(packageId), origin).href;
    },

    /** Split a request into the mounted package and the request below its mount, absent outside any mount. */
    route(request: Request): { readonly packageId: string; readonly request: Request } | undefined {
        // find the mount the path names
        const url = new URL(request.url);
        const [, packageId, path] = MOUNTED.exec(url.pathname) ?? [];
        if (packageId === undefined) {
            return undefined;
        }

        // keep the request below the mount
        url.pathname = path ?? "/";

        return { packageId, request: copyRequest(request, {}, url.href) };
    },
};

/** The paths cells take other spaces' calls to their installations at, one per installation. */
export const InstallationMount = {
    /** Name the URL of an installation's service on its cell's endpoint. */
    url(endpoint: string, installationId: string): string {
        return new URL(`/installation/${installationId}`, endpoint).href;
    },

    /** Split a request into the installation and the request below its mount, absent outside any mount. */
    route(
        request: Request,
    ): { readonly installationId: string; readonly request: Request } | undefined {
        // find the installation the path names
        const url = new URL(request.url);
        const [, installationId, path] = INSTALLED.exec(url.pathname) ?? [];
        if (installationId === undefined) {
            return undefined;
        }

        // keep the request below the mount
        url.pathname = path ?? "/";

        return { installationId, request: copyRequest(request, {}, url.href) };
    },
};
