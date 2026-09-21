import {
    check,
    foreignKey,
    identifier,
    index,
    integer,
    recordColumns,
    type Select,
    sql,
    table,
    text,
    unique,
} from "@destack/db";
import { deviceKey } from "./key.ts";
import { device } from "./device.ts";
import { host } from "./host.ts";

/** An authenticated connection leased by a relay to a device host. */
export const tunnel = table(
    "tunnel",
    {
        ...recordColumns("tunnel"),
        /** The host reachable through this connection. */
        hostId: identifier("host_id", "host")
            .notNull()
            .references(() => host.id),
        /** The authenticated device establishing this connection. */
        deviceId: identifier("device_id", "device")
            .notNull()
            .references(() => device.id),
        /** The device key used to establish the connection. */
        deviceKeyId: identifier("device_key_id", "device-key").notNull(),
        /** The relay routing this connection. */
        relay: text("relay").notNull(),
        /** The relay's opaque connection identifier. */
        connection: text("connection").notNull(),
        /** The last authenticated heartbeat. */
        heartbeatAt: integer("heartbeat_at").notNull(),
        /** The lease deadline; expiry makes the connection unusable. */
        expiresAt: integer("expires_at").notNull(),
        /** Explicit closure or revocation time. */
        closedAt: integer("closed_at"),
    },
    (tunnel) => [
        foreignKey({
            columns: [tunnel.deviceId, tunnel.hostId],
            foreignColumns: [host.deviceId, host.id],
        }),
        foreignKey({
            columns: [tunnel.deviceId, tunnel.deviceKeyId],
            foreignColumns: [deviceKey.deviceId, deviceKey.id],
        }),
        unique("tunnel_connection").on(tunnel.relay, tunnel.connection),
        index("tunnel_host_expiry").on(tunnel.hostId, tunnel.expiresAt),
        check(
            "tunnel_lease",
            sql`${tunnel.heartbeatAt} >= ${tunnel.createdAt} AND ${tunnel.expiresAt} > ${tunnel.heartbeatAt}`,
        ),
        check(
            "tunnel_close",
            sql`${tunnel.closedAt} IS NULL OR ${tunnel.closedAt} >= ${tunnel.createdAt}`,
        ),
    ],
);

/** A host's leased relay connection. */
export type Tunnel = Select<typeof tunnel>;
