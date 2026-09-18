import {
    check,
    foreignKey,
    identifier,
    integer,
    recordColumns,
    type Select,
    sql,
    table,
    text,
    unique,
} from "@destack/db";
import { account } from "../account/account.ts";
import { hostAccess } from "../host/access.ts";
import { RESIDENCIES } from "../host/residency.ts";
import { reconcileChecks, reconcileColumns } from "../resource/reconcile.ts";
import { ENVIRONMENTS } from "./environment.ts";

/** Space records. */
export const space = table("space", {
    ...recordColumns("space"),
    /** The account owning the space. */
    accountId: identifier("account_id", "account").notNull().references(() => account.id, {
        onDelete: "restrict",
    }),
    /** The account-local name used in addresses. */
    slug: text("slug").notNull(),
    /** The displayed space name. */
    name: text("name").notNull(),
    /** The purpose of this space. */
    environment: text("environment", { enum: ENVIRONMENTS }).notNull(),
    /** The permitted storage and processing region. */
    residency: text("residency", { enum: RESIDENCIES }).notNull(),
    /** Whether applications should be available or suspended. */
    state: text("state", { enum: ["enabled", "suspended"] }).notNull().default("enabled"),
    /** Select automatic placement or a specific host. */
    placement: text("placement", { enum: ["automatic", "host"] }).notNull().default("automatic"),
    /** The requested host when placement is explicit. */
    requestedHostId: identifier("requested_host_id", "host"),
    /** The assigned host, absent while unassigned. */
    hostId: identifier("host_id", "host"),
    /** The authority epoch incremented when host access changes. */
    hostEpoch: integer("host_epoch").notNull().default(0),

    ...reconcileColumns(),
}, (space) => [
    ...reconcileChecks("space", space),
    unique("space_account_slug").on(space.accountId, space.slug),
    unique("space_account_id").on(space.accountId, space.id),
    foreignKey({
        columns: [space.accountId, space.hostId],
        foreignColumns: [hostAccess.accountId, hostAccess.hostId],
    }).onDelete("restrict"),
    foreignKey({
        columns: [space.accountId, space.requestedHostId],
        foreignColumns: [hostAccess.accountId, hostAccess.hostId],
    }).onDelete("restrict"),
    check("space_state", sql`${space.state} IN ('enabled', 'suspended')`),
    check(
        "space_placement",
        sql`(${space.placement} = 'automatic' AND ${space.requestedHostId} IS NULL) OR (${space.placement} = 'host' AND ${space.requestedHostId} IS NOT NULL)`,
    ),
    check(
        "space_host_epoch",
        sql`${space.hostEpoch} >= 0 AND (${space.hostId} IS NULL OR ${space.hostEpoch} > 0)`,
    ),
    check(
        "space_environment",
        sql`${space.environment} IN ('development', 'preview', 'production')`,
    ),
    check("space_residency", sql`${space.residency} IN ('eu', 'us')`),
    check(
        "space_slug",
        sql`length(${space.slug}) BETWEEN 1 AND 63 AND ${space.slug} NOT GLOB '*[^a-z0-9-]*' AND ${space.slug} NOT LIKE '-%' AND ${space.slug} NOT LIKE '%-'`,
    ),
]);

/** A persisted space record. */
export type Space = Select<typeof space>;
