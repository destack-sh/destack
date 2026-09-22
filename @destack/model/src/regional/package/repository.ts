import {
    check,
    identifier,
    recordColumns,
    type Select,
    sql,
    table,
    text,
    unique,
} from "@destack/db";

import { reconciliationChecks, reconciliationColumns } from "../../record/index.ts";

/** A registered Git repository and its authoritative origin. */
export const repository = table(
    "repository",
    {
        ...recordColumns("repository"),
        ...reconciliationColumns(),
        /** The account owning the repository registration. */
        accountId: identifier("account_id", "account").notNull(),
        /** Who stores the authoritative Git history. */
        hosting: text("hosting", { enum: ["platform", "external", "host"] }).notNull(),
        /** The host storing a host-local origin; ordinary checkouts are independent. */
        hostId: identifier("host_id", "host"),
        /** The Git provider adapter, such as code-storage, github, or git. */
        provider: text("provider"),
        /** The provider's stable repository identifier, retained across URL changes. */
        providerRepositoryId: text("provider_repository_id"),
        /** The credential-free clone URL; absent before provisioning or for a host-local origin. */
        remote: text("remote"),
        /** The last observed symbolic HEAD target, including refs/heads/. */
        defaultReference: text("default_reference"),

        /** The external origin's authentication method. */
        authentication: text("authentication", {
            enum: ["anonymous", "connection", "secret"],
        }),
        /** The global connected account authorisation used for unattended Git operations. */
        connectedAccountId: identifier("connected_account_id", "connected-account"),
        /** The space holding an explicit Git credential. */
        secretSpaceId: identifier("secret_space_id", "space"),
        /** The vault-held Git credential; values never enter repository records. */
        secretId: identifier("secret_id", "secret"),
    },
    (repository) => [
        ...reconciliationChecks("repository", repository),
        unique("repository_account_id").on(repository.accountId, repository.id),
        check(
            "repository_origin",
            sql`(${repository.hosting} = 'platform' AND ${repository.provider} IS NOT NULL AND ${repository.hostId} IS NULL)
            OR (${repository.hosting} = 'external' AND ${repository.provider} IS NOT NULL AND ${repository.remote} IS NOT NULL AND ${repository.hostId} IS NULL)
            OR (${repository.hosting} = 'host' AND ${repository.hostId} IS NOT NULL AND ${repository.provider} IS NULL AND ${repository.providerRepositoryId} IS NULL AND ${repository.remote} IS NULL)`,
        ),
        check(
            "repository_authentication",
            sql`(${repository.hosting} IN ('platform', 'host') AND ${repository.authentication} IS NULL AND ${repository.connectedAccountId} IS NULL AND ${repository.secretSpaceId} IS NULL AND ${repository.secretId} IS NULL)
            OR (${repository.hosting} = 'external' AND ${repository.authentication} IS NOT NULL AND (
                (${repository.authentication} = 'anonymous' AND ${repository.connectedAccountId} IS NULL AND ${repository.secretSpaceId} IS NULL AND ${repository.secretId} IS NULL)
                OR (${repository.authentication} = 'connection' AND ${repository.connectedAccountId} IS NOT NULL AND ${repository.secretSpaceId} IS NULL AND ${repository.secretId} IS NULL)
                OR (${repository.authentication} = 'secret' AND ${repository.connectedAccountId} IS NULL AND ${repository.secretSpaceId} IS NOT NULL AND ${repository.secretId} IS NOT NULL)))`,
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
);

/** A persisted repository record. */
export type Repository = Select<typeof repository>;
