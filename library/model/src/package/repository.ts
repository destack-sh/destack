import { identifier, recordColumns, type Select, table, text, unique } from "@destack/db";
import { account } from "../account/account.ts";
import { reconcileChecks, reconcileColumns } from "../resource/reconcile.ts";

/** Repository records. */
export const repository = table("repository", {
    ...recordColumns("repository"),
    ...reconcileColumns(),
    /** The account owning the repository registration. */
    accountId: identifier("account_id", "account").notNull().references(() => account.id, {
        onDelete: "restrict",
    }),
    /** The account-local repository name. */
    name: text("name").notNull(),
    /** The Git remote URL, absent for a local-only repository. */
    remote: text("remote"),
}, (repository) => [
    ...reconcileChecks("repository", repository),
    unique("repository_account_name").on(repository.accountId, repository.name),
    unique("repository_account_id").on(repository.accountId, repository.id),
]);

/** A persisted repository record. */
export type Repository = Select<typeof repository>;
