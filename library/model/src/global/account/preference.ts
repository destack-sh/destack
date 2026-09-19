import {
    check,
    foreignKey,
    identifier,
    json,
    recordColumns,
    type Select,
    sql,
    table,
    text,
    uniqueIndex,
} from "@destack/db";
import { schema } from "@destack/schema";
import { device } from "../host/device.ts";
import { account } from "./account.ts";
import { user } from "./user.ts";

/** Preference records. */
export const preference = table("preference", {
    ...recordColumns("preference"),
    /** The account whose applications use the preference. */
    accountId: identifier("account_id", "account").notNull().references(() => account.id, {
        onDelete: "cascade",
    }),
    /** The user override, absent for account defaults. */
    userId: identifier("user_id", "user").references(() => user.id, { onDelete: "cascade" }),
    /** The device override, absent for a cross-device preference. */
    deviceId: identifier("device_id", "device").references(() => device.id, {
        onDelete: "cascade",
    }),
    /** The namespaced setting key. */
    name: text("name").notNull(),
    /** The value validated by the setting's declaring package. */
    value: json("value", schema.json()).notNull(),
}, (preference) => [
    foreignKey({
        columns: [preference.accountId, preference.deviceId],
        foreignColumns: [device.accountId, device.id],
    }).onDelete("cascade"),
    uniqueIndex("preference_account").on(preference.accountId, preference.name).where(
        sql`${preference.userId} IS NULL AND ${preference.deviceId} IS NULL`,
    ),
    uniqueIndex("preference_user").on(preference.accountId, preference.userId, preference.name)
        .where(sql`${preference.userId} IS NOT NULL AND ${preference.deviceId} IS NULL`),
    uniqueIndex("preference_device").on(
        preference.accountId,
        preference.userId,
        preference.deviceId,
        preference.name,
    ).where(sql`${preference.deviceId} IS NOT NULL`),
    check(
        "preference_device_user",
        sql`${preference.deviceId} IS NULL OR ${preference.userId} IS NOT NULL`,
    ),
]);

/** A persisted preference record. */
export type Preference = Select<typeof preference>;
