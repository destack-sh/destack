import {
    check,
    foreignKey,
    identifier,
    integer,
    primaryKey,
    type Select,
    sql,
    table,
    text,
} from "@destack/db";

import { space } from "./space.ts";

/** Hosts authorized to execute installations in a space. */
export const spaceHost = table("space_host", {
    /** The account administering the space. */
    accountId: identifier("account_id", "account").notNull(),
    /** The administered space. */
    spaceId: identifier("space_id", "space").notNull(),
    /** The authorized host. */
    hostId: identifier("host_id", "host").notNull(),
    /** Whether this host accepts new work or drains existing instances. */
    state: text("state", { enum: ["enabled", "draining", "disabled"] }).notNull().default(
        "enabled",
    ),
    /** The authorization epoch used to fence revoked host credentials. */
    epoch: integer("epoch").notNull().default(1),
}, (entry) => [
    primaryKey({ columns: [entry.spaceId, entry.hostId] }),
    foreignKey({
        columns: [entry.accountId, entry.spaceId],
        foreignColumns: [space.accountId, space.id],
    }).onDelete("restrict"),
    check("space_host_state", sql`${entry.state} IN ('enabled', 'draining', 'disabled')`),
    check("space_host_epoch", sql`${entry.epoch} >= 1`),
]);

/** An authorized execution host in a space. */
export type SpaceHost = Select<typeof spaceHost>;
