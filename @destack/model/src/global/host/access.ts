import { identifier, integer, primaryKey, type Select, table } from "@destack/db";
import { account } from "../account/account.ts";
import { host } from "./host.ts";

/** An account allowed to place workloads on a host. */
export const hostAccess = table(
    "host_access",
    {
        /** The host granting access, including to its owning account. */
        hostId: identifier("host_id", "host")
            .notNull()
            .references(() => host.id),
        /** The account authorised to use the host. */
        accountId: identifier("account_id", "account")
            .notNull()
            .references(() => account.id),
        /** The grant creation time. */
        createdAt: integer("created_at").notNull(),
        /** Revocation time; controllers must drain affected workloads. */
        revokedAt: integer("revoked_at"),
    },
    (access) => [primaryKey({ columns: [access.accountId, access.hostId] })],
);

/** An account's host grant. */
export type HostAccess = Select<typeof hostAccess>;
