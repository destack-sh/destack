import {
    check,
    identifier,
    integer,
    recordColumns,
    type Select,
    sql,
    table,
    text,
    unique,
} from "@destack/db";

import { reconciliationChecks, reconciliationColumns } from "../../record/index.ts";

/** Regional execution and reconciliation state for a globally registered space. */
export const space = table(
    "space",
    {
        ...recordColumns("space"),
        /** The global account ID replicated for regional authorization and foreign keys. */
        accountId: identifier("account_id", "account").notNull(),
        /** Whether applications should be available or suspended. */
        state: text("state", { enum: ["enabled", "suspended"] })
            .notNull()
            .default("enabled"),
        /** Select automatic placement or a specific host. */
        placement: text("placement", { enum: ["automatic", "host"] })
            .notNull()
            .default("automatic"),
        /** The requested primary host when placement is explicit. */
        requestedPrimaryHostId: identifier("requested_primary_host_id", "host"),
        /** The primary host coordinating this space, absent while unassigned. */
        primaryHostId: identifier("primary_host_id", "host"),
        /** The authority epoch incremented when host access changes. */
        primaryHostEpoch: integer("primary_host_epoch").notNull().default(0),

        ...reconciliationColumns(),
    },
    (space) => [
        ...reconciliationChecks("space", space),
        unique("space_account_id").on(space.accountId, space.id),
        check("space_state", sql`${space.state} IN ('enabled', 'suspended')`),
        check(
            "space_placement",
            sql`(${space.placement} = 'automatic' AND ${space.requestedPrimaryHostId} IS NULL) OR (${space.placement} = 'host' AND ${space.requestedPrimaryHostId} IS NOT NULL)`,
        ),
        check(
            "space_host_epoch",
            sql`${space.primaryHostEpoch} >= 0 AND (${space.primaryHostId} IS NULL OR ${space.primaryHostEpoch} > 0)`,
        ),
    ],
);

/** A persisted regional space record. */
export type Space = Select<typeof space>;
