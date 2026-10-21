import { permission, principal, relation, through, union, universe } from "@destack/access";
import { account, space } from "@destack/account/object";
import type { Select } from "@destack/db";
import { defineObject, field } from "@destack/object";
import { Lease, LeaseMode } from "@destack/resource";
import { schema, present } from "@destack/schema";
import { CallError } from "@destack/object";
import { GitListing } from "../storage/storage.ts";

/** Where a repository's authoritative history lives. */
const HOSTINGS = ["platform", "github", "git", "machine"] as const;

/** The ways the platform authenticates to a Git remote. */
const AUTHENTICATIONS = ["anonymous", "credential"] as const;

/** The fields of a repository's origin, as callers write them. */
export const ORIGIN_FIELDS = [
    "hosting",
    "machineId",
    "remote",
    "authentication",
    "connectionId",
] as const;

/** The origin fields of a call or a repository, before they are read as an origin. */
type OriginFields = { readonly [Name in (typeof ORIGIN_FIELDS)[number]]?: unknown };

/** Where a repository's authoritative history lives, and how the platform authenticates to it. */
const repositoryOrigin = schema.union([
    schema.object({ hosting: schema.literal("platform") }),
    schema.object({ hosting: schema.literal("machine"), machineId: schema.identifier("machine") }),
    schema.object({
        hosting: schema.literal("github"),
        remote: schema.url(),
        connectionId: schema.identifier("connection"),
    }),
    schema.object({
        hosting: schema.literal("git"),
        remote: schema.url(),
        authentication: schema.literal("anonymous"),
    }),
    schema.object({
        hosting: schema.literal("git"),
        remote: schema.url(),
        authentication: schema.literal("credential"),
    }),
]);
/** Where a repository's authoritative history lives, and how the platform authenticates to it. */
export type RepositoryOrigin = schema.Infer<typeof repositoryOrigin>;

/** Where a repository's authoritative history lives, read from origin fields. */
export const RepositoryOrigin = Object.assign(repositoryOrigin, {
    /** Read the origin from origin fields, refusing invalid combinations. */
    read(fields: OriginFields): RepositoryOrigin {
        // parse the given origin fields
        const given = Object.fromEntries(
            ORIGIN_FIELDS.filter((name) => fields[name] !== undefined && fields[name] !== null).map(
                (name) => [name, fields[name]],
            ),
        );
        const parsed = repositoryOrigin.safeParse(given);
        if (!parsed.success) {
            throw new CallError("BAD_REQUEST", {
                message: "repository origin is no platform, github, git or machine origin",
                data: { issues: parsed.error.issues },
            });
        }

        return parsed.data;
    },
});

/** The origin fields a repository's update may change: every one but its machine. */
const MOVABLE_ORIGIN_FIELDS = ORIGIN_FIELDS.filter(
    (name): name is Exclude<(typeof ORIGIN_FIELDS)[number], "machineId"> => name !== "machineId",
);

/** The columns of a repository's origin, and its identity in storage or at GitHub. */
const originColumns = schema.object({
    /** Where the repository is hosted. */
    hosting: schema.enum(HOSTINGS),
    /** The machine serving a machine repository. */
    machineId: schema.identifier("machine").nullable(),
    /** The remote of a GitHub, Git or platform repository. */
    remote: schema.string().nullable(),
    /** How a Git repository authenticates. */
    authentication: schema.enum(AUTHENTICATIONS).nullable(),
    /** The connection opening a GitHub repository. */
    connectionId: schema.identifier("connection").nullable(),
    /** The storage provider of a platform repository. */
    provider: schema.string().nullable(),
    /** The repository's identity in storage or at GitHub. */
    providerRepositoryId: schema.string().nullable(),
});
/** The columns of a repository's origin, and its identity in storage or at GitHub. */
export type OriginColumns = schema.Infer<typeof originColumns>;

/** The columns of a repository's origin, and its identity in storage or at GitHub. */
export const OriginColumns = Object.assign(originColumns, {
    /** The origin columns of no origin, which the columns of each origin overwrite. */
    blank: {
        machineId: null,
        remote: null,
        authentication: null,
        connectionId: null,
        provider: null,
        providerRepositoryId: null,
    } as const satisfies Omit<OriginColumns, "hosting">,

    /** Read a Git or GitHub repository's remote, which its origin constraint requires. */
    requireRemote(row: Pick<OriginColumns, "remote">): string {
        return present(row.remote, "a remote repository's remote");
    },

    /** Read a repository's origin columns. */
    of(row: OriginColumns): OriginColumns {
        return {
            hosting: row.hosting,
            machineId: row.machineId,
            remote: row.remote,
            authentication: row.authentication,
            connectionId: row.connectionId,
            provider: row.provider,
            providerRepositoryId: row.providerRepositoryId,
        };
    },
});

/** The origin an update moves to, prepared at the revision the transaction requires. */
export const OriginMove = schema
    .object({
        /** The revision the move was prepared at. */
        revision: schema.int(),
        /** The origin columns the update records. */
        columns: OriginColumns,
    })
    .nullable();
/** The origin an update moves to, prepared at the revision the transaction requires. */
export type OriginMove = schema.Infer<typeof OriginMove>;

/** The user and password a Git remote takes over Basic authentication, never read back. */
export const GitRemoteCredential = schema.object({
    /** The user name. */
    user: schema.string().min(1),
    /** The password or token. */
    password: schema.string().min(1),
});
/** The user and password a Git remote takes over Basic authentication. */
export type GitRemoteCredential = schema.Infer<typeof GitRemoteCredential>;

