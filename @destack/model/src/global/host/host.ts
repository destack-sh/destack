import {
    check,
    foreignKey,
    identifier,
    integer,
    json,
    recordColumns,
    type Select,
    sql,
    table,
    text,
    unique,
} from "@destack/db";
import { account } from "../account/account.ts";
import { device } from "./device.ts";
import { region } from "./region.ts";
import { schema } from "@destack/schema";
import { Runtime } from "@destack/package/runtime";

/** Host records. */
export const host = table(
    "host",
    {
        ...recordColumns("host"),
        /** The account operating this host. */
        accountId: identifier("account_id", "account")
            .notNull()
            .references(() => account.id, {
                onDelete: "restrict",
            }),
        /** The host execution type. */
        kind: text("kind", { enum: ["device", "cloud"] }).notNull(),
        /** The device running a local host. */
        deviceId: identifier("device_id", "device"),
        /** The hosting provider region, when known. */
        regionId: identifier("region_id", "region").references(() => region.id),
        /** Whether the host accepts work, drains existing work, or is disabled. */
        state: text("state", { enum: ["enabled", "draining", "disabled"] })
            .notNull()
            .default("enabled"),
        /** The running host software version reported by the host. */
        version: text("version"),
        /** Runtimes supported by this host's installed adapters. */
        runtimes: json("runtimes", schema.array(Runtime))
            .notNull()
            .default(sql`'[]'`),
        /** The last authenticated heartbeat received by the administration service. */
        lastSeenAt: integer("last_seen_at"),
    },
    (host) => [
        unique("host_device_id").on(host.deviceId, host.id),
        unique("host_account_id").on(host.accountId, host.id),
        check("host_state", sql`${host.state} IN ('enabled', 'draining', 'disabled')`),
        foreignKey({
            columns: [host.accountId, host.deviceId],
            foreignColumns: [device.accountId, device.id],
        }).onDelete("restrict"),
        check(
            "host_kind",
            sql`(${host.kind} = 'device' AND ${host.deviceId} IS NOT NULL) OR (${host.kind} = 'cloud' AND ${host.deviceId} IS NULL)`,
        ),
    ],
);

/** A persisted host record. */
export type Host = Select<typeof host>;
