import {
    check,
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
import { account } from "../account/account.ts";
import { user } from "../account/user.ts";

/** Device records. */
export const device = table(
    "device",
    {
        ...recordColumns("device"),
        /** The account registering the device. */
        accountId: identifier("account_id", "account")
            .notNull()
            .references(() => account.id, {
                onDelete: "restrict",
            }),
        /** The displayed device name. */
        name: text("name").notNull(),
        /** The user completing device enrollment. */
        enrolledBy: identifier("enrolled_by", "user").references(() => user.id, {
            onDelete: "set null",
        }),
        /** The client-reported form factor used for display. */
        category: text("category", {
            enum: ["desktop", "laptop", "phone", "tablet", "server", "unknown"],
        })
            .notNull()
            .default("unknown"),
        /** The client-reported operating system. */
        operatingSystem: text("operating_system"),
        /** The client-reported operating-system version. */
        operatingSystemVersion: text("operating_system_version"),
        /** The client-reported processor architecture. */
        architecture: text("architecture"),
        /** The client-reported hardware model. */
        model: text("model"),
        /** The installed Destack client version. */
        clientVersion: text("client_version"),
        /** Revocation time, when access was withdrawn. */
        revokedAt: integer("revoked_at"),
        /** Last authenticated contact with the device. */
        lastSeenAt: integer("last_seen_at"),
    },
    (device) => [
        unique("device_account_id").on(device.accountId, device.id),
        index("device_enrolled_by").on(device.enrolledBy),
        check(
            "device_category",
            sql`${device.category} IN ('desktop', 'laptop', 'phone', 'tablet', 'server', 'unknown')`,
        ),
    ],
);

/** A persisted device record. */
export type Device = Select<typeof device>;
