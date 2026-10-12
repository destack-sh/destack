import { Instant, schema } from "@destack/schema";

/** What a lease lets its holder do: read the resource, or write it as well. */
export const LeaseMode = schema.enum(["read", "write"]);
/** What a lease lets its holder do: read the resource, or write it as well. */
export type LeaseMode = schema.Infer<typeof LeaseMode>;

/** Short-lived, direct access to a resource's data. */
export const Lease = schema.object({
    /** Where the holder sends its requests: a presigned URL, or a Git remote. */
    url: schema.sensitive(schema.url()),
    /** What the holder may do. */
    mode: LeaseMode,
    /** The headers the holder sends with each request, such as its authorization. */
    headers: schema.sensitive(schema.record(schema.string(), schema.string())),
    /** The time the lease ends, in UTC epoch milliseconds. */
    expiresAt: Instant.exactOptional(),
});
/** Short-lived, direct access to a resource's data. */
export type Lease = schema.Infer<typeof Lease>;
