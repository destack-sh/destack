import { Instant, schema } from "@destack/schema";

/** A Git object name: 40 hexadecimal digits for SHA-1, 64 for SHA-256. */
const ObjectName = schema.string().regex(/^(?:[0-9a-f]{40}|[0-9a-f]{64})$/);

/** A branch or tag as its origin lists it. */
export const GitReference = schema.object({
    /** The complete name, such as refs/heads/main or refs/tags/v1. */
    name: schema.string().regex(/^refs\/(?:heads|tags)\/./),
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
        .regex(/^refs\/heads\/./)
        .nullable(),
    /** The branches and tags. */
    references: schema.array(GitReference),
});
/** Every branch and tag of a repository, and the existing branch its HEAD names. */
export type GitListing = schema.Infer<typeof GitListing>;

/** What a Git client may do: clone and fetch, or push as well. */
export const GitMode = schema.enum(["pull", "push"]);
/** What a Git client may do: clone and fetch, or push as well. */
export type GitMode = schema.Infer<typeof GitMode>;

/** A short-lived HTTP basic credential for Git over HTTPS. */
export const GitCredential = schema.object({
    /** The basic authentication user name. */
    username: schema.string().min(1),
    /** The basic authentication password, such as a token. */
    password: schema.sensitive(schema.string().min(1)),
    /** The expiry in UTC epoch milliseconds. */
    expiresAt: Instant,
});
/** A short-lived HTTP basic credential for Git over HTTPS. */
export type GitCredential = schema.Infer<typeof GitCredential>;

/** How a Git client reaches a repository: its remote and the credential it presents, if any. */
export const GitAccess = schema.object({
    /** The credential-free remote. */
    remote: schema.string().min(1),
    /** The credential the client presents, null for remotes needing none, such as local paths. */
    credential: GitCredential.nullable(),
});
/** How a Git client reaches a repository: its remote and the credential it presents, if any. */
export type GitAccess = schema.Infer<typeof GitAccess>;

/** The fetch a host supplies for reaching Git hosts and their APIs. */
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
    /** Grant a Git client access to a repository for a checkout. */
    access(id: string, mode: GitMode): Promise<GitAccess>;
}
