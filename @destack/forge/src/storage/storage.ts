import type { Lease, LeaseMode } from "@destack/resource";
import { Instant, schema } from "@destack/schema";

/** A Git object name: 40 hexadecimal digits for SHA-1, 64 for SHA-256. */
const ObjectName = schema.string().regex(/^(?:[0-9a-f]{40}|[0-9a-f]{64})$/u);

/** A branch or tag as its origin lists it. */
export const GitReference = schema.object({
    /** The complete name, such as refs/heads/main or refs/tags/v1. */
    name: schema.string().regex(/^refs\/(?:heads|tags)\/./u),
    /** The object the reference names directly: a commit, or a tag object for annotated tags. */
    object: ObjectName,
    /** The commit the reference resolves to, null when it resolves to no commit. */
    commit: ObjectName.nullable(),
});
/** A branch or tag as its origin lists it. */
export type GitReference = schema.Infer<typeof GitReference>;

/** Every branch and tag of a repository, and the existing branch its HEAD names. */
export const GitListing = schema.object({
    /** The branch HEAD names, including refs/heads/, null when HEAD names no existing branch. */
    defaultReference: schema
        .string()
        .regex(/^refs\/heads\/./u)
        .nullable(),
    /** The branches and tags. */
    references: schema.array(GitReference),
});
/** Every branch and tag of a repository, and the existing branch its HEAD names. */
export type GitListing = schema.Infer<typeof GitListing>;

/** A short-lived HTTP basic credential for Git over HTTPS. */
const credential = schema.object({
    /** The basic authentication user name. */
    username: schema.string().min(1),
    /** The basic authentication password, such as a token. */
    password: schema.sensitive(schema.string().min(1)),
    /** The expiry in UTC epoch milliseconds. */
    expiresAt: Instant,
});
/** A short-lived HTTP basic credential for Git over HTTPS. */
export type GitCredential = schema.Infer<typeof credential>;

/** A short-lived HTTP basic credential for Git over HTTPS, and the lease it grants on a remote. */
export const GitCredential = Object.assign(credential, {
    /** Lease a remote with the credential in an Authorization header, as Git sends it. */
    lease(granted: GitCredential, remote: string, mode: LeaseMode): Lease {
        const basic = new TextEncoder().encode(`${granted.username}:${granted.password}`);

        return {
            url: remote,
            mode,
            headers: { authorization: `Basic ${basic.toBase64()}` },
            expiresAt: granted.expiresAt,
        };
    },
});

/** The fetch a host supplies for calling Git hosts and their APIs. */
export type Fetch = (...arguments_: Parameters<typeof globalThis.fetch>) => Promise<Response>;

/** Storage for the repositories the platform hosts, supplied by the host like a bucket. */
export interface GitStorage {
    /** The provider name repositories stored here record. */
    readonly provider: string;
    /** Build the credential-free remote of a repository. */
    remote(id: string): string;
    /** Create an empty repository, accepting one that exists. */
    create(id: string): Promise<void>;
    /** Delete a repository and its history, accepting one already gone. */
    delete(id: string): Promise<void>;
    /** List a repository's branches and tags, and its default branch. */
    references(id: string): Promise<GitListing>;
    /** Lease a Git client access to a repository for a checkout. */
    open(id: string, mode: LeaseMode): Promise<Lease>;
}
