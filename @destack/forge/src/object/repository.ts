import { contained, none, principal, relation, union, universe } from "@destack/access";
import { check, index, sql, unique, type Select } from "@destack/db";
import { defineObject, field } from "@destack/object";
import { schema } from "@destack/schema";
import { ObjectReference } from "@destack/sync";
import { Lease, LeaseMode } from "@destack/resource";
import { GitListing } from "../storage/storage.ts";
import { account } from "@destack/account/object";
import {
    AUTHENTICATIONS,
    HOSTINGS,
    ORIGIN_FIELDS,
    OriginColumns,
    OriginMove,
    MOVABLE_ORIGIN_FIELDS,
} from "./origin.ts";

/** An account-local repository name: lowercase letters, digits and hyphens between them, at most 63 characters. */
const RepositoryName = schema.string().regex(/^(?!-)[a-z0-9-]{1,63}(?<!-)$/u);

/** A Git repository of an account: on platform storage, at GitHub, at any Git remote, or on one of its machines, read and pulled by the account's spaces. */
export const repository = defineObject({
    name: "repository",
    plural: "repositories",
    scope: account,
    fields: {
        /** The account-local name. */
        name: field.string(RepositoryName),
        /** Where the authoritative history lives: platform storage, GitHub, a Git remote or a machine. */
        hosting: field.enum(HOSTINGS),
        /** The machine that keeps a machine repository and reports its references. */
        machine: field.subject(principal.machine).optional(),
        /** The storage keeping a platform repository, by its provider name, such as code-storage. */
        provider: field.string().optional(),
        /** The storage's or GitHub's stable identifier of the repository. */
        providerRepositoryId: field.string().optional(),
        /** The credential-free clone URL. */
        remote: field.string().optional(),
        /** The branch the origin's HEAD names, including refs/heads/, as last observed. */
        defaultReference: field.string().optional(),
        /** How the platform authenticates to a Git remote. */
        authentication: field.enum(AUTHENTICATIONS).optional(),
        /** The account's connection to the GitHub App installation opening the origin. */
        connectedAccountId: field.string(schema.identifier("connected-account")).optional(),
        /** The vault secret with the origin's credential. */
        secret: field.json(ObjectReference).optional(),
    },
    indexes: {
        name: { on: ["name"], unique: true, across: () => account },
        id: { on: ["id"], unique: true, across: () => universe },
    },
    permissions: {
        read: union(relation("machine"), contained(principal.space)),
        create: none(),
        update: none(),
        refresh: none(),
        report: relation("machine"),
        delete: none(),
        pull: contained(principal.space),
        push: none(),
    },
    reserved: ["report"],
    recoverable: { within: { days: 30 }, by: "delete", prepared: OriginColumns },
    methods: (method) => ({
        get: method.get("read"),
        list: method.list("read"),
        create: method.create("create", {
            isPredicted: false,
            fields: ["name", ...ORIGIN_FIELDS],
            prepared: OriginColumns,
        }),
        update: method.update("update", {
            fields: ["name", ...MOVABLE_ORIGIN_FIELDS],
            prepared: OriginMove,
        }),
        /** Observe the origin's references and default branch. */
        refresh: method.mutation({ permission: "refresh", prepared: GitListing }),
        /** Record the references the repository's machine observed. */
        report: method.mutation({ permission: "report", input: GitListing }),
        /** Lease a checkout short-lived, direct access to read the repository, or to write it as well. */
        open: method.query({
            permission: "pull",
            input: schema.object({ mode: LeaseMode }),
            output: Lease,
            prepared: Lease,
            audited: true,
        }),
    }),
    constraints: (columns) => [
        unique("repository_scope_id").on(columns.scope, columns.id),
        index("repository_provider_repository").on(columns.providerRepositoryId),
        check(
            "repository_origin",
            sql`(${columns.hosting} = 'platform' AND ${columns.provider} IS NOT NULL AND ${columns.providerRepositoryId} IS NOT NULL AND ${columns.remote} IS NOT NULL AND ${columns.machine} IS NULL)
            OR (${columns.hosting} = 'github' AND ${columns.provider} IS NULL AND ${columns.providerRepositoryId} IS NOT NULL AND ${columns.remote} IS NOT NULL AND ${columns.machine} IS NULL)
            OR (${columns.hosting} = 'git' AND ${columns.provider} IS NULL AND ${columns.providerRepositoryId} IS NULL AND ${columns.remote} IS NOT NULL AND ${columns.machine} IS NULL)
            OR (${columns.hosting} = 'machine' AND ${columns.machine} IS NOT NULL AND ${columns.provider} IS NULL AND ${columns.providerRepositoryId} IS NULL AND ${columns.remote} IS NULL)`,
        ),
        check(
            "repository_authentication",
            sql`(${columns.hosting} IN ('platform', 'machine') AND ${columns.authentication} IS NULL AND ${columns.connectedAccountId} IS NULL AND ${columns.secret} IS NULL)
            OR (${columns.hosting} = 'github' AND ${columns.authentication} IS NULL AND ${columns.connectedAccountId} IS NOT NULL AND ${columns.secret} IS NULL)
            OR (${columns.hosting} = 'git' AND ${columns.connectedAccountId} IS NULL AND (
                (${columns.authentication} = 'anonymous' AND ${columns.secret} IS NULL)
                OR (${columns.authentication} = 'secret' AND ${columns.secret} IS NOT NULL)))`,
        ),
        check(
            "repository_default_reference",
            sql`${columns.defaultReference} IS NULL OR ${columns.defaultReference} LIKE 'refs/heads/%'`,
        ),
        check("repository_remote", sql`${columns.remote} IS NULL OR length(${columns.remote}) > 0`),
        check(
            "repository_provider",
            sql`(${columns.provider} IS NULL OR length(${columns.provider}) > 0)
            AND (${columns.providerRepositoryId} IS NULL OR length(${columns.providerRepositoryId}) > 0)`,
        ),
    ],
});

/** A persisted repository record. */
export type Repository = Select<typeof repository.table>;
