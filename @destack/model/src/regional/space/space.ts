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

/** Authoritative administration and desired execution for a space. */
export const space = table(
    "space",
    {
        ...recordColumns("space"),
        /** Account registered by the universe before this space is created. */
        accountId: identifier("account_id", "account").notNull(),
        /** The display name, independent of its globally reserved address. */
        name: text("name").notNull(),
        /** The region administering this space when administration is regional. */
        authorityRegionId: identifier("authority_region_id", "region").notNull(),
        /** The fencing epoch changed only by an explicit administration transfer. */
        authorityEpoch: integer("authority_epoch").notNull(),
        /** Whether applications should be available or suspended. */
        status: text("status", { enum: ["enabled", "suspended"] })
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
        check("space_name", sql`length(${space.name}) > 0`),
        check("space_authority_epoch", sql`${space.authorityEpoch} > 0`),
        check("space_status", sql`${space.status} IN ('enabled', 'suspended')`),
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

/** A regionally administered space with local or hosted execution. */
export type Space = Select<typeof space>;
