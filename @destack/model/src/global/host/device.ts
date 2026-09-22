import { identifier, integer, recordColumns, type Select, table, text, unique } from "@destack/db";
import { account } from "../account/account.ts";

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
        /** Revocation time, when access was withdrawn. */
        revokedAt: integer("revoked_at"),
        /** Last authenticated contact with the device. */
        lastSeenAt: integer("last_seen_at"),
    },
    (device) => [unique("device_account_id").on(device.accountId, device.id)],
);

/** A persisted device record. */
export type Device = Select<typeof device>;
