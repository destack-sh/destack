import type { Lease, LeaseMode } from "@destack/resource";
import type { BucketHost, BucketReference } from "./host.ts";
import { BucketKey } from "../bucket/key.ts";
import { S3Location } from "./location.ts";
import { S3Signature } from "./signature.ts";

/** How long a presigned lease stays valid in seconds: fifteen minutes. */
export const LEASE_LIFETIME_SECONDS = 15 * 60;

/** The HTTP method a lease of each mode signs: GET reads a body, PUT writes one. */
const LEASE_METHODS: Readonly<Record<LeaseMode, "GET" | "PUT">> = { read: "GET", write: "PUT" };

/** One S3 request a presigned lease grants. */
export interface BucketLeaseRequest {
    /** What the lease lets its holder do. */
    readonly mode: LeaseMode;
    /** The file key. */
    readonly key: string;
    /** The headers the signature binds. */
    readonly headers: Headers;
    /** The query parameters selecting a multipart part. */
    readonly query?: Record<string, string>;
}

/** Presigned S3 requests on a host's buckets, the only way a caller reads a file's body. */
export const BucketLease = {
    /** Presign one S3 request on a bucket at a time, valid for the lease lifetime. */
    async presign(
        host: BucketHost,
        target: BucketReference,
        lease: BucketLeaseRequest,
        now: number,
    ): Promise<Lease> {
        // refuse a key the bucket refuses, then address it and the part the query selects
        BucketKey.check(lease.key);
        const { location, credentials } = await host.locate(target, lease.mode);
        const url = S3Location.url(location, lease.key);
        for (const [name, value] of Object.entries(lease.query ?? {})) {
            url.searchParams.set(name, value);
        }

        // refuse a URL that left the bucket's path, which only a malformed key causes
        const bucketPath = `${S3Location.url(location).pathname}/`;
        if (!url.pathname.startsWith(bucketPath)) {
            throw new TypeError(`key ${lease.key} is outside its bucket's path`);
        }

        // sign the request at the time, keeping the headers the caller sends
        const method = LEASE_METHODS[lease.mode];
        const request = new Request(url.href, { method, headers: lease.headers });
        const signature = new S3Signature({ region: location.region });
        const presigned = await signature.presign(
            request,
            credentials,
            LEASE_LIFETIME_SECONDS,
            now,
        );

        return {
            url: presigned.url,
            mode: lease.mode,
            headers: Object.fromEntries(presigned.headers),
            expiresAt: now + LEASE_LIFETIME_SECONDS * 1000,
        };
    },
};
