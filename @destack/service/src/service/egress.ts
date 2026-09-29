import { copyRequest } from "../request/request.ts";

/** The path below which a host takes a workload's calls to addresses. */
const EGRESS_PATH = "/.destack/egress";

/** An address followed by the path below it: a scoped package as two segments, an installation as one. */
const ADDRESSED = /^\/(@[^/]+\/[^/]+|[^/@][^/]*)(\/.*)?$/;

/** The paths at which a host takes its workloads' calls: `<egress>/<address>/<path>`, the address an installation (`notes`, `notes.work.acme`) or a package's service (`@destack/audit`). */
export const Egress = {
    /** The path of a host's egress on its loopback origin. */
    path: EGRESS_PATH,

    /** Write the URL at which a workload reaches an address through its host's egress. */
    url(egress: string, address: string): string {
        return `${egress.replace(/\/$/, "")}/${address}`;
    },

    /** Split a request to a host's egress into the address and the request below it, absent outside the egress. */
    route(request: Request): { readonly address: string; readonly request: Request } | undefined {
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
