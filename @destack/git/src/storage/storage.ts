import type { Lease, LeaseMode } from "@destack/resource";
import { Instant, schema } from "@destack/schema";

/** A Git object name: 40 hexadecimal digits for SHA-1, 64 for SHA-256. */
const ObjectName = schema.string().regex(/^(?:[0-9a-f]{40}|[0-9a-f]{64})$/u);

/** A branch or tag as its origin lists it. */
export const GitReference = schema.object({
    /** The complete name, such as refs/heads/main or refs/tags/v1. */
    name: schema.string().regex(/^refs\/(?:heads|tags)\/./u),
    /** The object the reference points at directly: a commit, or an annotated tag's object. */
    object: ObjectName,
    /** The commit the reference resolves to, null when it resolves to no commit. */
    commit: ObjectName.nullable(),
});
/** A branch or tag as its origin lists it. */
export type GitReference = schema.Infer<typeof GitReference>;

/** Every branch and tag of a repository, and the existing branch of its HEAD. */
export const GitListing = schema.object({
    /** The branch of HEAD, including refs/heads/, null when HEAD points at no existing branch. */
    defaultReference: schema
        .string()
        .regex(/^refs\/heads\/./u)
        .nullable(),
    /** The branches and tags. */
    references: schema.array(GitReference),
});
/** Every branch and tag of a repository, and the existing branch of its HEAD. */
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
        return {
            url: remote,
            mode,
            headers: GitCredential.basic(granted.username, granted.password),
            expiresAt: granted.expiresAt,
        };
    },

    /** Build the Authorization header of a user and password, as Git sends Basic authentication. */
    basic(username: string, password: string): Readonly<Record<string, string>> {
        const encoded = new TextEncoder().encode(`${username}:${password}`);

        return { authorization: `Basic ${encoded.toBase64()}` };
    },
});

/** Storage for the repositories the platform hosts, supplied by the host like a bucket. */
export interface GitStorage {
    /** The provider name repositories stored here record. */
    readonly provider: string;
    /** Create an empty repository and answer its credential-free remote. */
    readonly create: (id: string) => Promise<string>;
    /** Delete a repository and its history, accepting one already gone. */
    readonly delete: (id: string) => Promise<void>;
    /** List a repository's branches and tags, and its default branch. */
    readonly references: (id: string) => Promise<GitListing>;
    /** Lease a Git client access to a repository for a checkout. */
    readonly open: (id: string, mode: LeaseMode) => Promise<Lease>;
}
