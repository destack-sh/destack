/** The provider code of external resources: the host's egress, reaching each at its identifier. */
export const EGRESS_PROVIDER = "egress";

/** A client of an external resource, called through the host's egress below its identifier. */
export interface ExternalClient {
    /** The resource's address below the host's egress that calls go to. */
    readonly url: string;
    /** Call a path below the resource, the egress applying what the resource needs. */
    readonly fetch: (path: string, init?: RequestInit) => Promise<Response>;
}
