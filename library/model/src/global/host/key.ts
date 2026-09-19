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
import { device } from "./device.ts";

/** A revocable device authentication key; private keys remain on the device. */
export const deviceKey = table("device_key", {
    ...recordColumns("device-key"),
    /** The device authenticated by this key. */
    deviceId: identifier("device_id", "device").notNull().references(() => device.id),
    /** The encoded public key, including its algorithm. */
    publicKey: text("public_key").notNull().unique(),
    /** The key expiry time. */
    expiresAt: integer("expires_at").notNull(),
    /** Explicit key revocation time. */
    revokedAt: integer("revoked_at"),
}, (key) => [
    unique("device_key_device_id").on(key.deviceId, key.id),
    check("device_key_expiry", sql`${key.expiresAt} > ${key.createdAt}`),
]);

/** A device authentication key. */
export type DeviceKey = Select<typeof deviceKey>;
