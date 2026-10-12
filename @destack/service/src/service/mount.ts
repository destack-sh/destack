import { copyRequest } from "../request/request.ts";

/** The path at which an installation's origin serves its package's service. */
export const SERVICE_PATH = "/.destack/service";

/** The path a host mounts each package's service under. */
const MOUNTED = /^\/service\/([^/]+)(\/.*)?$/u;

/** The path a machine serves each space under. */
const SPACE = /^\/spaces\/([^/]+)(\/.*)?$/u;

/** The path where a machine takes calls to each installation it serves. */
const INSTALLED = /^\/spaces\/([^/]+)\/installations\/([^/]+)(\/.*)?$/u;

/** The paths hosts serve packages' services at, one mount per package. */
export const ServiceMount = {
    /** Build the path a package's service is mounted at. */
    path(packageId: string): `/service/${string}` {
        return `/service/${packageId}`;
    },

    /** Build the URL of a package's service on a host's origin. */
    url(origin: string, packageId: string): string {
        return new URL(ServiceMount.path(packageId), origin).href;
    },

    /** Split a path into the mounted package and the path below its mount. */
    parse(pathname: string): { readonly packageId: string; readonly path: string } | undefined {
        const [, packageId, path] = MOUNTED.exec(pathname) ?? [];

        return packageId === undefined ? undefined : { packageId, path: path ?? "/" };
    },

    /** Split a request into the mounted package and the request below its mount. */
    of(request: Request): { readonly packageId: string; readonly request: Request } | undefined {
        // find the mount the path selects
        const url = new URL(request.url);
        const mounted = ServiceMount.parse(url.pathname);
        if (mounted === undefined) {
            return undefined;
        }

        // keep the request below the mount
        url.pathname = mounted.path;

        return { packageId: mounted.packageId, request: copyRequest(request, {}, url.href) };
    },
};

/** The paths machines serve their spaces at, one per space. */
export const SpaceMount = {
    /** Build the path a space is served at. */
    path(spaceId: string): `/spaces/${string}` {
        return `/spaces/${spaceId}`;
    },

    /** Build the URL of a space on the endpoint of the machine serving it. */
    url(endpoint: string, spaceId: string): string {
        return new URL(SpaceMount.path(spaceId), endpoint).href;
    },

    /** Split a path into the space and the path below its mount, absent outside any space. */
    parse(pathname: string): { readonly spaceId: string; readonly path: string } | undefined {
        const [, spaceId, path] = SPACE.exec(pathname) ?? [];

        return spaceId === undefined ? undefined : { spaceId, path: path ?? "/" };
    },
};

/** The paths where machines take calls to their installations. */
export const InstallationMount = {
    /** Build the URL of an installation's service on its machine's endpoint. */
    url(endpoint: string, spaceId: string, installation: string): string {
        return new URL(`${SpaceMount.path(spaceId)}/installations/${installation}`, endpoint).href;
    },

    /** Split a request into the space, the installation and the request below its mount. */
    of(
        request: Request,
    ):
        | { readonly spaceId: string; readonly installation: string; readonly request: Request }
        | undefined {
        // find the space and the installation the path selects
        const url = new URL(request.url);
        const [, spaceId, installation, path] = INSTALLED.exec(url.pathname) ?? [];
        if (spaceId === undefined || installation === undefined) {
            return undefined;
        }

        // keep the request below the mount
        url.pathname = path ?? "/";

        return { spaceId, installation, request: copyRequest(request, {}, url.href) };
    },
};
