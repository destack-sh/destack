import { none, principal, relation } from "@destack/access";
import { check, index, sql, unique, type Select } from "@destack/db";
import { defineObject, field, method } from "@destack/object";
import { identifier, schema } from "@destack/schema";
import { Lease, LeaseMode } from "@destack/resource";
import { GitListing } from "../storage/storage.ts";
import { account } from "@destack/account/object";
import { AUTHENTICATIONS, HOSTINGS, ORIGIN_FIELDS, UPDATED_ORIGIN_FIELDS } from "./origin.ts";

/** An account-local repository name: lowercase letters, digits and inner hyphens, at most 63 characters. */
const RepositoryName = schema.string().regex(/^(?!-)[a-z0-9-]{1,63}(?<!-)$/);

/** Observe the origin's references and default branch. */
const refresh = method({ permission: "refresh" });

/** Record the references the repository's host observed. */
const report = method({ permission: "report", input: GitListing });

/** Lease a checkout short-lived, direct access to read the repository, or to write it as well. */
const open = method({
    permission: "pull",
    input: schema.object({ mode: LeaseMode }),
    output: Lease,
    mutates: false,
    audited: true,
});

/** A Git repository of an account: on platform storage, at GitHub, at any Git remote, or on one of its hosts. */
export const repository = defineObject({
    name: "repository",
    plural: "repositories",
    scope: account,
    fields: {
        /** The account-local name. */
        name: field.string(RepositoryName),
        /** Where the authoritative history lives: platform storage, GitHub, a Git remote or a host. */
        hosting: field.enum(HOSTINGS),
        /** The host that keeps a host repository and reports its references. */
        host: field.subject(principal.host).optional(),
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
        /** The account's connection to the GitHub App installation reaching the origin. */
        connectedAccountId: field.string(identifier("connected-account")).optional(),
        /** The space whose vault has the origin's secret credential. */
        secretSpaceId: field.string(identifier("space")).optional(),
        /** The secret with the origin's credential. */
        secretId: field.string(identifier("secret")).optional(),
    },
    indexes: { name: { on: ["name"], unique: true, across: account } },
    permissions: {
        read: relation("host"),
        create: none(),
        update: none(),
        refresh: none(),
        report: relation("host"),
        delete: none(),
        pull: none(),
        push: none(),
    },
    reserved: ["report"],
    recoverable: { within: { days: 30 }, by: "delete" },
    methods: {
        get: method.get("read"),
        list: method.list("read"),
        create: method.create("create", {
            isPredicted: false,
            fields: ["name", ...ORIGIN_FIELDS],
        }),
        update: method.update("update", {
            fields: ["name", ...UPDATED_ORIGIN_FIELDS],
        }),
        refresh,
        report,
        open,
    },
    constraints: (repository) => [
        unique("repository_scope_id").on(repository.scope, repository.id),
        index("repository_provider_repository").on(repository.providerRepositoryId),
        check(
            "repository_origin",
            sql`(${repository.hosting} = 'platform' AND ${repository.provider} IS NOT NULL AND ${repository.providerRepositoryId} IS NOT NULL AND ${repository.remote} IS NOT NULL AND ${repository.host} IS NULL)
            OR (${repository.hosting} = 'github' AND ${repository.provider} IS NULL AND ${repository.providerRepositoryId} IS NOT NULL AND ${repository.remote} IS NOT NULL AND ${repository.host} IS NULL)
            OR (${repository.hosting} = 'git' AND ${repository.provider} IS NULL AND ${repository.providerRepositoryId} IS NULL AND ${repository.remote} IS NOT NULL AND ${repository.host} IS NULL)
            OR (${repository.hosting} = 'host' AND ${repository.host} IS NOT NULL AND ${repository.provider} IS NULL AND ${repository.providerRepositoryId} IS NULL AND ${repository.remote} IS NULL)`,
        ),
        check(
            "repository_authentication",
            sql`(${repository.hosting} IN ('platform', 'host') AND ${repository.authentication} IS NULL AND ${repository.connectedAccountId} IS NULL AND ${repository.secretSpaceId} IS NULL AND ${repository.secretId} IS NULL)
            OR (${repository.hosting} = 'github' AND ${repository.authentication} IS NULL AND ${repository.connectedAccountId} IS NOT NULL AND ${repository.secretSpaceId} IS NULL AND ${repository.secretId} IS NULL)
            OR (${repository.hosting} = 'git' AND ${repository.connectedAccountId} IS NULL AND (
                (${repository.authentication} = 'anonymous' AND ${repository.secretSpaceId} IS NULL AND ${repository.secretId} IS NULL)
                OR (${repository.authentication} = 'secret' AND ${repository.secretSpaceId} IS NOT NULL AND ${repository.secretId} IS NOT NULL)))`,
        ),
        check(
            "repository_default_reference",
            sql`${repository.defaultReference} IS NULL OR ${repository.defaultReference} LIKE 'refs/heads/%'`,
        ),
        check(
            "repository_remote",
            sql`${repository.remote} IS NULL OR length(${repository.remote}) > 0`,
        ),
        check(
            "repository_provider",
            sql`(${repository.provider} IS NULL OR length(${repository.provider}) > 0)
            AND (${repository.providerRepositoryId} IS NULL OR length(${repository.providerRepositoryId}) > 0)`,
        ),
    ],
});

/** A persisted repository record. */
export type Repository = Select<typeof repository.table>;
