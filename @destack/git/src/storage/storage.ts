import { BasicCredential } from "@destack/environment";
import type { Lease, LeaseMode } from "@destack/resource";
import { Instant, schema } from "@destack/schema";

/** The domain of commit authors' addresses, which receives no mail. */
const AUTHOR_DOMAIN = "users.noreply.destack.app";

/** A Git object name: 40 hexadecimal digits for SHA-1, 64 for SHA-256. */
const ObjectName = schema.string().regex(/^(?:[0-9a-f]{40}|[0-9a-f]{64})$/u);

/** The namespaces of the references a repository lists: branches, tags and checkouts. */
const NAMESPACES = {
    branch: "refs/heads/",
    tag: "refs/tags/",
    checkout: "refs/checkouts/",
} as const;

/** The names a repository lists: a name inside one of its namespaces. */
const LISTED = new RegExp(`^(?:${Object.values(NAMESPACES).join("|")}).`, "u");

/** A branch, tag or checkout's working tree as its origin lists it. */
const reference = schema.object({
    /** The complete name, such as refs/heads/main or refs/checkouts/<machine>/<checkout>. */
    name: schema.string().regex(LISTED),
    /** The object the reference points at directly: a commit, or an annotated tag's object. */
    object: ObjectName,
    /** The commit the reference resolves to, null when it resolves to no commit. */
    commit: ObjectName.nullable(),
});
/** A branch, tag or checkout's working tree as its origin lists it. */
export const GitReference = Object.assign(reference, {
    /** The namespaces of the references a repository lists. */
    namespaces: NAMESPACES,

    /** Report whether a repository lists a reference of a name. */
    isListed(name: string): boolean {
        return LISTED.test(name);
    },

    /** Read a reference's name below its namespace, such as `main` for `refs/heads/main`. */
    short(name: string): string {
        const namespace = Object.values(NAMESPACES).find((each) => name.startsWith(each));

        return namespace === undefined ? name : name.slice(namespace.length);
    },

    /** Write the name of a branch's reference, such as `refs/heads/main` for `main`. */
    branch(branch: string): string {
        return `${NAMESPACES.branch}${branch}`;
    },

    /** Report whether a reference is a branch's. */
    isBranch(name: string): boolean {
        return name.startsWith(NAMESPACES.branch);
    },

    /** Write the name of the reference a machine's checkout pushes its working tree to. */
    checkout(machine: string, checkout: string): string {
        return `${NAMESPACES.checkout}${machine}/${checkout}`;
    },

    /** Report whether a reference is a checkout's. */
    isCheckout(name: string): boolean {
        return name.startsWith(NAMESPACES.checkout);
    },

    /** Read the machine a checkout's reference names, absent for another or a malformed one. */
    machine(name: string): string | undefined {
        // take the machine of refs/checkouts/<machine>/<checkout> alone
        const [machine, checkout, ...rest] = GitReference.isCheckout(name)
            ? name.slice(NAMESPACES.checkout.length).split("/")
            : [];

        return checkout !== undefined && rest.length === 0 ? machine : undefined;
    },
});
/** A branch, tag or checkout's working tree as its origin lists it. */
export type GitReference = schema.Infer<typeof reference>;

/** Every branch, tag and checkout of a repository, and the existing branch of its HEAD. */
export const GitListing = schema.object({
    /** The branch of HEAD, including refs/heads/, null when HEAD points at no existing branch. */
    defaultReference: schema
        .string()
        .regex(/^refs\/heads\/./u)
        .nullable(),
    /** The branches, tags and checkouts. */
    references: schema.array(GitReference),
});
/** Every branch, tag and checkout of a repository, and the existing branch of its HEAD. */
export type GitListing = schema.Infer<typeof GitListing>;

/** The author of a commit: a name and an email address. */
const author = schema.object({
    /** The author's name. */
    name: schema.string().min(1),
    /** The author's email address. */
    email: schema.string().min(1),
});
/** The author of a commit: a name and an email address. */
export const GitAuthor = Object.assign(author, {
    /** Make an account's author: its name, and its handle at the noreply domain. */
    of(account: { readonly name: string; readonly handle: string }): GitAuthor {
        return { name: account.name, email: `${account.handle}@${AUTHOR_DOMAIN}` };
    },
});
/** The author of a commit. */
export type GitAuthor = schema.Infer<typeof author>;

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
        return { authorization: BasicCredential.write(username, password) };
    },
});

/** Storage for the repositories the platform hosts, supplied by the host like a bucket. */
export interface GitStorage {
    /** The provider name repositories stored here record. */
    readonly provider: string;
    /** Create an empty repository, or leave an existing one, and answer its remote. */
    readonly create: (id: string) => Promise<string>;
    /** Delete a repository and its history, accepting one already gone. */
    readonly delete: (id: string) => Promise<void>;
    /** List a repository's branches and tags, and its default branch. */
    readonly references: (id: string) => Promise<GitListing>;
    /** Lease a Git client access to a repository for a checkout. */
    readonly open: (id: string, mode: LeaseMode) => Promise<Lease>;
    /** Answer a smart HTTP request below a repository, such as `/info/refs`, through a lease. */
    readonly serve: (lease: Lease, path: string, request: Request) => Promise<Response>;
}
