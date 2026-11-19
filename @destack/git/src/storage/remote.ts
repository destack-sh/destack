import type { Lease } from "@destack/resource";
import type { Fetch } from "@destack/object";
import { copyRequest } from "@destack/environment";

/** The request headers a forward passes on, as Git's smart HTTP protocol reads them. */
const FORWARDED_HEADERS = ["accept", "content-type", "content-encoding", "git-protocol"] as const;

/** Git remotes over the smart HTTP protocol, reached through leases. */
export const GitRemote = {
    /** Forward a smart HTTP request below a repository, such as `/info/refs`, to its remote. */
    forward(lease: Lease, path: string, request: Request, fetch: Fetch): Promise<Response> {
        // send Git's headers beside the lease's, streaming the body
        const headers = new Headers(lease.headers);
        for (const name of FORWARDED_HEADERS) {
            const value = request.headers.get(name);
            if (value !== null) {
                headers.set(name, value);
            }
        }
        const { search } = new URL(request.url);
        const url = `${lease.url.replace(/\/$/u, "")}${path}${search}`;

        return fetch(copyRequest(request, { headers }, url));
    },
};
