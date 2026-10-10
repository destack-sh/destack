import { type Identifier, schema } from "@destack/schema";
import { copyRequest } from "../request/request.ts";

/** The path below which a host takes a workload's calls to addresses. */
const EGRESS_PATH = "/.destack/egress";

/** The separator between an installation's identifier and its space's in its address. */
const INSTALLATION_SEPARATOR = "--";

/** An address followed by the path below it: a scoped package as two segments, an installation as one. */
const ADDRESSED = /^\/(@[^/]+\/[^/]+|[^/@][^/]*)(\/.*)?$/u;

/** The paths at which a host takes its workloads' calls: `<egress>/<address>/<path>`, the address an installation its space binds (`notes`), an installation in a space by identifier or alias (`<installation>--<space>`) or a package's service (`@destack/audit`). */
export const Egress = {
    /** The path of a host's egress on its loopback origin. */
    path: EGRESS_PATH,

    /** Write the URL at which a workload reaches an address through its host's egress. */
    url(egress: string, address: string): string {
        return `${egress.replace(/\/$/u, "")}/${address}`;
    },

    /** Write the URL of a host path beside an egress, below the same prefix, such as its journal. */
    beside(egress: string, path: string): string {
        // keep the prefix above the egress path
        const url = new URL(egress);
        const prefix = url.pathname.endsWith(EGRESS_PATH)
            ? url.pathname.slice(0, -EGRESS_PATH.length)
            : "";
        url.pathname = `${prefix}${path}`;

        return url.href;
    },

    /** Write the address of an installation in a space, by its identifier or its alias. */
    address(installationId: string, spaceId: string): string {
        return `${installationId}${INSTALLATION_SEPARATOR}${spaceId}`;
    },

    /** Read an installation's identifiers from its address, absent for any other address. */
    installation(address: string):
        | {
              readonly installationId: Identifier<"installation">;
              readonly spaceId: Identifier<"space">;
          }
        | undefined {
        // split the address into the two identifiers
        const [installationId, spaceId, ...rest] = address.split(INSTALLATION_SEPARATOR);
        const installation = schema.identifier("installation").safeParse(installationId);
        const space = schema.identifier("space").safeParse(spaceId);

        return installation.success && space.success && rest.length === 0
            ? { installationId: installation.data, spaceId: space.data }
            : undefined;
    },

    /** Split a request to a host's egress into the address and the request below it, absent outside the egress. */
    of(request: Request): { readonly address: string; readonly request: Request } | undefined {
        // require the egress path and an address below it
        const url = new URL(request.url);
        if (!url.pathname.startsWith(`${EGRESS_PATH}/`)) {
            return undefined;
        }
        const [, address, path] = ADDRESSED.exec(url.pathname.slice(EGRESS_PATH.length)) ?? [];
        if (address === undefined) {
            return undefined;
        }

        // keep the request below the address
        url.pathname = path ?? "/";

        return { address, request: copyRequest(request, {}, url.href) };
    },
};