/** An account-local repository name, at most 63 characters. */
const RepositoryName = schema.string().regex(/^[a-z0-9](?:[a-z0-9-]{0,61}[a-z0-9])?$/u);

/** A Git repository of an account, read and pulled by the account's spaces. */
export const repository = defineObject(
    {
        name: "repository",
        plural: "repositories",
        scope: space,
        owner: "account",
        fields: {
            /** The account owning the repository. */
            account: field.reference(account).relate("account"),
            /** The account-local name. */
            name: field.string(RepositoryName),
            /** Where the authoritative history lives. */
            hosting: field.enum(HOSTINGS),
            /** The machine that hosts a machine repository and reports its references. */
            machine: field.reference(principal.machine).optional(),
            /** A platform repository's storage provider, such as local or cloudflare-artifacts. */
            provider: field.string().optional(),
            /** The storage's or GitHub's stable identifier of the repository. */
            providerRepositoryId: field.string().optional(),
            /** The credential-free clone URL. */
            remote: field.string().optional(),
            /** The branch of the origin's HEAD, including refs/heads/, as last observed. */
            defaultReference: field.string().optional(),
            /** How the platform authenticates to a Git remote. */
            authentication: field.enum(AUTHENTICATIONS).optional(),
            /** The account's connection to the GitHub App installation opening the origin. */
            connectionId: field.string(schema.identifier("connection")).optional(),
        },
        indexes: {
            name: {
                on: ["accountId", "name"],
                across: () => universe,
            },
            id: { on: ["id"], across: () => universe },
        },
        permissions: {
            read: union(relation("machine"), through("account", "read")),
            create: through("account", "manage"),
            update: through("account", "manage"),
            // the account's owners, and its servers on deliveries and identification
            refresh: through("account", "serve"),
            report: relation("machine"),
            record: union(permission("refresh"), permission("report")),
            delete: through("account", "manage"),
            pull: through("account", "read"),
            push: through("account", "manage"),
        },
        reserved: ["report"],
        recoverable: { within: { days: 30 }, by: "delete", prepared: OriginColumns },
        constraints: {
            scopeId: { kind: "unique", on: ["scope", "id"] },
            providerRepository: { kind: "index", on: ["providerRepositoryId"] },
            origin: {
                kind: "check",
                condition: {
                    OR: [
                        {
                            hosting: "platform",
                            provider: { isNotNull: true },
                            providerRepositoryId: { isNotNull: true },
                            remote: { isNotNull: true },
                            machineId: { isNull: true },
                        },
                        {
                            hosting: "github",
                            provider: { isNull: true },
                            remote: { isNotNull: true },
                            machineId: { isNull: true },
                        },
                        {
                            hosting: "git",
                            provider: { isNull: true },
                            providerRepositoryId: { isNull: true },
                            remote: { isNotNull: true },
                            machineId: { isNull: true },
                        },
                        {
                            hosting: "machine",
                            machineId: { isNotNull: true },
                            provider: { isNull: true },
                            providerRepositoryId: { isNull: true },
                            remote: { isNull: true },
                        },
                    ],
                },
            },
            authentication: {
                kind: "check",
                condition: {
                    OR: [
                        {
                            hosting: { in: ["platform", "machine"] },
                            authentication: { isNull: true },
                            connectionId: { isNull: true },
                        },
                        {
                            hosting: "github",
                            authentication: { isNull: true },
                            connectionId: { isNotNull: true },
                        },
                        {
                            hosting: "git",
                            connectionId: { isNull: true },
                            authentication: { in: ["anonymous", "credential"] },
                        },
                    ],
                },
            },
            defaultReference: {
                kind: "check",
                condition: {
                    defaultReference: { OR: [{ isNull: true }, { startsWith: "refs/heads/" }] },
                },
            },
            remote: {
                kind: "check",
                condition: { remote: { OR: [{ isNull: true }, { length: { gt: 0 } }] } },
            },
            provider: {
                kind: "check",
                condition: {
                    provider: { OR: [{ isNull: true }, { length: { gt: 0 } }] },
                    providerRepositoryId: { OR: [{ isNull: true }, { length: { gt: 0 } }] },
                },
            },
        },
    },
    (method) => ({
        create: method.create("create", {
            isPredicted: false,
            fields: ["accountId", "name", ...ORIGIN_FIELDS],
            input: schema.object({
                /** The credential of a Git remote authenticating by credential, written once. */
                credential: schema.sensitive(GitRemoteCredential).exactOptional(),
            }),
            prepared: OriginColumns,
        }),
        /** Lease a checkout short-lived, direct access to read or write the repository. */
        open: method.query({
            permission: "pull",
            input: schema.object({ mode: LeaseMode }),
            output: Lease,
            prepared: Lease,
            isAudited: true,
        }),
        /** Observe the origin's references and default branch. */
        refresh: method.mutation({ permission: "refresh", prepared: GitListing }),
        /** Record GitHub's identifier of the repository. */
        identify: method.mutation({ permission: "refresh", prepared: schema.string().min(1) }),
        /** Record the references the repository's machine observed. */
        report: method.mutation({ permission: "report", input: GitListing }),
        update: method.update("update", {
            fields: ["name", ...MOVABLE_ORIGIN_FIELDS],
            input: schema.object({
                /** A new credential of a Git remote authenticating by credential, written once. */
                credential: schema.sensitive(GitRemoteCredential).exactOptional(),
            }),
            prepared: OriginMove,
        }),
    }),
);

/** A persisted repository record. */
export type Repository = Select<typeof repository.table>;
